#![allow(deprecated)] // TODO remove deprecated
#![feature(result_flattening, let_chains)]
pub mod action;
pub mod actions;
pub mod events;
pub mod extensions;
pub mod graph;
pub mod lifecycle;
pub mod observers;
pub mod reflect;
#[cfg(any(test, feature = "test"))]
pub mod test;
pub mod tree;

// required for action macros
extern crate self as beet_ecs;

pub mod prelude {
	pub use crate::action::*;
	pub use crate::actions::flow::*;
	pub use crate::actions::leaf::*;
	pub use crate::actions::global::*;
	pub use crate::actions::*;
	pub use crate::events::*;
	pub use crate::extensions::*;
	pub use crate::graph::*;
	pub use crate::lifecycle::actions::*;
	pub use crate::lifecycle::beet_debug_plugin::*;
	pub use crate::lifecycle::components::*;
	pub use crate::lifecycle::lifecycle_plugin::*;
	pub use crate::lifecycle::lifecycle_systems_plugin::*;
	pub use crate::lifecycle::selectors::*;
	pub use crate::observers::observer_utils::*;
	pub use crate::observers::*;
	// pub use crate::lifecycle::*;
	pub use crate::reflect::*;
	#[cfg(any(test, feature = "test"))]
	pub use crate::test::*;
	pub use crate::tree::*;
	pub use beet_ecs_macros::*;
}
