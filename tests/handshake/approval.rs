use lanqrus::{
	LanQRusError, client_handshake, server_handshake,
};
use tokio::net::{TcpListener, TcpStream};

async fn establish_pending_connections() -> (
	lanqrus::PendingConnection<TcpStream>,
	lanqrus::PendingConnection<TcpStream>,
) {
	let listener = TcpListener::bind("127.0.0.1:0")
		.await
		.expect("bind server");

	let server_addr = listener.local_addr().unwrap();

	let (server_result, client_result) = tokio::join!(
		async {
			let (socket, _) =
				listener.accept().await.unwrap();
			server_handshake(socket).await
		},
		async {
			let socket = TcpStream::connect(server_addr)
				.await
				.unwrap();
			client_handshake(socket).await
		}
	);

	(
		server_result.expect("server handshake"),
		client_result.expect("client handshake"),
	)
}

#[tokio::test]
async fn fingerprint_mismatch_rejects_connection() {
	let (server_pending, client_pending) =
		establish_pending_connections().await;

	let server_sas = server_pending.sas().to_owned();
	let client_sas = client_pending.sas().to_owned();

	assert_eq!(server_sas, client_sas);

	let wrong_sas = if server_sas == "00000000" {
		"00000001"
	} else {
		"00000000"
	};

	let (client_result, server_result) = tokio::join!(
		client_pending.approve(wrong_sas),
		server_pending.wait_for_approval(),
	);

	assert!(
		matches!(
			client_result,
			Err(LanQRusError::VerificationFailed)
		),
		"Client should reject an incorrect SAS"
	);

	assert!(
		matches!(
			server_result,
			Err(LanQRusError::PeerRejected)
		),
		"Server should receive the client's rejection"
	);
}

#[tokio::test]
async fn tcp_explicit_rejection() {
	let (server_pending, client_pending) =
		establish_pending_connections().await;

	let server_sas = server_pending.sas().to_owned();
	let client_sas = client_pending.sas().to_owned();

	assert_eq!(server_sas, client_sas);

	// Simulate the user entering an incorrect SAS.
	let wrong_sas = if server_sas == "00000000" {
		"00000001"
	} else {
		"00000000"
	};

	let (client_result, server_result) = tokio::join!(
		client_pending.approve(wrong_sas),
		server_pending.wait_for_approval(),
	);

	assert!(
		matches!(
			client_result,
			Err(LanQRusError::VerificationFailed)
		),
		"Client should report SAS mismatch"
	);

	assert!(
		matches!(
			server_result,
			Err(LanQRusError::PeerRejected)
		),
		"Server should acknowledge client rejection"
	);
}

#[tokio::test]
async fn tcp_peer_disconnects_during_approval() {
	let (server_pending, client_pending) =
		establish_pending_connections().await;

	let client_sas = client_pending.sas().to_owned();

	drop(server_pending);

	let result = client_pending.approve(&client_sas).await;

	assert!(
		result.is_err(),
		"Client must not establish after peer disconnects"
	);
}
