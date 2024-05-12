#![feature(let_chains)]
pub mod ml_plugin;
pub mod models;
#[cfg(target_arch = "wasm32")]
pub mod wasm;

pub mod prelude {
	pub use crate::ml_plugin::selectors::*;
	pub use crate::ml_plugin::*;
	pub use crate::models::*;
	#[cfg(target_arch = "wasm32")]
	pub use crate::wasm::*;
}
