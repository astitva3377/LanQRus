use lanqrus::{
	LanQRusError, client_handshake, server_handshake,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::sleep;

async fn establish_sas() -> String {
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

	let server_sas = server_pending.sas().to_owned();

	let client_sas = client_pending.sas().to_owned();

	assert_eq!(
		server_sas, client_sas,
		"Both endpoints must derive the same SAS"
	);

	server_sas
}

#[tokio::test]
async fn tcp_handshake_and_encrypted_communication() {
	let listener = TcpListener::bind("127.0.0.1:0")
		.await
		.expect("bind server");

	let server_addr =
		listener.local_addr().expect("get server address");

	println!("Server started at {server_addr}");

	let (server_result, client_result) = tokio::join!(
		async {
			let (socket, peer_addr) = listener
				.accept()
				.await
				.expect("accept client");
			println!("Client accepted at {peer_addr}");
			server_handshake(socket).await
		},
		async {
			let socket = TcpStream::connect(server_addr)
				.await
				.expect("connect to server");
			println!("Connected to server");
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

	println!("Server fingerprint: {server_fingerprint}");
	println!("Client fingerprint: {client_fingerprint}");

	assert_eq!(
		server_fingerprint, client_fingerprint,
		"Fingerprints do not match"
	);

	let (client_result, server_result) = tokio::join!(
		client_pending.approve(&server_fingerprint),
		server_pending.approve(&client_fingerprint),
	);

	let client =
		client_result.expect("client approval error");
	let server =
		server_result.expect("Server approval error");

	println!("Both peers approve the connection");

	// Client to Server comm

	let (send_result, receive_result) = tokio::join!(
		client.send(b"Hello from client"),
		server.receive(),
	);

	send_result.expect("client send failed");

	let received =
		receive_result.expect("server receive failed");

	assert_eq!(received, b"Hello from client");
	println!(
		"Server received: {}",
		String::from_utf8_lossy(&received)
	);

	// Server to client comm

	let (send_result, receive_result) = tokio::join!(
		server.send(b"Hello from server"),
		client.receive(),
	);

	send_result.expect("server send error");

	let received =
		receive_result.expect("client receive failed");

	assert_eq!(received, b"Hello from server");
	println!(
		"Client received: {}",
		String::from_utf8_lossy(&received)
	);
}

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

#[tokio::test]
async fn tcp_concurrent_sends_preserve_message_integrity() {
	use std::collections::HashSet;
	use std::sync::Arc;

	const MESSAGE_COUNT: usize = 32;

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

	let sprint = server_pending.sas().to_owned();
	let cprint = client_pending.sas().to_owned();

	assert_eq!(sprint, cprint);

	// Both peers approve the connection.
	let (client_result, server_result) = tokio::join!(
		client_pending.approve(&sprint),
		server_pending.approve(&cprint),
	);

	let client =
		Arc::new(client_result.expect("client approval"));
	let server = server_result.expect("server approval");

	// Launch multiple independent tasks that send concurrently.
	let mut send_tasks = Vec::new();

	for index in 0..MESSAGE_COUNT {
		let client = Arc::clone(&client);
		let message = format!("message-{index}");

		let task = tokio::spawn(async move {
			client.send(message.as_bytes()).await
		});

		send_tasks.push(task);
	}

	// Receive all messages on the server.
	let mut received = HashSet::new();

	for _ in 0..MESSAGE_COUNT {
		let message = server
			.receive()
			.await
			.expect("receive message");

		let message = String::from_utf8(message)
			.expect("message should be valid UTF-8");

		assert!(
			received.insert(message.clone()),
			"duplicate message received: {message}"
		);
	}

	// Confirm every sending task completed successfully.
	for task in send_tasks {
		task.await
			.expect("sender task should not panic")
			.expect("message should be sent successfully");
	}

	// Verify that every expected message arrived.
	let expected: HashSet<String> = (0..MESSAGE_COUNT)
		.map(|index| format!("message-{index}"))
		.collect();

	assert_eq!(received, expected);
}

#[tokio::test]
async fn tcp_stream_closes_after_receive_failure() {
	let listener = TcpListener::bind("127.0.0.1:0")
		.await
		.expect("bind server");

	let server_address =
		listener.local_addr().expect("server address");

	let (server_result, client_result) = tokio::join!(
		async {
			let (socket, _) =
				listener.accept().await.expect("accept");
			server_handshake(socket).await
		},
		async {
			let socket = TcpStream::connect(server_address)
				.await
				.expect("connect");
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

	let (client_result, server_result) = tokio::join!(
		client_pending.approve(&server_fingerprint),
		server_pending.approve(&client_fingerprint),
	);

	let client = client_result.expect("client approval");
	let server = server_result.expect("server approval");

	drop(server);

	let first_result = client.receive().await;

	assert!(
		first_result.is_err(),
		"Receive should fail after disconnect"
	);

	let second_result = client.send(b"Pulse check").await;

	assert!(
		matches!(
			second_result,
			Err(LanQRusError::ConnectionClosed)
		),
		"Should be connection closed"
	);
}

#[tokio::test]
async fn tcp_explicit_close_interrupts_pending_receive() {
	use std::sync::Arc;
	use std::time::Duration;

	let listener = TcpListener::bind("127.0.0.1:0")
		.await
		.expect("bind server");

	let server_address =
		listener.local_addr().expect("server address");

	let (server_result, client_result) = tokio::join!(
		async {
			let (socket, _) =
				listener.accept().await.expect("accept");
			server_handshake(socket).await
		},
		async {
			let socket = TcpStream::connect(server_address)
				.await
				.expect("connect");
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

	let (client_result, server_result) = tokio::join!(
		client_pending.approve(&server_fingerprint),
		server_pending.approve(&client_fingerprint),
	);

	let client =
		Arc::new(client_result.expect("client approval"));
	let server = server_result.expect("server approval");

	// Start a receive operation that should remain pending.
	let receiving_client = Arc::clone(&client);

	let receive_task = tokio::spawn(async move {
		receiving_client.receive().await
	});

	// Give the receive task time to begin waiting for data.
	tokio::time::sleep(Duration::from_millis(50)).await;

	// Explicitly close the client connection.
	client.close().await.expect("close connection");

	// The pending receive should be woken up.
	let receive_result = tokio::time::timeout(
		Duration::from_secs(2),
		receive_task,
	)
	.await
	.expect("receive should not remain blocked")
	.expect("receive task should not panic");

	assert!(
		matches!(
			receive_result,
			Err(LanQRusError::ConnectionClosed)
		),
		"Pending receive should return ConnectionClosed"
	);

	// Future operations should also be rejected.
	let send_result = client.send(b"after close").await;

	assert!(matches!(
		send_result,
		Err(LanQRusError::ConnectionClosed)
	));

	// Avoid an unused-variable warning and explicitly release the peer.
	drop(server);
}

#[tokio::test]
async fn tcp_close_prevents_subsequent_sends() {
	use std::sync::Arc;

	let listener = TcpListener::bind("127.0.0.1:0")
		.await
		.expect("bind server");

	let server_address =
		listener.local_addr().expect("server address");

	let (server_result, client_result) = tokio::join!(
		async {
			let (socket, _) =
				listener.accept().await.expect("accept");
			server_handshake(socket).await
		},
		async {
			let socket = TcpStream::connect(server_address)
				.await
				.expect("connect");
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

	let (client_result, server_result) = tokio::join!(
		client_pending.approve(&server_fingerprint),
		server_pending.approve(&client_fingerprint),
	);

	let client =
		Arc::new(client_result.expect("client approval"));
	let server = server_result.expect("server approval");

	// Close the connection.
	client.close().await.expect("close connection");

	// Sending after close must fail.
	let result =
		client.send(b"this must not be sent").await;

	assert!(
		matches!(
			result,
			Err(LanQRusError::ConnectionClosed)
		),
		"Sending after close must return ConnectionClosed"
	);

	drop(server);
}

#[tokio::test]
async fn tcp_cancelled_receive_closes_connection() {
	// Create a real TCP connection.
	let listener =
		TcpListener::bind("127.0.0.1:0").await.unwrap();

	let server_address = listener.local_addr().unwrap();

	let (server_socket, client_socket) = tokio::join!(
		async { listener.accept().await.unwrap().0 },
		TcpStream::connect(server_address)
	);

	let client_socket = client_socket.unwrap();

	// Perform the Noise handshake.
	let (server_pending, client_pending) = tokio::join!(
		server_handshake(server_socket),
		client_handshake(client_socket)
	);

	let server_pending = server_pending.unwrap();
	let client_pending = client_pending.unwrap();

	let server_fingerprint =
		server_pending.sas().to_owned();

	let client_fingerprint =
		client_pending.sas().to_owned();

	// Both peers approve the connection.
	let (client_result, server_result) = tokio::join!(
		client_pending.approve(&server_fingerprint),
		server_pending.approve(&client_fingerprint)
	);

	let client_secure = Arc::new(client_result.unwrap());
	let _server_secure = server_result.unwrap();

	// Start receiving without sending anything from the peer.
	let client_for_task = Arc::clone(&client_secure);

	let receive_task = tokio::spawn(async move {
		client_for_task.receive().await
	});

	// Give receive() time to acquire the reader and start waiting.
	sleep(Duration::from_millis(100)).await;

	// Cancel the receive operation.
	receive_task.abort();

	let join_error = receive_task.await.unwrap_err();
	assert!(join_error.is_cancelled());

	// The cancellation guard should have closed the connection.
	let send_result =
		client_secure.send(b"should not be sent").await;

	assert!(matches!(
		send_result,
		Err(LanQRusError::ConnectionClosed)
	));
}

#[tokio::test]
async fn tcp_cancelled_send_closes_connection() {
	let listener =
		TcpListener::bind("127.0.0.1:0").await.unwrap();

	let server_address = listener.local_addr().unwrap();

	let (server_socket, client_socket) = tokio::join!(
		async { listener.accept().await.unwrap().0 },
		TcpStream::connect(server_address)
	);

	let server_socket = server_socket;
	let client_socket = client_socket.unwrap();

	// Perform the Noise handshake.
	let (server_pending, client_pending) = tokio::join!(
		server_handshake(server_socket),
		client_handshake(client_socket)
	);

	let server_pending = server_pending.unwrap();
	let client_pending = client_pending.unwrap();

	let server_fingerprint =
		server_pending.sas().to_owned();

	let client_fingerprint =
		client_pending.sas().to_owned();

	// Approve on both sides.
	let (client_result, server_result) = tokio::join!(
		client_pending.approve(&server_fingerprint),
		server_pending.approve(&client_fingerprint)
	);

	let client_secure = Arc::new(client_result.unwrap());
	let _server_secure = server_result.unwrap();

	// The server deliberately does not receive any application data.
	let payload = vec![0xAB; u16::MAX as usize - 16];

	let successful_sends = Arc::new(AtomicUsize::new(0));

	let client_for_task = Arc::clone(&client_secure);
	let sends_for_task = Arc::clone(&successful_sends);

	let send_task = tokio::spawn(async move {
		loop {
			client_for_task.send(&payload).await?;
			sends_for_task.fetch_add(1, Ordering::SeqCst);
		}

		#[allow(unreachable_code)]
		Ok::<(), LanQRusError>(())
	});

	// Wait until the send task stops completing writes.
	let mut previous_count = 0;
	let mut stalled = false;

	for _ in 0..50 {
		tokio::time::sleep(Duration::from_millis(100))
			.await;

		let current_count =
			successful_sends.load(Ordering::SeqCst);

		if current_count == previous_count
			&& current_count > 0
		{
			stalled = true;
			break;
		}

		previous_count = current_count;
	}

	assert!(
		stalled,
		"send task did not become blocked by TCP backpressure"
	);

	// Cancel the blocked send.
	send_task.abort();

	let join_error = send_task.await.unwrap_err();
	assert!(join_error.is_cancelled());

	// Cancellation must have made the connection unusable.
	let result =
		client_secure.send(b"another message").await;

	assert!(matches!(
		result,
		Err(LanQRusError::ConnectionClosed)
	));
}

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
