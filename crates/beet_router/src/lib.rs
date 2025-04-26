#![doc = include_str!("../README.md")]
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![allow(async_fn_in_trait)]
#![feature(more_qualified_paths, if_let_guard)]

#[cfg(feature = "bevy")]
pub mod bevy;
#[cfg(feature = "build")]
pub mod build;
pub mod client_islands;
#[cfg(all(feature = "parser", not(target_arch = "wasm32")))]
pub mod parser;
pub mod server_actions;
pub mod types;

pub mod prelude {
	pub use crate::app_cx;
	#[cfg(feature = "bevy")]
	#[allow(unused_imports)]
	pub use crate::bevy::*;
	#[cfg(feature = "build")]
	pub use crate::build::*;
	pub use crate::client_islands::*;
	#[cfg(all(feature = "parser", not(target_arch = "wasm32")))]
	pub use crate::parser::*;
	pub use crate::server_actions::*;
	pub use crate::types::*;
}


pub mod exports {
	pub use http;
	#[cfg(feature = "parser")]
	pub use ron;
	pub use sweet::prelude::GlobFilter;
	#[cfg(feature = "build")]
	pub use syn;
}

/// expose prelude and as beet for macros
pub mod as_beet {
	pub use beet::prelude::*;
	pub mod beet {
		pub use crate as router;
		pub use beet_rsx as rsx;
		pub mod prelude {
			pub use crate::prelude::*;
			pub use beet_rsx::prelude::*;
		}
		pub mod exports {
			pub use crate::exports::*;
			pub use beet_rsx::exports::*;
		}
	}
}

#[cfg(any(test, feature = "_test_site"))]
pub mod test_site;
