mod components;
pub use components::*;
pub mod routes;



use crate::as_beet::*;

/// Gets the [`FileGroupConfig`] for this crate
pub fn setup_config() -> FileGroupConfig {
	FileGroupConfig::new(app_cx!())
	// .add_group(mockups())
}
