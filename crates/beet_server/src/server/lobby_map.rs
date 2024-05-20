use super::*;
use axum::extract::ConnectInfo;
use axum::extract::WebSocketUpgrade;
use axum::response::IntoResponse;
use axum_extra::TypedHeader;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;


#[derive(Default, Clone)]
pub struct LobbyMap(pub Arc<RwLock<LobbyMapInner>>);


impl LobbyMap {
	pub async fn handle_socket(
		self,
		ws: WebSocketUpgrade,
		user_agent: Option<TypedHeader<headers::UserAgent>>,
		connect_info: ConnectInfo<SocketAddr>,
	) -> impl IntoResponse {
		let lobby = self.clone();
		ws.on_upgrade(move |socket| {
			lobby.handle_upgrade(Client::new(socket, user_agent, connect_info))
		})
	}


	// async fn handle_ws(
	// 	ws: WebSocketUpgrade,
	// 	// user_agent: Option<TypedHeader<headers::UserAgent>>,
	// 	// connect_info: ConnectInfo<SocketAddr>,
	// 	lobby_map: Arc<RwLock<LobbyMap>>,
	// )  {
	// 	ws.on_upgrade(move |socket: WebSocket| -> () {
	// 		lobby_map.write().await.push_client(Client::new(socket))
	// 	})
	// }

	async fn handle_upgrade(self, client: Client) {
		self.0.write().await.push_client(client).await;
	}
}


#[derive(Default)]
pub struct LobbyMapInner {
	pub lobbies: HashMap<LobbyId, Lobby>,
}

impl LobbyMapInner {
	pub async fn push_client(&mut self, client: Client) {
		let lobby_id = LobbyId::default();

		let lobby = self.lobbies.entry(lobby_id).or_insert_with(Lobby::default);

		let lobby_arc = lobby.clone();
		lobby.write().await.push_client(lobby_arc, client);
	}
}
