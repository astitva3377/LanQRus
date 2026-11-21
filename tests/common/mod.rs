use lanqrus::{client_handshake, server_handshake};
use tokio::net::{TcpListener, TcpStream};

pub async fn establish_sas() -> String {
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
