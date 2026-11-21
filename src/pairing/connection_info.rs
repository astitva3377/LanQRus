use crate::error::LanQRusError;
use serde::{Deserialize, Serialize};
use std::net::SocketAddrV4;

const FORMAT_VERSION: u8 = 1;
const MAX_PAYLOAD_SIZE: usize = 2048;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionInfo {
	version: u8,
	address: SocketAddrV4,
}

impl ConnectionInfo {
	pub fn new(
		address: SocketAddrV4,
	) -> Result<Self, LanQRusError> {
		let info = Self {
			version: FORMAT_VERSION,
			address,
		};

		info.validate()?;

		Ok(info)
	}

	pub fn address(&self) -> &SocketAddrV4 {
		&self.address
	}

	pub fn to_qr_payload(
		&self,
	) -> Result<String, LanQRusError> {
		self.validate()?;

		let payload =
			serde_json::to_string(self).map_err(|_| {
				LanQRusError::InvalidConnectionInfo
			})?;

		if payload.len() > MAX_PAYLOAD_SIZE {
			return Err(
				LanQRusError::InvalidConnectionInfo,
			);
		}

		Ok(payload)
	}

	pub fn from_qr_payload(
		payload: &str,
	) -> Result<Self, LanQRusError> {
		if payload.is_empty()
			|| payload.len() > MAX_PAYLOAD_SIZE
		{
			return Err(
				LanQRusError::InvalidConnectionInfo,
			);
		}

		let info: Self = serde_json::from_str(payload)
			.map_err(|_| {
				LanQRusError::InvalidConnectionInfo
			})?;

		info.validate()?;

		Ok(info)
	}

	fn validate(&self) -> Result<(), LanQRusError> {
		if self.version != FORMAT_VERSION {
			return Err(
				LanQRusError::InvalidConnectionInfo,
			);
		}

		let ip = self.address.ip();

		if !ip.is_private() || self.address.port() == 0 {
			return Err(
				LanQRusError::InvalidConnectionInfo,
			);
		}

		Ok(())
	}
}
