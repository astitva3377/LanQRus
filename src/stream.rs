use crate::error::LanQRusError;
use snow::TransportState;
use tokio::io::{
	AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt,
	ReadHalf, WriteHalf,
};
use tokio::sync::Mutex;

const TAG_SIZE: usize = 16;

pub struct SecureStream<S> {
	reader: Mutex<ReadHalf<S>>,
	writer: Mutex<WriteHalf<S>>,
	transport: Mutex<TransportState>,
	send_gate: Mutex<()>,
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
		}
	}

	pub async fn send(
		&self,
		plaintext: &[u8],
	) -> Result<(), LanQRusError> {
		let max_plaintext = u16::MAX as usize - TAG_SIZE;

		if plaintext.len() > max_plaintext {
			return Err(LanQRusError::FrameTooLarge);
		}

		let _send_guard = self.send_gate.lock().await;

		let mut ciphertext =
			vec![0u8; plaintext.len() + TAG_SIZE];

		let ciphertext_len = {
			let mut transport = self.transport.lock().await;

			transport
				.write_message(plaintext, &mut ciphertext)?
		};

		let length = u16::try_from(ciphertext_len)
			.map_err(|_| LanQRusError::FrameTooLarge)?;

		let mut writer = self.writer.lock().await;

		writer.write_all(&length.to_be_bytes()).await?;
		writer
			.write_all(&ciphertext[..ciphertext_len])
			.await?;
		writer.flush().await?;

		Ok(())
	}

	pub async fn receive(
		&self,
	) -> Result<Vec<u8>, LanQRusError> {
		let ciphertext = {
			let mut reader = self.reader.lock().await;

			let header = [0u8; 2];

			let length =
				u16::from_be_bytes(header) as usize;

			if length < TAG_SIZE {
				return Err(LanQRusError::InvalidFrame);
			}

			let mut ciphertext = vec![0u8; length];
			reader.read_exact(&mut ciphertext).await?;

			ciphertext
		};

		let mut plaintext = vec![0u8; ciphertext.len()];

		let plaintext_len = {
			let mut transport = self.transport.lock().await;

			transport
				.read_message(&ciphertext, &mut plaintext)?
		};

		plaintext.truncate(plaintext_len);

		Ok(plaintext)
	}
}
