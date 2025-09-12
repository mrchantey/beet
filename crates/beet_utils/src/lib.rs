#![cfg_attr(
	feature = "nightly",
	feature(fn_traits, unboxed_closures, exit_status_error)
)]
pub use utils::async_ext;
pub use utils::time_ext;

pub mod arena;
mod bevy_utils;
pub mod extensions;
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
pub mod fs;
mod mini_utils;
pub mod path_utils;
#[cfg(feature = "tokens")]
mod tokens_utils;
pub mod utils;
pub mod prelude {
	pub use crate::cross_log;
	pub use crate::cross_log_error;

	pub use crate::abs_file;
	pub use crate::arena::*;
	pub use crate::bevy_utils::*;
	pub use crate::bevybail;
	pub use crate::bevyhow;
	pub use crate::dir;
	pub use crate::extensions::*;
	#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
	pub use crate::fs::*;
	pub use crate::mini_utils::*;
	pub use crate::path_utils::*;
	#[cfg(feature = "tokens")]
	pub use crate::tokens_utils::*;
	pub use crate::utils::*;
	#[cfg(feature = "rand")]
	pub use rand::Rng;
}
pub mod exports {
	#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
	pub use crate::fs::exports::*;
	pub use glob;
	#[cfg(target_arch = "wasm32")]
	pub use wasm_exports::*;
	#[cfg(target_arch = "wasm32")]
	mod wasm_exports {
		pub use js_sys;
		pub use wasm_bindgen;
		pub use wasm_bindgen_futures;
		pub use web_sys;
	}
}
