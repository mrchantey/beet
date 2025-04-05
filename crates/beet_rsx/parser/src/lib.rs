#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![feature(associated_type_defaults)]


pub mod node_tokens;
pub mod parse_node;
pub mod parse_rsx;
pub mod utils;


pub mod prelude {
	pub use crate::node_tokens::*;
	pub use crate::parse_node::*;
	pub use crate::parse_rsx::*;
	pub use crate::utils::*;

	pub use rstml::node::Node as RstmlNode;
}
