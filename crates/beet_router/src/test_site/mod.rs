mod components;
pub use components::*;
pub mod routes;



use crate::as_beet::*;

/// Gets the [`AppConfig`] for this crate
pub fn config() -> AppConfig {
	AppConfig::new()
	// .add_group(mockups())
}
