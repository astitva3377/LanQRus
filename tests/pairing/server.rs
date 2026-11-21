use std::net::{IpAddr, Ipv4Addr};

use lanqrus::LanQRusServer;
use tokio::net::TcpStream;

#[tokio::test]
async fn each_server_gets_an_available_port() {
	let first = LanQRusServer::bind().await.unwrap();
	let second = LanQRusServer::bind().await.unwrap();

	assert_ne!(
		first.connection_info().address().port(),
		second.connection_info().address().port()
	);
}

#[test]
fn does_not_select_public_or_loopback_addresses() {
	let addresses = vec![
		IpAddr::V4(Ipv4Addr::LOCALHOST),
		IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
	];

	let selected =
		addresses.into_iter().find_map(|address| {
			match address {
				IpAddr::V4(ip) if ip.is_private() => {
					Some(ip)
				}
				_ => None,
			}
		});

	assert_eq!(selected, None);
}

#[test]
fn selects_private_ipv4_address() {
	let addresses = vec![
		IpAddr::V4(Ipv4Addr::LOCALHOST),
		IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
		IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20)),
	];

	let selected =
		addresses.into_iter().find_map(|address| {
			match address {
				IpAddr::V4(ip) if ip.is_private() => {
					Some(ip)
				}
				_ => None,
			}
		});

	assert_eq!(
		selected,
		Some(Ipv4Addr::new(192, 168, 1, 20))
	);
}

#[tokio::test]
async fn connection_info_uses_assigned_port_and_lan_address()
 {
	let server = LanQRusServer::bind().await.unwrap();

	let info = server.connection_info();
	let address = info.address();

	assert_eq!(
		address.port(),
		server.connection_info().address().port()
	);
	assert!(address.ip().is_private());
}

#[tokio::test]
async fn binds_to_dynamic_port_on_all_ipv4_interfaces() {
	let server = LanQRusServer::bind().await.unwrap();

	assert_ne!(
		server.connection_info().address().port(),
		0
	);

	let address = server.connection_info().address().ip();

	assert!(address.is_private());
}

#[tokio::test]
async fn accepts_one_client() {
	let server = LanQRusServer::bind().await.unwrap();
	let port = server.connection_info().address().port();

	let accept_task =
		tokio::spawn(async move { server.accept().await });

	let client = TcpStream::connect(("127.0.0.1", port))
		.await
		.unwrap();

	let accepted = accept_task.await.unwrap().unwrap();

	assert!(accepted.peer_addr().is_ok());

	drop(client);
}
