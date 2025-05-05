//! This crate is upstream of `beet_rsx` unlike `beet_build`
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![feature(associated_type_defaults, if_let_guard, let_chains)]


pub mod derive_node;
pub mod parse_rsx;
pub mod utils;


pub mod prelude {
	pub use crate::derive_node::*;
	pub use crate::parse_rsx::*;
	pub use crate::utils::*;

	pub use rstml::node::Node as RstmlNode;
}
