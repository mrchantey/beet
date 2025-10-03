//! This crate exists as an upstream dependency for utilities used by sweet,
//! which is depended upon by all other crates.
//! It should not be depended upon or referred to anywhere except for `beet_core`
//! where all types and macros are re-exported, and in `sweet`.
mod cross_log;
mod glob_filter;
#[cfg(target_arch = "wasm32")]
pub mod js_runtime;
mod path_utils;
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
pub mod terminal;
#[cfg(feature = "tokens")]
mod tokens_utils;
mod workspace_root;
mod xtend;

pub mod prelude {
	pub use crate::abs_file;
	pub use crate::cross_log;
	pub use crate::cross_log_error;
	pub use crate::dir;
	pub use crate::glob_filter::*;
	#[cfg(target_arch = "wasm32")]
	pub use crate::js_runtime;
	pub use crate::path_utils::*;
	#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
	pub use crate::terminal;
	#[cfg(feature = "tokens")]
	pub use crate::tokens_utils::*;
	pub use crate::workspace_root::*;
	pub use crate::xtend::*;
}

pub mod exports {
	#[cfg(target_arch = "wasm32")]
	pub use web_sys;
}
