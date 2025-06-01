//! This crate is downstream of `beet_rsx` unlike `beet_rsx_parser`
//!
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![cfg_attr(test, feature(stmt_expr_attributes))]
#![feature(let_chains, if_let_guard, result_flattening)]

mod bevy;
mod build_codegen;
mod build_codegen_actions;
mod build_templates;
mod config;
mod utils;
pub mod prelude {
	// pub use crate::bevy::*;
	// pub use crate::build_codegen::*;
	// pub use crate::build_codegen_actions::*;
	pub use crate::build_templates::*;
	pub use crate::config::*;
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
		pub use beet_rsx as rsx;
		pub mod prelude {
			pub use crate::prelude::*;
			pub use beet_common::prelude::*;
			pub use beet_parse::prelude::*;
			pub use beet_rsx::prelude::*;
		}
		pub mod exports {
			pub use crate::exports::*;
			pub use beet_rsx::exports::*;
		}
	}
}
