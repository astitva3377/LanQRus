use thiserror::Error;

#[derive(Debug, Error)]
pub enum LanQRusError {
	#[error("invalid connection info")]
	InvalidConnectionInfo,

	#[error("I/O error: {0}")]
	Io(#[from] std::io::Error),

	#[error("Noise protocol error: {0}")]
	Noise(#[from] snow::Error),

	#[error("Secure connection is no longer usable")]
	ConnectionClosed,

	#[error("verification code does not match")]
	VerificationFailed,

	#[error("invalid verification code")]
	InvalidCode,

	#[error("message frame is too large")]
	FrameTooLarge,

	#[error("message frame has invalid size")]
	InvalidFrame,

	#[error("peer approval timed out")]
	Timeout,

	#[error("invalid peer approval message")]
	InvalidApproval,

	#[error("peer rejected connection")]
	PeerRejected,

	#[error("Noise handshake did not complete")]
	HandshakeIncomplete,
}
