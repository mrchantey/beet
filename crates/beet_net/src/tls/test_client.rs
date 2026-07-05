//! Raw test clients trusting the cached dev certificate, shared by the
//! mini-server and socket-server tls tests.
use crate::prelude::*;
use async_io::Async;
use beet_core::prelude::*;
use futures_rustls::TlsConnector;
use futures_rustls::rustls;
use std::net::SocketAddr;
use std::net::TcpStream;
use std::sync::Arc;

/// TLS-connect to a local listener, trusting exactly the cached dev cert.
pub(crate) async fn connect(
	addr: SocketAddr,
) -> Result<futures_rustls::client::TlsStream<Async<TcpStream>>> {
	let config = DevCert::client_config()?;
	let connector = TlsConnector::from(Arc::new(config));
	let tcp = Async::<TcpStream>::connect(addr).await?;
	let server_name =
		rustls::pki_types::ServerName::try_from("127.0.0.1").unwrap();
	connector.connect(server_name, tcp).await?.xok()
}

/// Send a raw http/1.1 `GET` over `stream` and read the response to EOF.
/// Tolerates a missing TLS `close_notify` once bytes have landed, since the
/// servers close the TCP stream after `connection: close`.
pub(crate) async fn raw_get<S>(mut stream: S, path: &str) -> Result<String>
where
	S: futures_lite::AsyncRead + futures_lite::AsyncWrite + Unpin,
{
	use futures_lite::AsyncReadExt;
	use futures_lite::AsyncWriteExt;
	stream
		.write_all(
			format!(
				"GET {path} HTTP/1.1\r\nhost: 127.0.0.1\r\nconnection: close\r\n\r\n"
			)
			.as_bytes(),
		)
		.await?;
	stream.flush().await?;
	let mut response = Vec::new();
	let mut buf = [0u8; 4096];
	loop {
		match stream.read(&mut buf).await {
			Ok(0) => break,
			Ok(bytes_read) => response.extend_from_slice(&buf[..bytes_read]),
			Err(_) if !response.is_empty() => break,
			Err(err) => return Err(err.into()),
		}
	}
	String::from_utf8(response)?.xok()
}
