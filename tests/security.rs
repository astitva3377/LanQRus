use lanqrus::{client_handshake, server_handshake};
use tokio::net::{TcpListener, TcpStream};
mod common;
use common::establish_sas;

#[tokio::test]
async fn independent_sessions_produce_different_sas() {
	let (sas_a, sas_b) =
		tokio::join!(establish_sas(), establish_sas());

	assert_ne!(
		sas_a, sas_b,
		"Independent Noise sessions should produce different SAS values"
	);
}

#[tokio::test]
async fn tcp_mitm_relay_produces_different_sas_values() {
	// actual server
	let server_listener = TcpListener::bind("127.0.0.1:0")
		.await
		.expect("bind real server");
	let server_addr = server_listener
		.local_addr()
		.expect("real server address");

	//relay attacker server
	let relay_listener = TcpListener::bind("127.0.0.1:0")
		.await
		.expect("bind relay");
	let relay_addr =
		relay_listener.local_addr().expect("relay address");

	// real server accept the relay connection
	let real_server = async move {
		let (socket, _) = server_listener
			.accept()
			.await
			.expect("real server accept");
		let pending = server_handshake(socket)
			.await
			.expect("real server handshake");
		pending.sas().to_owned()
	};

	// relay creates 2 independent noise sessions
	let relay = async move {
		let (client_socket, _) = relay_listener
			.accept()
			.await
			.expect("relay accept client");
		let server_socket = TcpStream::connect(server_addr)
			.await
			.expect("relay connect to real server");

		// Session A -> relay is responder to client
		// Session B -> relay is initiator to server
		let (client_side, server_side) = tokio::join!(
			server_handshake(client_socket),
			client_handshake(server_socket),
		);

		let client_side =
			client_side.expect("relay-client handshake");
		let server_side =
			server_side.expect("relay-server handshake");

		(
			client_side.sas().to_owned(),
			server_side.sas().to_owned(),
		)
	};

	// client connect relay intending server
	let client = async move {
		let socket = TcpStream::connect(relay_addr)
			.await
			.expect("client connect to relay");
		let pending = client_handshake(socket)
			.await
			.expect("client handshake");
		pending.sas().to_owned()
	};

	// run all concurrently
	let (real_server_sas, relay_sas, client_sas) =
		tokio::join!(real_server, relay, client);
	let (relay_client_sas, relay_server_sas) = relay_sas;

	// The relay and client share the same first session.
	assert_eq!(
		client_sas, relay_client_sas,
		"Client and relay should agree on session A"
	);

	// The relay and real server share the second session.
	assert_eq!(
		real_server_sas, relay_server_sas,
		"Relay and real server should agree on session B"
	);

	// But the two endpoints must see different SAS values.
	assert_ne!(
		client_sas, real_server_sas,
		"MITM relay should produce different SAS values"
	);

	println!("Client SAS:      {client_sas}");
	println!("Relay/client SAS: {relay_client_sas}");
	println!("Relay/server SAS: {relay_server_sas}");
	println!("Server SAS:      {real_server_sas}");
}
