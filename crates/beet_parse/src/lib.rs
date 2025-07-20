//! General parsing utilities for both the beet cli and various macros.
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![feature(if_let_guard, let_chains, exact_size_is_empty)]

pub mod derive;
pub mod lang;
pub mod parse_rsx_tokens;
pub mod tokenize;
pub mod utils;

pub mod prelude {
	pub use crate::derive::*;
	#[allow(unused)]
	pub use crate::lang::*;
	pub use crate::parse_rsx_tokens::*;
	pub use crate::tokenize::*;
	pub use crate::utils::*;
}

pub mod exports {

	pub use send_wrapper::SendWrapper;
}
