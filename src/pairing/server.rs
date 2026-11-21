use crate::{ConnectionInfo, LanQRusError};
use if_addrs::get_if_addrs;
use std::net::{IpAddr, Ipv4Addr, SocketAddrV4};
use tokio::net::{TcpListener, TcpStream};

pub struct LanQRusServer {
	listener: TcpListener,
	connection_info: ConnectionInfo,
}

impl LanQRusServer {
	pub async fn bind() -> Result<Self, LanQRusError> {
		let ip = discover_lan_ipv4()?;

		let listener =
			TcpListener::bind("0.0.0.0:0").await?;

		let port = listener.local_addr()?.port();

		let address = SocketAddrV4::new(ip, port);
		let connection_info = ConnectionInfo::new(address)?;

		Ok(Self {
			listener,
			connection_info,
		})
	}

	pub fn connection_info(&self) -> &ConnectionInfo {
		&self.connection_info
	}

	pub async fn accept(
		self,
	) -> Result<TcpStream, LanQRusError> {
		let (stream, _) = self.listener.accept().await?;
		Ok(stream)
	}
}

fn discover_lan_ipv4() -> Result<Ipv4Addr, LanQRusError> {
	let interfaces = get_if_addrs()?;

	interfaces
		.into_iter()
		.filter_map(|interface| match interface.ip() {
			IpAddr::V4(ip) if ip.is_private() => Some(ip),
			_ => None,
		})
		.next()
		.ok_or(LanQRusError::NoLANAddress)
}
