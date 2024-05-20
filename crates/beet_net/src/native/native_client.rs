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
					// TungWsEvent::Text(txt) => {
					// 	#[cfg(feature = "json")]
					// 	recv2.recv(Message::from_string(&txt)?).await?;
					// 	#[cfg(not(feature = "json"))]
					// 	anyhow::bail!("received text but feature coora_core/json disabled");
					// }
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


	pub async fn send(&mut self, messages: &Vec<Message>) -> Result<()> {
		self.send_bytes(Message::into_bytes(messages)?).await
	}

	pub async fn send_bytes(&mut self, bytes: Vec<u8>) -> Result<()> {
		self.send.send(TungMessage::Binary(bytes)).await?;
		Ok(())
	}

	pub fn recv(&mut self) -> Result<Vec<Message>> {
		let bytes = self
			.recv
			.try_recv_all()?
			.into_iter()
			.flatten()
			.collect::<Vec<_>>();

		if bytes.is_empty() {
			return Ok(Vec::new());
		}


		// log::info!("received bytes: {:?}", bytes);
		let messages = Message::from_bytes(&bytes)?;
		log::info!("received messages: {:?}", messages);
		Ok(messages)
	}
}

impl Drop for NativeWsClient {
	fn drop(&mut self) { self.recv_task.abort(); }
}
