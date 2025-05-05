//! This crate is downstream of `beet_rsx` unlike `beet_rsx_parser`
//!
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![cfg_attr(test, feature(stmt_expr_attributes))]
#![feature(let_chains, if_let_guard, result_flattening)]

#[cfg(feature = "bevy")]
mod bevy;

mod build_codegen;
mod build_codegen_actions;
mod build_templates;
mod utils;

pub mod prelude {
	#[cfg(feature = "bevy")]
	pub use crate::bevy::*;

	pub use crate::build_codegen::*;
	pub use crate::build_codegen_actions::*;
	pub use crate::build_templates::*;
	pub use crate::utils::*;
}


pub mod exports {
	pub use proc_macro2;
	pub use proc_macro2_diagnostics;
	pub use syn;
}
