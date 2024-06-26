#![feature(result_flattening, let_chains)]
pub mod action;
pub mod extensions;
pub mod graph;
pub mod lifecycle;
pub mod reflect;
#[cfg(test)]
pub mod test;
pub mod tree;

pub mod prelude {
	pub use crate::action::*;
	pub use crate::extensions::*;
	pub use crate::graph::*;
	pub use crate::lifecycle::actions::*;
	pub use crate::lifecycle::beet_debug_plugin::*;
	pub use crate::lifecycle::components::*;
	pub use crate::lifecycle::lifecycle_plugin::*;
	pub use crate::lifecycle::lifecycle_systems_plugin::*;
	pub use crate::lifecycle::selectors::*;
	// pub use crate::lifecycle::*;
	pub use crate::reflect::*;
	#[cfg(test)]
	pub use crate::test::*;
	pub use crate::tree::*;
	// pub use beet_ecs_macros::*;
}
