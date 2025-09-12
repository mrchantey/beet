//! This crate exists as an upstream dependency for utilities used by sweet,
//! which is depended upon by all other crates
mod cross_log;
mod glob_filter;
mod path_utils;
#[cfg(feature = "tokens")]
pub mod pkg_ext;
#[cfg(feature = "fs")]
pub mod terminal;
mod workspace_root;
mod xtend;

pub mod prelude {
	pub use crate::abs_file;
	pub use crate::cross_log;
	pub use crate::cross_log_error;
	pub use crate::dir;
	pub use crate::glob_filter::*;
	pub use crate::path_utils::*;
	#[cfg(feature = "tokens")]
	pub use crate::pkg_ext;
	#[cfg(feature = "fs")]
	pub use crate::terminal;
	pub use crate::workspace_root::*;
	pub use crate::xtend::*;
}

pub mod exports {
	#[cfg(target_arch = "wasm32")]
	pub use web_sys;
}
