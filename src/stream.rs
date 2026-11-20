use crate::error::LanQRusError;
use snow::TransportState;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io::{
	AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt,
	ReadHalf, WriteHalf,
};
use tokio::sync::{Mutex, Notify};

const TAG_SIZE: usize = 16;

struct CloseOnDrop<'a, S>
where
	S: AsyncRead + AsyncWrite + Unpin,
{
	stream: &'a SecureStream<S>,
	active: bool,
}

impl<'a, S> CloseOnDrop<'a, S>
where
	S: AsyncRead + AsyncWrite + Unpin,
{
	fn new(stream: &'a SecureStream<S>) -> Self {
		Self {
			stream,
			active: true,
		}
	}

	fn disarm(&mut self) {
		self.active = false;
	}
}

impl<S> Drop for CloseOnDrop<'_, S>
where
	S: AsyncRead + AsyncWrite + Unpin,
{
	fn drop(&mut self) {
		if self.active {
			self.stream.close_state();
		}
	}
}

pub struct SecureStream<S> {
	reader: Mutex<ReadHalf<S>>,
	writer: Mutex<WriteHalf<S>>,
	transport: Mutex<TransportState>,
	send_gate: Mutex<()>,
	closed: AtomicBool,
	shutdown: Notify,
}

impl<S> SecureStream<S>
where
	S: AsyncRead + AsyncWrite + Unpin,
{
	pub(crate) fn new(
		stream: S,
		transport: TransportState,
	) -> Self {
		let (reader, writer) = tokio::io::split(stream);

		Self {
			reader: Mutex::new(reader),
			writer: Mutex::new(writer),
			transport: Mutex::new(transport),
			send_gate: Mutex::new(()),
			closed: AtomicBool::new(false),
			shutdown: Notify::new(),
		}
	}

	/// Mark the connection as closed.
	fn close_state(&self) {
		let was_closed =
			self.closed.swap(true, Ordering::SeqCst);

		if !was_closed {
			self.shutdown.notify_waiters();
		}
	}

	/// Mark the connection closed after synchronizing with send operations.
	async fn fail_closed(&self) {
		let _send_guard = self.send_gate.lock().await;
		self.close_state();
	}

	/// Check whether the connection is still usable.
	fn ensure_open(&self) -> Result<(), LanQRusError> {
		if self.closed.load(Ordering::SeqCst) {
			return Err(LanQRusError::ConnectionClosed);
		}

		Ok(())
	}

	/// Close the connection and wake operations waiting for network data.
	pub async fn close(&self) -> Result<(), LanQRusError> {
		// Wait for any in-progress send to finish.
		let _send_guard = self.send_gate.lock().await;

		// No subsequent send can begin after this point.
		self.close_state();

		// Shut down the write half.
		let mut writer = self.writer.lock().await;
		writer.shutdown().await?;

		Ok(())
	}

	/// Encrypt and send one application message.
	pub async fn send(
		&self,
		plaintext: &[u8],
	) -> Result<(), LanQRusError> {
		self.ensure_open()?;

		let max_plaintext = u16::MAX as usize - TAG_SIZE;

		if plaintext.len() > max_plaintext {
			return Err(LanQRusError::FrameTooLarge);
		}

		// Serialize the complete send operation with shutdown.
		let _send_guard = self.send_gate.lock().await;

		self.ensure_open()?;

		let mut ciphertext =
			vec![0u8; plaintext.len() + TAG_SIZE];

		// Encrypt the message and advance the Noise sending nonce.
		let ciphertext_len = {
			let mut transport = self.transport.lock().await;

			match transport
				.write_message(plaintext, &mut ciphertext)
			{
				Ok(length) => length,
				Err(error) => {
					self.close_state();
					return Err(error.into());
				}
			}
		};

		let mut cancellation_guard = CloseOnDrop::new(self);

		let length = match u16::try_from(ciphertext_len) {
			Ok(length) => length,
			Err(_) => {
				self.close_state();
				return Err(LanQRusError::FrameTooLarge);
			}
		};

		// Write the encrypted frame.
		let mut writer = self.writer.lock().await;

		if let Err(error) =
			writer.write_all(&length.to_be_bytes()).await
		{
			self.close_state();
			return Err(error.into());
		}

		if let Err(error) = writer
			.write_all(&ciphertext[..ciphertext_len])
			.await
		{
			self.close_state();
			return Err(error.into());
		}

		if let Err(error) = writer.flush().await {
			self.close_state();
			return Err(error.into());
		}

		cancellation_guard.disarm();

		Ok(())
	}

	/// Receive one encrypted message and return its plaintext.
	pub async fn receive(
		&self,
	) -> Result<Vec<u8>, LanQRusError> {
		self.ensure_open()?;

		// Register for shutdown notification before checking state again.
		let notified = self.shutdown.notified();
		tokio::pin!(notified);
		notified.as_mut().enable();

		self.ensure_open()?;

		let (ciphertext, mut cancellation_guard) = {
			let mut reader = tokio::select! {
				biased;

				_ = &mut notified => {
					return Err(LanQRusError::ConnectionClosed);
				}

				result = self.reader.lock() => result,
			};

			let cancellation_guard = CloseOnDrop::new(self);

			self.ensure_open()?;

			let mut header = [0u8; 2];

			let header_result = tokio::select! {
				biased;

				_ = &mut notified => {
					self.fail_closed().await;
					return Err(LanQRusError::ConnectionClosed);
				}

				result = reader.read_exact(&mut header) => result,
			};

			if let Err(error) = header_result {
				self.fail_closed().await;
				return Err(error.into());
			}

			let length =
				u16::from_be_bytes(header) as usize;

			if length < TAG_SIZE {
				self.fail_closed().await;
				return Err(LanQRusError::InvalidFrame);
			}

			let mut ciphertext = vec![0u8; length];

			let body_result = tokio::select! {
				biased;

				_ = &mut notified => {
					self.fail_closed().await;
					return Err(LanQRusError::ConnectionClosed);
				}

				result = reader.read_exact(&mut ciphertext) => result,
			};

			if let Err(error) = body_result {
				self.fail_closed().await;
				return Err(error.into());
			}

			(ciphertext, cancellation_guard)
		};

		self.ensure_open()?;

		let mut plaintext = vec![0u8; ciphertext.len()];

		let decrypt_result = {
			let mut transport = self.transport.lock().await;
			transport
				.read_message(&ciphertext, &mut plaintext)
		};

		let plaintext_len = match decrypt_result {
			Ok(length) => length,
			Err(error) => {
				self.fail_closed().await;
				return Err(error.into());
			}
		};

		self.ensure_open()?;

		cancellation_guard.disarm();

		plaintext.truncate(plaintext_len);

		Ok(plaintext)
	}
}
