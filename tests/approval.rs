use lanqrus::{
	LanQRusError, client_handshake, server_handshake,
};
use tokio::net::{TcpListener, TcpStream};

#[tokio::test]
async fn fingerprint_mismatch_rejects_connection() {
	let listener = TcpListener::bind("127.0.0.1:0")
		.await
		.expect("bind server");
	let server_addr =
		listener.local_addr().expect("server address");

	let (server_result, client_result) = tokio::join!(
		async {
			let (socket, _) = listener
				.accept()
				.await
				.expect("server accept");
			server_handshake(socket).await
		},
		async {
			let socket = TcpStream::connect(server_addr)
				.await
				.expect("client connect");
			client_handshake(socket).await
		}
	);

	let server_pending =
		server_result.expect("server handshake");
	let client_pending =
		client_result.expect("client handshake");

	let server_fingerprint =
		server_pending.sas().to_owned();
	let client_fingerprint =
		client_pending.sas().to_owned();

	assert_eq!(server_fingerprint, client_fingerprint);

	let wrong_fingerprint =
		if server_fingerprint == "00000000" {
			"00000001".to_owned()
		} else {
			"00000000".to_owned()
		};

	let (client_result, server_result) = tokio::join!(
		client_pending.approve(&wrong_fingerprint),
		server_pending.approve(&client_fingerprint),
	);

	assert!(
		matches!(
			client_result,
			Err(LanQRusError::VerificationFailed)
		),
		"Client should reject its incorrect fingerprint"
	);

	assert!(
		matches!(
			server_result,
			Err(LanQRusError::PeerRejected)
		),
		"Server should receive client fingerprint_mismatch_rejects_connection"
	);
}

#[tokio::test]
async fn tcp_explicit_rejection() {
	let listener = TcpListener::bind("127.0.0.1:0")
		.await
		.expect("bind server");
	let server_addr =
		listener.local_addr().expect("server address");

	let (server_result, client_result) = tokio::join!(
		async {
			let (socket, _) = listener
				.accept()
				.await
				.expect("server accept");
			server_handshake(socket).await
		},
		async {
			let socket = TcpStream::connect(server_addr)
				.await
				.expect("client connect");
			client_handshake(socket).await
		}
	);

	let server_pending =
		server_result.expect("Server handshake");
	let client_pending =
		client_result.expect("client handshake");

	let server_fingerprint =
		server_pending.sas().to_owned();
	let client_fingerprint =
		client_pending.sas().to_owned();

	assert_eq!(server_fingerprint, client_fingerprint);

	// client approve server reject
	let (client_result, server_result) = tokio::join!(
		client_pending.approve(&server_fingerprint),
		server_pending.reject(),
	);

	assert!(
		matches!(
			client_result,
			Err(LanQRusError::PeerRejected)
		),
		"Client should receive the server's rejection"
	);

	assert!(
		server_result.is_ok(),
		"server should successfully send rejection"
	);
}

#[tokio::test]
async fn tcp_peer_disconnects_during_approval() {
	let listener = TcpListener::bind("127.0.0.1:0")
		.await
		.expect("bind server");
	let server_addr =
		listener.local_addr().expect("server address");

	let (server_result, client_result) = tokio::join!(
		async {
			let (socket, _) = listener
				.accept()
				.await
				.expect("server accept");
			server_handshake(socket).await
		},
		async {
			let socket = TcpStream::connect(server_addr)
				.await
				.expect("client connect");
			client_handshake(socket).await
		}
	);

	let server_pending =
		server_result.expect("Server handshake");
	let client_pending =
		client_result.expect("client handshake");

	let fingerprint = client_pending.sas().to_owned();

	drop(server_pending);

	let result = client_pending.approve(&fingerprint).await;

	assert!(
		result.is_err(),
		"Client must not establish after peer lost"
	);
}
