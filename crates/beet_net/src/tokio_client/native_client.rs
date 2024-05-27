use crate::prelude::*;
use anyhow::Result;
use flume::Receiver;
use futures_util::stream::SplitSink;
use futures_util::SinkExt;
use futures_util::StreamExt;
use tokio::net::TcpStream;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream;

type TungMessage = tokio_tungstenite::tungstenite::protocol::Message;


pub struct NativeWsClient {
	pub send:
		SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, TungMessage>,
	recv_task: tokio::task::JoinHandle<Result<()>>,
	recv: Receiver<Vec<u8>>,
}

// impl Default for NativeWsClient {
// 	fn default() -> Self {
// 		Self {
// 			user_id: 7,
// 			channel_id: 16,
// 			url: "ws://127.0.0.1:3000/ws".into(),
// 		}
// 	}
// }


impl NativeWsClient {
	pub async fn new(url: &str) -> Result<Self> {
		let (ws_stream, _response) = connect_async(url).await?;
		let (send, mut recv_stream) = ws_stream.split();

		let (recv_send, recv_recv) = flume::unbounded();

		let recv_task = tokio::spawn(async move {
			while let Some(Ok(msg)) = recv_stream.next().await {
				match msg {
					// #[allow(unused_variables)]
					TungWsEvent::Text(txt) => {
						// #[cfg(feature = "json")]
						// recv_send.recv(Message::vec_from_json(&txt)?).await?;
						// 	#[cfg(not(feature = "json"))]
						// 	anyhow::bail!("received text but feature coora_core/json disabled");
					}
					TungMessage::Binary(bytes) => {
						recv_send.send(bytes)?;
					}
					_ => {}
				}
			}
			Ok(())
		});

		Ok(Self {
			send,
			recv_task,
			recv: recv_recv,
		})
	}
}

impl Drop for NativeWsClient {
	fn drop(&mut self) { self.recv_task.abort(); }
}

impl Transport for NativeWsClient {
	async fn send_bytes(&mut self, bytes: Vec<u8>) -> Result<()> {
		self.send.send(TungMessage::Binary(bytes)).await?;
		Ok(())
	}

	fn recv_bytes(&mut self) -> Result<Vec<Vec<u8>>> {
		let bytes = self.recv.try_recv_all()?;
		Ok(bytes)
	}
}
