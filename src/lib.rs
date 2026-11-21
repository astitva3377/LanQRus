//! LanQRus provides authenticated, encrypted, message-oriented
//! communication over asynchronous byte streams.
//!
//! The handshake uses Noise XX and produces an eight-digit short
//! authentication string (SAS). Applications must compare the SAS out of
//! band before calling [`PendingConnection::approve`].
//!
//! # Example
//!
//! ```no_run
//! use lanqrus::{client_handshake, server_handshake};
//! use tokio::net::{TcpListener, TcpStream};
//! # async fn example() -> Result<(), lanqrus::LanQRusError> {
//! let listener = TcpListener::bind("127.0.0.1:9000").await?;
//! let server = tokio::spawn(async move {
//!     let (stream, _) = listener.accept().await?;
//!     server_handshake(stream).await
//! });
//!
//! let stream = TcpStream::connect("127.0.0.1:9000").await?;
//! let client = client_handshake(stream).await?;
//! let server = server.await.expect("server task")?;
//!
//! let sas = client.sas().to_owned();
//! assert_eq!(sas, server.sas().to_owned());
//! let client = client.approve(&sas).await?;
//! let server = server.approve(&sas).await?;
//! # let _ = (client, server);
//! # Ok(())
//! # }
//! ```

mod error;
mod handshake;
mod pairing;
mod protocol;
mod stream;

pub use error::LanQRusError;
pub use handshake::{
	PendingConnection, client_handshake, server_handshake,
};
pub use pairing::{ConnectionInfo, LanQRusServer};
pub use stream::SecureStream;
