// src/lib.rs

//! LanQRus provides authenticated, encrypted, message-oriented
//! communication over asynchronous byte streams.
//!
//! The handshake uses Noise XX and produces an eight-digit short
//! authentication string (SAS).
//!
//! Both endpoints derive the same SAS. The server displays it, and
//! the client must verify the value out of band before calling
//! [`PendingConnection::approve`]. The server waits for the client's
//! decision using [`PendingConnection::wait_for_approval`].
//!
//! # Example
//!
//! ```no_run
//! use lanqrus::{client_handshake, server_handshake};
//! use tokio::net::{TcpListener, TcpStream};
//! # async fn example() -> Result<(), lanqrus::LanQRusError> {
//! let listener = TcpListener::bind("127.0.0.1:9000").await?;
//!
//! let server_task = tokio::spawn(async move {
//!     let (stream, _) = listener.accept().await?;
//!     server_handshake(stream).await
//! });
//!
//! let stream = TcpStream::connect("127.0.0.1:9000").await?;
//! let client_pending = client_handshake(stream).await?;
//! let server_pending = server_task.await.expect("server task")?;
//!
//! let sas = client_pending.sas().to_owned();
//! assert_eq!(sas, server_pending.sas());
//!
//! let (client, server) = tokio::join!(
//!     client_pending.approve(&sas),
//!     server_pending.wait_for_approval(),
//! );
//!
//! let client = client?;
//! let server = server?;
//!
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
pub use pairing::{
	ConnectionInfo, LanQRusClient, LanQRusServer,
};
pub use stream::SecureStream;
