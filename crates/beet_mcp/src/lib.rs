#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![doc = include_str!("../README.md")]

pub mod crate_rag;
pub mod mcp;
pub mod utils;
pub mod vector_db;

pub mod prelude {
	pub use crate::crate_rag::*;
	pub use crate::mcp::*;
	pub use crate::utils::*;
	pub use crate::vector_db::*;
}
