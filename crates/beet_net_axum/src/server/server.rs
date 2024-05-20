use crate::prelude::*;
use axum::extract::connect_info::ConnectInfo;
use axum::extract::ws::WebSocketUpgrade;
use axum::response::Html;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use axum_extra::TypedHeader;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;


pub const DEFAULT_ADDRESS: &str = "0.0.0.0:3000";
// address: "127.0.0.1:3000".to_string(),

pub struct Server {
	pub address: String,
	pub lobby_map: Arc<RwLock<LobbyMap>>,
}

impl Default for Server {
	fn default() -> Self {
		Self {
			address: DEFAULT_ADDRESS.to_string(),
			lobby_map: Arc::new(RwLock::new(LobbyMap::default())),
		}
	}
}

impl Server {
	pub fn new(address: String) -> Self {
		Self {
			address,
			..Default::default()
		}
	}
	pub async fn run(self) -> anyhow::Result<()> {
		let assets_dir =
			PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");

		// let pool1 = self.lobbies.clone();
		let lobby_map = self.lobby_map.clone();

		let app = Router::new()
			.fallback_service(
				ServeDir::new(assets_dir.clone())
					.append_index_html_on_directories(true),
			)
			// .ser
			.route("/", get(Self::handle_root))
			// .nest("/api", rest_router(pool1))
			.route(
				"/ws",
				get(move |ws, agent, connect_info| {
					Self::handle_ws(ws, agent, connect_info, lobby_map)
				}),
			)
			.layer(tracing_layer());



		let listener = tokio::net::TcpListener::bind(self.address).await?;
		log::info!(
			"\nlistening on {}\nserving assets from {}",
			listener.local_addr().unwrap(),
			assets_dir.to_str().unwrap()
		);
		// ::tracing::debug!("listening on {}", listener.local_addr().unwrap());
		axum::serve(
			listener,
			app.into_make_service_with_connect_info::<SocketAddr>(),
		)
		.await?;
		Ok(())
	}

	async fn handle_root() -> Html<&'static str> {
		Html("Welcome to the coora server.")
	}

	async fn handle_ws(
		ws: WebSocketUpgrade,
		user_agent: Option<TypedHeader<headers::UserAgent>>,
		connect_info: ConnectInfo<SocketAddr>,
		lobby_map: Arc<RwLock<LobbyMap>>,
	) -> impl IntoResponse {
		ws.on_upgrade(async move |socket| {
			lobby_map.write().await.push_client(Client::new(
				socket,
				user_agent,
				connect_info,
			));
		})
	}
}
