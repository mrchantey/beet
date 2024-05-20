use crate::prelude::*;
use anyhow::Result;
use futures::future::try_join_all;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;


pub type AxumWsEvent = axum::extract::ws::Message;

pub type LobbyId = usize;
pub type ClientId = usize;

pub type Lobby = Arc<RwLock<LobbyInner>>;

#[derive(Default)]
pub struct LobbyMap {
	pub lobbies: HashMap<LobbyId, Lobby>,
}


impl LobbyMap {
	pub async fn push_client(&mut self, client: Client) {
		let lobby_id = LobbyId::default();

		let lobby = self.lobbies.entry(lobby_id).or_insert_with(Lobby::default);

		let lobby_arc = lobby.clone();
		lobby.write().await.push_client(lobby_arc, client);
	}
}


#[derive(Default)]
pub struct LobbyInner {
	client_id_incr: ClientId,
	clients: HashMap<ClientId, LobbyClient>,
}


impl LobbyInner {
	fn next_id(&mut self) -> ClientId {
		let id = self.client_id_incr;
		self.client_id_incr += 1;
		id
	}

	pub fn push_client(&mut self, self_arc: Lobby, client: Client) {
		let id = self.next_id();
		let lobby_client = LobbyClient::new(self, client, id);
		self.clients.insert(id, lobby_client);
	}

	pub async fn handle_message(
		&mut self,
		client_id: ClientId,
		msg: Vec<u8>,
	) -> Result<()> {
		let futs = self
			.clients
			.iter_mut()
			.filter(|(id, _)| **id != client_id)
			.map(|(_, client)| client.send(msg.clone()));

		try_join_all(futs).await?;
		Ok(())
	}

	pub fn remove_client(&mut self, client_id: ClientId) {
		self.clients.remove(&client_id);
	}
}
