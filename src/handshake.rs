use crate::error::LanQRusError;
use crate::protocol::{
	APPROVAL_MESSAGE, ControlMessage, REJECTION_MESSAGE,
	decode_control_message,
};
use crate::stream::SecureStream;
use snow::TransportState;
use std::time::Duration;
use tokio::io::{
	AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt,
};
use tokio::time::timeout;

const NOISE_PATTERN: &str =
	"Noise_XX_25519_ChaChaPoly_BLAKE2s";

async fn write_frame<S>(
	stream: &mut S,
	message: &[u8],
) -> Result<(), LanQRusError>
where
	S: AsyncWrite + Unpin,
{
	let length = u16::try_from(message.len())
		.map_err(|_| LanQRusError::FrameTooLarge)?;

	stream.write_all(&length.to_be_bytes()).await?;
	stream.write_all(message).await?;
	stream.flush().await?;

	Ok(())
}

async fn read_frame<S>(
	stream: &mut S,
) -> Result<Vec<u8>, LanQRusError>
where
	S: AsyncRead + Unpin,
{
	let mut header = [0u8; 2];
	stream.read_exact(&mut header).await?;

	let length = u16::from_be_bytes(header) as usize;

	if length == 0 {
		return Err(LanQRusError::InvalidFrame);
	}

	let mut message = vec![0u8; length];
	stream.read_exact(&mut message).await?;

	Ok(message)
}

fn create_fingerprint(handshake_hash: &[u8]) -> String {
	let mut prefix = [0u8; 8];
	prefix.copy_from_slice(&handshake_hash[..8]);

	let value = u64::from_be_bytes(prefix) % 100_000_000;
	format!("{value:08}")
}

pub struct PendingConnection<S> {
	pub(crate) stream: S,
	pub(crate) transport: TransportState,
	verification_fingerprint: String,
}

impl<S> PendingConnection<S> {
	pub fn verification_fingerprint(&self) -> &str {
		&self.verification_fingerprint
	}
}

impl<S> PendingConnection<S>
where
	S: AsyncRead + AsyncWrite + Unpin,
{
	pub async fn approve(
		self,
		peer_fingerprint: &str,
	) -> Result<SecureStream<S>, LanQRusError> {
		let matches = self.verification_fingerprint
			== peer_fingerprint;

		let connection =
			SecureStream::new(self.stream, self.transport);

		let decision = if matches {
			&APPROVAL_MESSAGE
		} else {
			&REJECTION_MESSAGE
		};

		let exchange = async {
			connection.send(decision).await?;

			let response = connection.receive().await?;

			match decode_control_message(&response) {
				Some(ControlMessage::APPROVE)
					if matches =>
				{
					Ok(())
				}

				Some(ControlMessage::APPROVE) => {
					Err(LanQRusError::VerificationFailed)
				}

				Some(ControlMessage::REJECT) => {
					Err(LanQRusError::PeerRejected)
				}

				None => Err(LanQRusError::InvalidApproval),
			}
		};

		timeout(Duration::from_secs(30), exchange)
			.await
			.map_err(|_| LanQRusError::Timeout)??;

		Ok(connection)
	}

	pub async fn reject(self) -> Result<(), LanQRusError> {
		let connection =
			SecureStream::new(self.stream, self.transport);
		connection.send(&REJECTION_MESSAGE).await?;
		Ok(())
	}
}

pub async fn client_handshake<S>(
	mut stream: S,
) -> Result<PendingConnection<S>, LanQRusError>
where
	S: AsyncRead + AsyncWrite + Unpin,
{
	// Create Noise builder for selected protocol.
	let builder =
		snow::Builder::new(NOISE_PATTERN.parse()?);

	let keypair = builder.generate_keypair()?;

	let mut noise =
		snow::Builder::new(NOISE_PATTERN.parse()?)
			.local_private_key(&keypair.private)?
			.build_initiator()?;
	let mut message = vec![0u8; u16::MAX as usize];
	let mut payload = vec![0u8; u16::MAX as usize];

	// client -> server
	let msg_len = noise.write_message(&[], &mut message)?;
	write_frame(&mut stream, &message[..msg_len]).await?;

	// server -> client
	let response = read_frame(&mut stream).await?;
	noise.read_message(&response, &mut payload)?;

	// client -> server
	let msg_len = noise.write_message(&[], &mut message)?;
	write_frame(&mut stream, &message[..msg_len]).await?;

	if !noise.is_handshake_finished() {
		return Err(LanQRusError::HandshakeIncomplete);
	}

	let fingerprint =
		create_fingerprint(noise.get_handshake_hash());

	let transport = noise.into_transport_mode()?;

	Ok(PendingConnection {
		stream,
		transport,
		verification_fingerprint: fingerprint,
	})
}

pub async fn server_handshake<S>(
	mut stream: S,
) -> Result<PendingConnection<S>, LanQRusError>
where
	S: AsyncRead + AsyncWrite + Unpin,
{
	let builder =
		snow::Builder::new(NOISE_PATTERN.parse()?);

	let keypair = builder.generate_keypair()?;

	let mut noise =
		snow::Builder::new(NOISE_PATTERN.parse()?)
			.local_private_key(&keypair.private)?
			.build_responder()?;

	let mut payload = vec![0u8; u16::MAX as usize];
	let mut message = vec![0u8; u16::MAX as usize];

	// client -> server
	let request = read_frame(&mut stream).await?;
	noise.read_message(&request, &mut payload)?;

	// server -> client
	let msg_len = noise.write_message(&[], &mut message)?;
	write_frame(&mut stream, &message[..msg_len]).await?;

	// client -> server
	let request = read_frame(&mut stream).await?;
	noise.read_message(&request, &mut payload)?;

	if !noise.is_handshake_finished() {
		return Err(LanQRusError::HandshakeIncomplete);
	}

	let fingerprint =
		create_fingerprint(noise.get_handshake_hash());

	let transport = noise.into_transport_mode()?;

	Ok(PendingConnection {
		stream,
		transport,
		verification_fingerprint: fingerprint,
	})
}
