use lanqrus::{LanQRusError, LanQRusServer};
use std::io::{self, BufRead, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let server = LanQRusServer::bind().await?;

	println!(
		"LanQRus server up @ {}",
		server.connection_info().address()
	);

	let qr_payload =
		server.connection_info().to_qr_payload()?;

	println!("\nQR code:\n{}", qr_payload);

	println!("\nWaiting for client...");
	let pending = server.accept().await?;

	println!("Handshake complete...");
	println!("Connect Code: {}", pending.sas());

	let secure = pending.wait_for_approval().await?;
	println!("Connection approved");
	println!("Secure connection established");

	println!("Type a message and press Enter.");
	println!("Type /quit to exit.\n");

	let stdin = io::stdin();
	let mut lines =
		io::BufReader::new(stdin.lock()).lines();

	loop {
		print!("You: ");
		io::stdout().flush()?;

		let line = match lines.next() {
			Some(result) => result?,
			None => break,
		};

		if line == "/quit" {
			break;
		}

		// 6. Send an encrypted message.
		secure.send(line.as_bytes()).await?;

		// 7. Receive the encrypted response.
		match secure.receive().await {
			Ok(_response) => {
				io::stdout().flush()?;
			}
			Err(LanQRusError::ConnectionClosed) => {
				println!("Server disconnected.");
				break;
			}
			Err(error) => {
				eprintln!("Connection error: {error}");
				break;
			}
		}
	}

	secure.close().await?;

	Ok(())
}
