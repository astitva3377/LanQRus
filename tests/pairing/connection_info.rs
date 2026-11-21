use lanqrus::ConnectionInfo;
use std::net::SocketAddrV4;

#[test]
fn qr_payload_round_trip() {
	let address: SocketAddrV4 = "192.168.1.20:65432"
		.parse()
		.expect("Socket Address");
	let info = ConnectionInfo::new(address).unwrap();

	let payload = info.to_qr_payload().unwrap();
	let decoded =
		ConnectionInfo::from_qr_payload(&payload).unwrap();

	assert_eq!(decoded.address(), &address);
}

#[test]
fn reject_public_ip() {
	let address: SocketAddrV4 =
		"8.8.8.8:54321".parse().expect("Socket Address");
	assert!(ConnectionInfo::new(address).is_err());
}

#[test]
fn reject_zero_port() {
	let address: SocketAddrV4 =
		"192.168.1.20:0".parse().expect("Server Address");
	assert!(ConnectionInfo::new(address).is_err());
}

#[test]
fn reject_unsupported_version() {
	let payload = r#"{"version":2,"addresses":["192.168.1.20:54321"]}"#;
	assert!(
		ConnectionInfo::from_qr_payload(payload).is_err()
	);
}

#[test]
fn rejects_malformed_payload() {
	assert!(
		ConnectionInfo::from_qr_payload("not-json")
			.is_err()
	);
}
