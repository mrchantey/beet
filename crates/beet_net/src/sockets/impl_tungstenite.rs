use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;
use bytes::Bytes;
use futures::FutureExt;
use futures::SinkExt;
use futures::StreamExt;
use futures::future::BoxFuture;
use std::pin::Pin;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Error as TungError;
use tokio_tungstenite::tungstenite::Message as TungMessage;
use tokio_tungstenite::tungstenite::Utf8Bytes;
use tokio_tungstenite::tungstenite::protocol::CloseFrame as TungCloseFrame;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode as TungCloseCode;

type DynTungSink =
	dyn futures::Sink<TungMessage, Error = TungError> + Send + Unpin;

/// Connect to a WebSocket endpoint using tokio-tungstenite and return a cross-platform `Socket`.
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

	let writer = Box::new(TungWriter {
		sink: std::sync::Arc::new(tokio::sync::Mutex::new(sink_boxed)),
	});

	Ok(Socket::new(incoming, writer))
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
		Message::Binary(b) => TungMessage::Binary(b),
		Message::Ping(b) => TungMessage::Ping(b),
		Message::Pong(b) => TungMessage::Pong(b),
		Message::Close(close) => {
			TungMessage::Close(close.map(|cf| TungCloseFrame {
				code: close_code_from_u16(cf.code),
				reason: Utf8Bytes::from(cf.reason),
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

struct TungWriter {
	sink: std::sync::Arc<tokio::sync::Mutex<Pin<Box<DynTungSink>>>>,
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
						reason: Utf8Bytes::from(cf.reason),
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
