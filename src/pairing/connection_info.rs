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
	pub(crate) fn new(
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn qr_payload_round_trip() {
		let address: SocketAddrV4 = "192.168.1.20:65432"
			.parse()
			.expect("Socket Address");
		let info = ConnectionInfo::new(address).unwrap();

		let payload = info.to_qr_payload().unwrap();
		let decoded =
			ConnectionInfo::from_qr_payload(&payload)
				.unwrap();

		assert_eq!(decoded.address(), &address);
	}

	#[test]
	fn reject_public_ip() {
		let address: SocketAddrV4 = "8.8.8.8:54321"
			.parse()
			.expect("Socket Address");
		assert!(ConnectionInfo::new(address).is_err());
	}

	#[test]
	fn reject_zero_port() {
		let address: SocketAddrV4 = "192.168.1.20:0"
			.parse()
			.expect("Server Address");
		assert!(ConnectionInfo::new(address).is_err());
	}
}
