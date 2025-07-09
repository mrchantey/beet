#![feature(exit_status_error)]
pub use utils::log::*;
pub use utils::sleep::*;
pub mod arena;
pub mod extensions;
#[cfg(feature = "fs")]
pub mod fs;
pub mod path_utils;
pub mod utils;
pub mod prelude {
	pub use crate::abs_file;
	pub use crate::arena::*;
	pub use crate::dir;
	pub use crate::extensions::*;
	#[cfg(feature = "fs")]
	pub use crate::fs::prelude::*;
	pub use crate::log;
	pub use crate::log_kvp;
	pub use crate::path_utils::*;
	pub use crate::utils::*;
	#[cfg(feature = "rand")]
	pub use rand::Rng;
}
pub mod exports {
	#[cfg(feature = "fs")]
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
