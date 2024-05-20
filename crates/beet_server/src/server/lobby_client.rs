use super::*;
use anyhow::Result;
use axum::extract::ws;
use forky_core::prelude::*;
use futures::SinkExt;
use futures_util::stream::SplitSink;
use futures_util::StreamExt;

pub type AxumWsEvent = axum::extract::ws::Message;


pub struct LobbyClient {
	send: SplitSink<ws::WebSocket, ws::Message>,
	#[allow(dead_code)]
	recv_task: tokio::task::JoinHandle<()>,
}

impl LobbyClient {
	pub fn new(lobby: Lobby, client: Client, client_id: ClientId) -> Self {
		let (send, mut recv) = client.socket.split();
		let recv_task = tokio::spawn(async move {
			while let Some(Ok(msg)) = recv.next().await {
				if let Some(msg) =
					filter_payload(msg).ok_or(|e| log::error!("{e}")).flatten()
				{
					lobby
						.write()
						.await
						.handle_message(client_id, msg)
						.await
						.ok_or(|e| log::error!("{e}"));
				}
			}
			lobby
				.write()
				.await
				.remove_client(client_id)
				.ok_or(|e| log::error!("{e}"));
			log::info!("<<< {}: Disconnected", client_id);
		});

		Self { send, recv_task }
	}

	pub async fn send(&mut self, msg: Vec<u8>) -> Result<()> {
		self.send.send(AxumWsEvent::Binary(msg)).await?;
		Ok(())
	}
}

impl Drop for LobbyClient {
	fn drop(&mut self) { self.recv_task.abort(); }
}

fn filter_payload(msg: AxumWsEvent) -> Result<Option<Vec<u8>>> {
	match msg {
		// AxumWsEvent::Text(txt) => Ok(Some(Message::from_string(&txt)?)),
		AxumWsEvent::Binary(bytes) => Ok(Some(bytes)),
		_ => Ok(None),
	}
}
