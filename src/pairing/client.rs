use crate::{
	ConnectionInfo, LanQRusError, PendingConnection,
	handshake::client_handshake,
};
use tokio::net::TcpStream;

pub struct LanQRusClient;

impl LanQRusClient {
	pub async fn connect(
		info: &ConnectionInfo,
	) -> Result<PendingConnection<TcpStream>, LanQRusError>
	{
		let stream =
			TcpStream::connect(info.address()).await?;

		client_handshake(stream).await
	}
}
