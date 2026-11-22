use lanqrus::{
	ConnectionInfo, LanQRusClient, LanQRusError,
};
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	print!("Paste connection string from server QR: ");
	io::stdout().flush()?;

	let mut qr_payload = String::new();
	io::stdin().read_line(&mut qr_payload)?;

	let connection_info =
		ConnectionInfo::from_qr_payload(qr_payload.trim())?;
	println!(
		"\nConnecting to: {}",
		connection_info.address()
	);

	let pending =
		LanQRusClient::connect(&connection_info).await?;
	println!("Connected to server");

	print!("Enter connect code from server: ");
	io::stdout().flush()?;

	let mut entered_sas = String::new();
	io::stdin().read_line(&mut entered_sas)?;

	let secure =
		pending.approve(entered_sas.trim()).await?;

	println!("Connection approved");
	println!("\nSecure connection established");
	println!("Waiting for encrypted messages...\n");

	loop {
		match secure.receive().await {
            Ok(message) => {
                let message = String::from_utf8_lossy(&message);

                println!("Client: {message}");

				secure.send("ACK".as_bytes()).await?;
            }
            Err(LanQRusError::ConnectionClosed) => {
                println!("Client disconnected.");
                break;
            }
            Err(LanQRusError::Io(error))
                if error.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
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
