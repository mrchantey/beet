mod components;
pub use components::*;
// #[path = "routes/docs/index.rs"]
// mod foo;
#[path = "codegen/routes.rs"]
pub mod routes;


use crate::as_beet::*;

/// Gets the [`AppConfig`] for this crate
pub fn config() -> AppConfig {
	AppConfig::new()
	// .add_group(mockups())
}
