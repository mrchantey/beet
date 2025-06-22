#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![cfg_attr(test, feature(stmt_expr_attributes))]
#![feature(let_chains, if_let_guard, result_flattening)]

mod router_codegen;
mod codegen_wasm;
// mod build_codegen_actions;
mod beet_config;
mod build_templates;
mod utils;

pub mod prelude {
	// pub use crate::bevy::*;
	pub use crate::router_codegen::*;
	pub use crate::codegen_wasm::*;
	// pub use crate::build_codegen_actions::*;
	pub use crate::beet_config::*;
	pub use crate::build_templates::*;
	pub use crate::utils::*;
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
