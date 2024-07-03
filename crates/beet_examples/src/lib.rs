// #![allow(unused, dead_code)]

#[cfg(target_arch = "wasm32")]
pub mod wasm;
#[cfg(target_arch = "wasm32")]
pub use wasm::*;
pub mod serde_utils;

pub mod components;
pub mod plugins;


pub mod prelude {
	pub use crate::components::*;
	pub use crate::plugins::*;
	pub use crate::serde_utils::*;
	#[cfg(target_arch = "wasm32")]
	pub use wasm::*;
}
