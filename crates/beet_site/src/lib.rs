#![feature(more_qualified_paths)]

pub mod components;
pub mod routes;

pub mod prelude {
	pub use super::*;
	pub use crate::components::*;
}
