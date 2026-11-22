use lanqrus::{client_handshake, server_handshake};
use tokio::net::{TcpListener, TcpStream};

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

	let server_sas = server_pending.sas().to_owned();
	let client_sas = client_pending.sas().to_owned();

	println!("Server SAS: {server_sas}");
	println!("Client SAS: {client_sas}");

	assert_eq!(
		server_sas, client_sas,
		"SAS values do not match"
	);

	let (client_result, server_result) = tokio::join!(
		client_pending.approve(&server_sas),
		server_pending.wait_for_approval(),
	);

	let client =
		client_result.expect("client approval error");
	let server =
		server_result.expect("server approval error");

	println!("Connection approved");

	// Client to server communication.

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

	// Server to client communication.

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

	let server_sas = server_pending.sas().to_owned();
	let client_sas = client_pending.sas().to_owned();

	assert_eq!(server_sas, client_sas);

	// Client approves; server waits for the decision.
	let (client_result, server_result) = tokio::join!(
		client_pending.approve(&server_sas),
		server_pending.wait_for_approval(),
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
