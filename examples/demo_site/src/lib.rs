#[cfg(any(feature = "server", feature = "client"))]
#[path = "codegen/client_actions.rs"]
mod client_actions;
#[cfg(any(feature = "server", feature = "client"))]
mod codegen;
// templates may rely on codegen like actions so exclude from launch builds
#[cfg(any(feature = "server", feature = "client"))]
mod templates;


#[cfg(feature = "launch")]
mod collections;
pub mod prelude {
	#[cfg(any(feature = "server", feature = "client"))]
	pub use templates::*;

	#[cfg(any(feature = "server", feature = "client"))]
	pub use crate::codegen::routes;
	// pub use crate::Article;
	#[cfg(any(feature = "server", feature = "client"))]
	pub use crate::client_actions::routes as actions;
	#[cfg(feature = "server")]
	pub use crate::codegen::actions::actions_routes;
	#[cfg(feature = "server")]
	pub use crate::codegen::docs::docs_routes;
	#[cfg(feature = "server")]
	pub use crate::codegen::pages::pages_routes;
	#[cfg(feature = "launch")]
	pub use crate::collections::*;
	pub use crate::*;
}



use beet::exports::bevy::ecs as bevy_ecs;
use beet::prelude::*;
use serde::Deserialize;
use std::sync::LazyLock;
use std::sync::Mutex;


/// The metadata at the top of a markdown article,
#[derive(Debug, Default, Clone, Component, Deserialize)]
pub struct Article {
	pub title: String,
	pub created: Option<String>,
}


#[derive(Clone)]
pub struct AppState {
	pub started: std::time::Instant,
	pub num_requests: u32,
}

impl AppState {
	pub fn get() -> AppState { APP_STATE.lock().unwrap().clone() }
	pub fn set(state: AppState) { *APP_STATE.lock().unwrap() = state; }
}
static APP_STATE: LazyLock<Mutex<AppState>> = LazyLock::new(|| {
	Mutex::new(AppState {
		started: std::time::Instant::now(),
		num_requests: 0,
	})
});
