pub mod ml_plugin;
pub mod models;

pub extern crate beet_ecs as beet;

pub mod prelude {
	pub use crate::ml_plugin::selectors::*;
	pub use crate::ml_plugin::*;
	pub use crate::models::*;
}
