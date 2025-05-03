pub use utils::log::*;
pub use utils::sleep::*;
pub mod extensions;
pub mod path_utils;
pub mod utils;
pub mod prelude {
	pub use crate::abs_file;
	pub use crate::extensions::*;
	pub use crate::path_utils::*;
	pub use crate::utils::*;
	#[cfg(feature = "rand")]
	pub use rand::Rng;
}
pub mod exports {
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
