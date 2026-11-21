use crate::error::LanQRusError;
use crate::protocol::{
	APPROVAL_ACK_MESSAGE, APPROVAL_MESSAGE, ControlMessage,
	REJECTION_ACK_MESSAGE, REJECTION_MESSAGE,
	decode_control_message,
};
use crate::stream::SecureStream;
use hkdf::Hkdf;
use sha2::Sha256;
use snow::TransportState;
use std::time::Duration;
use tokio::io::{
	AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt,
};
use tokio::time::timeout;
use zeroize::Zeroizing;

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

fn create_sas(noise: &mut snow::HandshakeState) -> String {
	let (key1, key2) = noise.dangerously_get_raw_split();

	let key1 = Zeroizing::new(key1);
	let key2 = Zeroizing::new(key2);

	let mut input_key_material = Zeroizing::new([0u8; 64]);

	input_key_material[..32].copy_from_slice(&*key1);
	input_key_material[32..].copy_from_slice(&*key2);

	let hkdf = Hkdf::<Sha256>::new(
		Some(noise.get_handshake_hash()),
		&input_key_material[..],
	);

	let mut sas_material = Zeroizing::new([0u8; 8]);

	hkdf.expand(b"lanqrus SAS v1", &mut sas_material[..])
		.expect("8B is a valid HKDF length");

	let value =
		u64::from_be_bytes(*sas_material) % 100_000_000;
	format!("{value:08}")
}

pub struct PendingConnection<S> {
	pub(crate) stream: S,
	pub(crate) transport: TransportState,
	sas: String,
}

impl<S> PendingConnection<S> {
	pub fn sas(&self) -> &str {
		&self.sas
	}
}

impl<S> PendingConnection<S>
where
	S: AsyncRead + AsyncWrite + Unpin,
{
	pub async fn approve(
		self,
		entered_sas: &str,
	) -> Result<SecureStream<S>, LanQRusError> {
		let matches = self.sas == entered_sas;

		let connection =
			SecureStream::new(self.stream, self.transport);

		let decision = if matches {
			&APPROVAL_MESSAGE
		} else {
			&REJECTION_MESSAGE
		};

		let exchange =
			async {
				connection.send(decision).await?;

				let response = connection.receive().await?;

				match decode_control_message(&response) {
				Some(ControlMessage::ApproveAck) if matches => {
					Ok(())
				}

				Some(ControlMessage::RejectAck) if !matches => {
					Err(LanQRusError::VerificationFailed)
				}

				Some(ControlMessage::RejectAck) => {
					Err(LanQRusError::PeerRejected)
				}

				Some(ControlMessage::ApproveAck) => {
					Err(LanQRusError::InvalidApproval)
				}

				_ => Err(LanQRusError::InvalidApproval)
			}
			};

		timeout(Duration::from_secs(30), exchange)
			.await
			.map_err(|_| LanQRusError::Timeout)??;

		Ok(connection)
	}

	pub async fn wait_for_approval(
		self,
	) -> Result<SecureStream<S>, LanQRusError> {
		let connection =
			SecureStream::new(self.stream, self.transport);

		let exchange = async {
			let decision = connection.receive().await?;

			match decode_control_message(&decision) {
				Some(ControlMessage::Approve) => {
					connection
						.send(&APPROVAL_ACK_MESSAGE)
						.await?;
					Ok(())
				}

				Some(ControlMessage::Reject) => {
					connection
						.send(&REJECTION_ACK_MESSAGE)
						.await?;
					Err(LanQRusError::PeerRejected)
				}

				_ => Err(LanQRusError::InvalidApproval),
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

		let peer_decision = connection.receive().await?;

		match decode_control_message(&peer_decision) {
			Some(ControlMessage::Approve)
			| Some(ControlMessage::Reject)
			| Some(ControlMessage::ApproveAck) => {}

			_ => return Err(LanQRusError::InvalidApproval),
		}

		let ack = connection.receive().await?;

		match decode_control_message(&ack) {
			Some(ControlMessage::RejectAck) => Ok(()),
			_ => Err(LanQRusError::InvalidApproval),
		}
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

	let fingerprint = create_sas(&mut noise);

	let transport = noise.into_transport_mode()?;

	Ok(PendingConnection {
		stream,
		transport,
		sas: fingerprint,
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

	let fingerprint = create_sas(&mut noise);

	let transport = noise.into_transport_mode()?;

	Ok(PendingConnection {
		stream,
		transport,
		sas: fingerprint,
	})
}
