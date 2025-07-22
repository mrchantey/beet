#[cfg(any(feature = "server", feature = "client"))]
#[path = "codegen/client_actions.rs"]
mod client_actions;
#[cfg(feature = "server")]
mod codegen;
#[cfg(any(feature = "server", feature = "client"))]
pub mod types;

#[cfg(feature = "config")]
mod config_plugin;
pub mod prelude {
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
	#[cfg(feature = "config")]
	pub use crate::config_plugin::*;
	#[cfg(any(feature = "server", feature = "client"))]
	pub use crate::types::*;
	pub use crate::*;
}
