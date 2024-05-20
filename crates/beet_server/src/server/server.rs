use super::*;
use axum::response::Html;
use axum::routing::get;
use axum::Router;
use std::net::SocketAddr;
use std::path::PathBuf;
use tower_http::services::ServeDir;


pub const DEFAULT_ADDRESS: &str = "0.0.0.0:3000";
// address: "127.0.0.1:3000".to_string(),

pub struct Server {
	pub address: String,
}

impl Default for Server {
	fn default() -> Self {
		Self {
			address: DEFAULT_ADDRESS.to_string(),
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
		init_tracing();
		::tracing::debug!("listenin");

		let assets_dir =
			PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");

		let lobby_map = LobbyMap::default();

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
				get(move |ws, user_agent, connect_info| {
					lobby_map.handle_socket(ws, user_agent, connect_info)
				}),
			)
			.layer(tracing_layer());

		let listener = tokio::net::TcpListener::bind(self.address).await?;
		println!("listening on {}", listener.local_addr().unwrap());
		// tracing::debug!("listening on {}", listener.local_addr().unwrap());

		axum::serve(
			listener,
			app.into_make_service_with_connect_info::<SocketAddr>(),
		)
		.await?;
		Ok(())
	}

	async fn handle_root() -> Html<&'static str> {
		Html("Welcome to the beet server.")
	}
}
