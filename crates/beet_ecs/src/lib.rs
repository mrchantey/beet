#![allow(deprecated)] // TODO remove deprecated
#![feature(result_flattening, let_chains, associated_type_defaults)]
pub mod action;
pub mod action_builder;
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
	pub use crate::action_builder::*;
	pub use crate::action::*;
	pub use crate::actions::flow::*;
	#[allow(ambiguous_glob_reexports)]
	pub use crate::actions::global::*;
	#[allow(ambiguous_glob_reexports)]
	pub use crate::actions::on_trigger::*;
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
	pub use crate::observers::*;
	// pub use crate::lifecycle::*;
	pub use crate::reflect::*;
	#[cfg(any(test, feature = "test"))]
	pub use crate::test::*;
	pub use crate::tree::*;
	pub use beet_ecs_macros::*;
}
