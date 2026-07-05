//! Classify an accepted TCP connection by its first bytes, without losing them.
//!
//! A listener serving TLS ([`Tls`](crate::prelude::Tls)) still receives
//! plaintext peers: a browser typing `http://`, the loopback reload watcher, a
//! native `ws://` client. [`Protocol::sniff`] reads just enough to tell a TLS
//! `ClientHello` (first byte `0x16`) from a plaintext http head, and returns a
//! [`ReplayStream`] that replays the consumed bytes so the real handler (a
//! rustls acceptor, the http parser, a websocket handshake) sees the full
//! stream untouched.
use crate::prelude::*;
use beet_core::prelude::*;
use futures_lite::AsyncRead;
use futures_lite::AsyncReadExt;
use futures_lite::AsyncWrite;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

/// Stop sniffing for a plaintext http head after this many bytes; a peer that
/// sent this much without a blank line is handed on as-is for the downstream
/// parser to reject.
pub const SNIFF_CAP: usize = 16 * 1024;

/// The classified first bytes of a connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
	/// First byte `0x16`: a TLS `ClientHello` record.
	Tls,
	/// Anything else: treated as a plaintext http head (websocket handshakes
	/// included), buffered up to the header end or [`SNIFF_CAP`].
	PlainHttp,
	/// The peer closed before sending anything.
	Empty,
}

impl Protocol {
	/// Read the first bytes off `stream` and classify them, returning the
	/// stream wrapped in a [`ReplayStream`] so nothing is lost.
	///
	/// For [`Protocol::Tls`] only the first chunk is consumed (a `ClientHello`
	/// has no http head to wait for); for [`Protocol::PlainHttp`] the read
	/// continues until the blank line ending the head, EOF, or [`SNIFF_CAP`].
	pub async fn sniff<S: AsyncRead + Unpin>(
		mut stream: S,
	) -> Result<(Self, ReplayStream<S>)> {
		let mut consumed = Vec::new();
		let mut buf = [0u8; 2048];
		let protocol = loop {
			let bytes_read = stream.read(&mut buf).await?;
			if bytes_read == 0 {
				break match consumed.is_empty() {
					true => Self::Empty,
					// partial head then EOF: hand on, the parser rejects it
					false => Self::PlainHttp,
				};
			}
			consumed.extend_from_slice(&buf[..bytes_read]);
			if consumed[0] == 0x16 {
				break Self::Tls;
			}
			if http_ext::find_header_end(&consumed).is_some()
				|| consumed.len() >= SNIFF_CAP
			{
				break Self::PlainHttp;
			}
		};
		Ok((protocol, ReplayStream::new(consumed, stream)))
	}
}

/// A stream that replays already-consumed `prefix` bytes before reading from
/// the inner stream; writes pass straight through. This is how a sniffed
/// connection reaches its real handler with no bytes missing.
pub struct ReplayStream<S> {
	prefix: Vec<u8>,
	offset: usize,
	inner: S,
}

impl<S> ReplayStream<S> {
	/// Wrap `inner`, replaying `prefix` before its reads.
	pub fn new(prefix: Vec<u8>, inner: S) -> Self {
		Self {
			prefix,
			offset: 0,
			inner,
		}
	}

	/// The sniffed bytes this stream will replay, ie the http head (and
	/// possibly some body) for a [`Protocol::PlainHttp`] connection.
	pub fn prefix(&self) -> &[u8] { &self.prefix }
}

impl<S: AsyncRead + Unpin> AsyncRead for ReplayStream<S> {
	fn poll_read(
		mut self: Pin<&mut Self>,
		cx: &mut Context<'_>,
		buf: &mut [u8],
	) -> Poll<std::io::Result<usize>> {
		let this = &mut *self;
		if this.offset < this.prefix.len() {
			let remaining = &this.prefix[this.offset..];
			let len = remaining.len().min(buf.len());
			buf[..len].copy_from_slice(&remaining[..len]);
			this.offset += len;
			return Poll::Ready(Ok(len));
		}
		Pin::new(&mut this.inner).poll_read(cx, buf)
	}
}

impl<S: AsyncWrite + Unpin> AsyncWrite for ReplayStream<S> {
	fn poll_write(
		mut self: Pin<&mut Self>,
		cx: &mut Context<'_>,
		buf: &[u8],
	) -> Poll<std::io::Result<usize>> {
		Pin::new(&mut self.inner).poll_write(cx, buf)
	}

	fn poll_flush(
		mut self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<std::io::Result<()>> {
		Pin::new(&mut self.inner).poll_flush(cx)
	}

	fn poll_close(
		mut self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<std::io::Result<()>> {
		Pin::new(&mut self.inner).poll_close(cx)
	}
}

/// Whether a raw http head is a websocket handshake (an `upgrade: websocket`
/// header), so a socket listener can serve a plain page to a browser `GET`
/// instead of failing the handshake.
pub fn head_is_websocket_upgrade(head: &[u8]) -> bool {
	header_value(head, "upgrade")
		.map(|value| value.to_ascii_lowercase().contains("websocket"))
		.unwrap_or(false)
}

/// The `307` redirecting a plaintext request to the same authority over
/// https, serialized and ready to write. `None` when the head has no `host`
/// header to redirect to (the caller answers 400 instead). Temporary (not
/// `308`/`301`) so a browser does not cache the upgrade past a dev server
/// that later runs without [`Tls`](crate::prelude::Tls).
pub fn https_redirect_response(head: &[u8]) -> Option<Vec<u8>> {
	let host = header_value(head, "host")?;
	let path = request_path(head).unwrap_or("/");
	format!(
		"HTTP/1.1 307 Temporary Redirect\r\n\
		location: https://{host}{path}\r\n\
		content-length: 0\r\n\
		connection: close\r\n\r\n"
	)
	.into_bytes()
	.xmap(Some)
}

/// A plain 400 for a plaintext request that cannot be redirected (no `host`
/// header), naming the fix.
pub fn tls_required_response() -> Vec<u8> {
	let body = "this server is serving TLS: connect via https / wss\n";
	format!(
		"HTTP/1.1 400 Bad Request\r\n\
		content-type: text/plain; charset=utf-8\r\n\
		content-length: {}\r\n\
		connection: close\r\n\r\n{body}",
		body.len()
	)
	.into_bytes()
}

/// The page a socket listener serves to a plain browser `GET` (no websocket
/// upgrade). Over TLS this doubles as the cert-acceptance step: browsers show
/// no acceptance UI for a failed `wss://` handshake, so visiting the socket
/// port over https once is how a per-origin exception lands (Firefox and iOS
/// scope exceptions per host:port).
pub fn socket_landing_response(tls: bool) -> Vec<u8> {
	let detail = match tls {
		true => {
			"<p>Certificate accepted for this origin. Return to the app tab; \
			its socket reconnects automatically.</p>\
			<p>This endpoint speaks WebSocket, connect via <code>wss://</code>.</p>"
		}
		false => {
			"<p>This endpoint speaks WebSocket, connect via <code>ws://</code>.</p>"
		}
	};
	let body = format!(
		"<!doctype html><html><head><meta charset=\"utf-8\">\
		<title>beet socket server</title></head>\
		<body style=\"font-family:system-ui;margin:2rem\">\
		<h1>beet socket server</h1>{detail}</body></html>"
	);
	format!(
		"HTTP/1.1 200 OK\r\n\
		content-type: text/html; charset=utf-8\r\n\
		content-length: {}\r\n\
		connection: close\r\n\r\n{body}",
		body.len()
	)
	.into_bytes()
}

/// Write a prebuilt raw response (a redirect, a landing page, a 400) and
/// flush; the caller drops the stream to close.
pub async fn write_and_close<S>(mut stream: S, response: Vec<u8>) -> Result
where
	S: AsyncWrite + Unpin,
{
	use futures_lite::AsyncWriteExt;
	stream.write_all(&response).await?;
	stream.flush().await?;
	Ok(())
}

/// The path from the head's request line, ie `/foo` in `GET /foo HTTP/1.1`.
fn request_path(head: &[u8]) -> Option<&str> {
	let line_end = head
		.windows(2)
		.position(|pair| pair == b"\r\n")
		.unwrap_or(head.len());
	str::from_utf8(&head[..line_end])
		.ok()?
		.split_whitespace()
		.nth(1)
}

/// The trimmed value of a header in a raw http head, case-insensitive on the
/// name.
fn header_value<'a>(head: &'a [u8], name: &str) -> Option<&'a str> {
	str::from_utf8(head)
		.ok()?
		.lines()
		.skip(1)
		.take_while(|line| !line.trim().is_empty())
		.find_map(|line| {
			let (key, value) = line.split_once(':')?;
			key.trim().eq_ignore_ascii_case(name).then(|| value.trim())
		})
}

#[cfg(test)]
mod tests {
	use super::*;
	use futures_lite::AsyncReadExt;
	use futures_lite::io::Cursor;

	const WS_HEAD: &[u8] = b"GET /chat HTTP/1.1\r\nHost: example.com:8338\r\nUpgrade: WebSocket\r\nConnection: Upgrade\r\n\r\n";
	const GET_HEAD: &[u8] =
		b"GET /debug HTTP/1.1\r\nhost: 192.168.1.7:8337\r\n\r\n";

	#[beet_core::test]
	async fn sniffs_tls() {
		let hello = [0x16, 0x03, 0x01, 0x00, 0x05, 0x01];
		let (protocol, replay) =
			Protocol::sniff(Cursor::new(hello.to_vec())).await.unwrap();
		protocol.xpect_eq(Protocol::Tls);
		// the ClientHello bytes replay untouched
		let mut replayed = Vec::new();
		let mut replay = replay;
		replay.read_to_end(&mut replayed).await.unwrap();
		replayed.xpect_eq(hello.to_vec());
	}

	#[beet_core::test]
	async fn sniffs_plaintext_and_replays() {
		let (protocol, mut replay) =
			Protocol::sniff(Cursor::new(GET_HEAD.to_vec()))
				.await
				.unwrap();
		protocol.xpect_eq(Protocol::PlainHttp);
		let mut replayed = Vec::new();
		replay.read_to_end(&mut replayed).await.unwrap();
		replayed.xpect_eq(GET_HEAD.to_vec());
	}

	#[beet_core::test]
	async fn sniffs_empty() {
		let (protocol, _replay) =
			Protocol::sniff(Cursor::new(Vec::new())).await.unwrap();
		protocol.xpect_eq(Protocol::Empty);
	}

	#[beet_core::test]
	async fn detects_websocket_upgrade() {
		head_is_websocket_upgrade(WS_HEAD).xpect_true();
		head_is_websocket_upgrade(GET_HEAD).xpect_false();
	}

	#[beet_core::test]
	async fn builds_redirect() {
		let redirect = https_redirect_response(GET_HEAD).unwrap();
		let text = String::from_utf8(redirect).unwrap();
		text.xpect_contains("307")
			.xpect_contains("location: https://192.168.1.7:8337/debug");
		https_redirect_response(b"GET / HTTP/1.0\r\n\r\n").xpect_none();
	}
}
