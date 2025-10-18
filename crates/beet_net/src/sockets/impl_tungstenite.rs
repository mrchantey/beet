use crate::prelude::sockets::Message;
use crate::prelude::sockets::*;
use async_lock::Mutex;
use async_tungstenite::tokio::accept_async;
use async_tungstenite::tokio::connect_async;
use async_tungstenite::tungstenite::Error as TungError;
use async_tungstenite::tungstenite::Message as TungMessage;
use async_tungstenite::tungstenite::protocol::CloseFrame as TungCloseFrame;
use async_tungstenite::tungstenite::protocol::frame::coding::CloseCode as TungCloseCode;
use beet_core::async_ext::NativeSendBoxedFuture;
use beet_core::prelude::*;
use bytes::Bytes;
use futures::FutureExt;
use futures::SinkExt;
use futures::StreamExt;
use futures::future::BoxFuture;
use std::borrow::Cow;
use std::pin::Pin;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::sync::mpsc;

type DynTungSink =
	dyn futures::Sink<TungMessage, Error = TungError> + Send + Unpin;

/// Connect to a WebSocket endpoint using async-tungstenite and return a cross-platform `Socket`.
///
/// This function:
/// - Establishes a client connection to `url`
/// - Splits the Tungstenite stream/sink
/// - Adapts the inbound `tungstenite::Message` stream into our cross-platform `Message`
/// - Wraps the sink in a writer that implements the `SocketWriter` trait
pub async fn connect_tungstenite(url: impl AsRef<str>) -> Result<Socket> {
	let (ws_stream, _resp) = connect_async(url.as_ref())
		.await
		.map_err(|e| bevyhow!("WebSocket connect failed: {}", e))?;
	let (sink, stream) = ws_stream.split();

	// Map incoming tungstenite messages to our cross-platform Message
	let incoming = stream.map(|res| match res {
		Ok(msg) => Ok(from_tung_msg(msg)),
		Err(err) => Err(bevyhow!("WebSocket receive error: {}", err)),
	});

	// Box the sink to make the writer object-safe
	let sink_boxed: Pin<Box<DynTungSink>> = Box::pin(sink);

	let writer = TungWriter {
		sink: Arc::new(Mutex::new(sink_boxed)),
	};

	Ok(Socket::new(incoming, writer))
}

/// Bind a WebSocket server to the given address using tokio TcpListener and return a cross-platform `SocketServer`.
///
/// This function:
/// - Binds a TCP listener to `addr`
/// - Returns a `SocketServer` that can accept incoming WebSocket connections
pub async fn bind_tungstenite(addr: impl AsRef<str>) -> Result<SocketServer> {
	let listener = TcpListener::bind(addr.as_ref())
		.await
		.map_err(|e| bevyhow!("Failed to bind server: {}", e))?;

	let (socket_tx, socket_rx) = mpsc::unbounded_channel();

	let acceptor = TungAcceptor {
		listener,
		socket_rx,
		socket_tx,
	};
	Ok(SocketServer::new(acceptor))
}

struct TungAcceptor {
	listener: TcpListener,
	socket_rx: mpsc::UnboundedReceiver<Result<Socket>>,
	socket_tx: mpsc::UnboundedSender<Result<Socket>>,
}

impl SocketAcceptor for TungAcceptor {
	fn accept(&mut self) -> NativeSendBoxedFuture<'_, Result<Socket>> {
		Box::pin(async move {
			loop {
				// Try to receive a completed socket first
				if let Ok(socket) = self.socket_rx.try_recv() {
					return socket;
				}

				// Otherwise accept a new connection and spawn it
				let (stream, _addr) =
					self.listener.accept().await.map_err(|e| {
						bevyhow!("Failed to accept connection: {}", e)
					})?;

				let tx = self.socket_tx.clone();
				async_ext::spawn(async move {
					let result = accept_connection(stream).await;
					let _ = tx.send(result);
				});
			}
		})
	}

	fn local_addr(&self) -> Result<std::net::SocketAddr> {
		self.listener
			.local_addr()
			.map_err(|e| bevyhow!("Failed to get local address: {}", e))
	}
}

impl futures::Stream for TungAcceptor {
	type Item = Result<Socket>;

	fn poll_next(
		mut self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Option<Self::Item>> {
		// First check if we have any completed sockets
		match self.socket_rx.poll_recv(cx) {
			std::task::Poll::Ready(Some(socket)) => {
				return std::task::Poll::Ready(Some(socket));
			}
			std::task::Poll::Ready(None) => {
				// Channel closed, no more sockets
				return std::task::Poll::Ready(None);
			}
			std::task::Poll::Pending => {
				// No completed sockets yet, try to accept new connections
			}
		}

		// Try to accept a new connection and spawn its handshake
		match self.listener.poll_accept(cx) {
			std::task::Poll::Ready(Ok((stream, _addr))) => {
				let tx = self.socket_tx.clone();

				async_ext::spawn(async move {
					let result = accept_connection(stream).await;
					let _ = tx.send(result);
				});

				// After spawning, wake ourselves to check for completed sockets
				cx.waker().wake_by_ref();
				std::task::Poll::Pending
			}
			std::task::Poll::Ready(Err(e)) => std::task::Poll::Ready(Some(
				Err(bevyhow!("Accept error: {}", e)),
			)),
			std::task::Poll::Pending => std::task::Poll::Pending,
		}
	}
}

async fn accept_connection(stream: TcpStream) -> Result<Socket> {
	let ws_stream = accept_async(stream)
		.await
		.map_err(|e| bevyhow!("WebSocket handshake failed: {}", e))?;

	let (sink, stream) = ws_stream.split();

	// Map incoming tungstenite messages to our cross-platform Message
	let reader = stream.map(|res| match res {
		Ok(msg) => Ok(from_tung_msg(msg)),
		Err(err) => Err(bevyhow!("WebSocket receive error: {}", err)),
	});

	// Box the sink to make the writer object-safe
	let sink_boxed: Pin<Box<DynTungSink>> = Box::pin(sink);

	let writer = TungWriter {
		sink: Arc::new(Mutex::new(sink_boxed)),
	};

	Ok(Socket::new(reader, writer))
}

struct TungWriter {
	sink: Arc<Mutex<Pin<Box<DynTungSink>>>>,
}

impl SocketWriter for TungWriter {
	fn send_boxed(&mut self, msg: Message) -> BoxFuture<'static, Result<()>> {
		let tmsg = to_tung_msg(msg);
		let sink = self.sink.clone();
		async move {
			let mut guard = sink.lock().await;
			guard
				.send(tmsg)
				.await
				.map_err(|e| bevyhow!("WebSocket send failed: {}", e))?;
			Ok(())
		}
		.boxed()
	}
	fn close_boxed(
		&mut self,
		close: Option<CloseFrame>,
	) -> BoxFuture<'static, Result<()>> {
		let sink = self.sink.clone();
		async move {
			let mut guard = sink.lock().await;
			match close {
				Some(cf) => {
					let frame = TungCloseFrame {
						code: close_code_from_u16(cf.code),
						reason: Cow::Owned(cf.reason),
					};
					guard.send(TungMessage::Close(Some(frame))).await.map_err(
						|e| bevyhow!("WebSocket close send failed: {}", e),
					)?;
					// ensure the sink closes gracefully after sending close
					guard.close().await.map_err(|e| {
						bevyhow!("WebSocket close failed: {}", e)
					})?;
				}
				None => {
					guard.close().await.map_err(|e| {
						bevyhow!("WebSocket close failed: {}", e)
					})?;
				}
			}
			Ok(())
		}
		.boxed()
	}
}

fn from_tung_msg(msg: TungMessage) -> Message {
	match msg {
		TungMessage::Text(s) => Message::Text(s.to_string()),
		TungMessage::Binary(v) => Message::Binary(Bytes::from(v)),
		TungMessage::Ping(v) => Message::Ping(Bytes::from(v)),
		TungMessage::Pong(v) => Message::Pong(Bytes::from(v)),
		TungMessage::Close(opt) => {
			let cf = opt.map(|f| CloseFrame {
				code: close_code_to_u16(f.code),
				reason: f.reason.to_string(),
			});
			Message::Close(cf)
		}
		// Treat other message variants as no-op or "empty" binary
		// (e.g., Frame or __NonExhaustive in older tungstenite versions)
		_ => Message::Binary(Bytes::new()),
	}
}

fn to_tung_msg(msg: Message) -> TungMessage {
	match msg {
		Message::Text(s) => TungMessage::Text(s.into()),
		Message::Binary(b) => TungMessage::Binary(b.to_vec()),
		Message::Ping(b) => TungMessage::Ping(b.to_vec()),
		Message::Pong(b) => TungMessage::Pong(b.to_vec()),
		Message::Close(close) => {
			TungMessage::Close(close.map(|cf| TungCloseFrame {
				code: close_code_from_u16(cf.code),
				reason: Cow::Owned(cf.reason),
			}))
		}
	}
}

// inlined close frame construction
fn close_code_to_u16(code: TungCloseCode) -> u16 {
	#[allow(unreachable_patterns)]
	match code {
		TungCloseCode::Normal => 1000,
		TungCloseCode::Away => 1001,
		TungCloseCode::Protocol => 1002,
		TungCloseCode::Unsupported => 1003,
		TungCloseCode::Status => 1005,
		TungCloseCode::Abnormal => 1006,
		TungCloseCode::Invalid => 1007,
		TungCloseCode::Policy => 1008,
		TungCloseCode::Size => 1009,
		TungCloseCode::Extension => 1010,
		TungCloseCode::Error => 1011,
		TungCloseCode::Restart => 1012,
		TungCloseCode::Again => 1013,
		// Some tungstenite versions include TLS close code
		TungCloseCode::Tls => 1015,
		_ => 1000,
	}
}

fn close_code_from_u16(code: u16) -> TungCloseCode { TungCloseCode::from(code) }

#[cfg(test)]
mod tests {
	use super::*;
	use sweet::prelude::*;

	#[sweet::test]
	fn maps_messages_roundtrip() {
		{
			let text = Message::text("hello");
			let bin = Message::binary(vec![1u8, 2, 3]);
			let ping = Message::ping(Bytes::from_static(b"p"));
			let pong = Message::pong(Bytes::from_static(b"q"));
			let close = Message::close(1000, "bye");

			let t_text = super::to_tung_msg(text.clone());
			let t_bin = super::to_tung_msg(bin.clone());
			let t_ping = super::to_tung_msg(ping.clone());
			let t_pong = super::to_tung_msg(pong.clone());
			let t_close = super::to_tung_msg(close.clone());

			super::from_tung_msg(t_text).xpect_eq(text);
			super::from_tung_msg(t_bin).xpect_eq(bin);
			super::from_tung_msg(t_ping).xpect_eq(ping);
			super::from_tung_msg(t_pong).xpect_eq(pong);
			// Close roundtrip may lose exact code mapping on older tungstenite, but should remain Close(..)
			matches!(super::from_tung_msg(t_close), Message::Close(_))
				.xpect_true();
		}
	}
}
