#![doc = include_str!("../README.md")]
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![cfg_attr(test, feature(stmt_expr_attributes))]
#![feature(let_chains, if_let_guard, result_flattening, exit_status_error)]

mod static_scene;
mod client_island_codegen;
mod route_codegen;
mod file_parsing;

pub mod prelude {
	pub use crate::static_scene::*;
	pub use crate::client_island_codegen::*;
	pub use crate::route_codegen::*;
	pub use crate::file_parsing::*;
}
pub mod exports {
	pub use proc_macro2;
	pub use proc_macro2_diagnostics;
	pub use syn;
}
pub mod as_beet {
	pub use beet::prelude::*;
	pub mod beet {
		pub use crate as build;
		pub use beet_parse as parse;
		pub use beet_template as template;
		pub mod prelude {
			pub use crate::prelude::*;
			pub use beet_utils::prelude::*;
			pub use beet_common::prelude::*;
			pub use beet_parse::prelude::*;
			pub use beet_template::prelude::*;
		}
		pub mod exports {
			pub use crate::exports::*;
			pub use beet_template::exports::*;
		}
	}
}
