#[cfg(feature = "client")]
#[path = "codegen/client_actions.rs"]
mod client_actions;
#[cfg(feature = "server")]
mod codegen;
mod types;

#[cfg(feature = "config")]
mod config_plugin;
pub mod prelude {
	#[cfg(feature = "client")]
	pub use crate::client_actions::routes as actions;
	#[cfg(feature = "server")]
	pub use crate::codegen::actions::actions_routes;
	#[cfg(feature = "server")]
	pub use crate::codegen::docs::docs_routes;
	#[cfg(feature = "server")]
	pub use crate::codegen::pages::pages_routes;
	#[cfg(feature = "config")]
	pub use crate::config_plugin::*;
	pub use crate::types::*;
}
