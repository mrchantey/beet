#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![allow(deprecated)] // TODO remove deprecated
#![feature(
	result_flattening,
	let_chains,
	associated_type_defaults,
	trait_upcasting
)]
pub mod action_builder;
pub mod actions;
#[cfg(feature = "bevyhub")]
pub mod bevyhub_plugins;
pub mod events;
pub mod extensions;
pub mod graph;
pub mod lifecycle;
#[cfg(feature = "net")]
pub mod net;
pub mod observers;
pub mod reflect;
pub mod tree;

// required for action macros
extern crate self as beet_flow;

pub mod prelude {
	pub use crate::action_builder::*;
	pub use crate::actions::flow::*;
	#[allow(ambiguous_glob_reexports)]
	pub use crate::actions::global::*;
	pub use crate::actions::misc::*;
	#[allow(ambiguous_glob_reexports)]
	pub use crate::actions::on_trigger::*;
	#[allow(ambiguous_glob_reexports)]
	pub use crate::actions::*;
	#[cfg(feature = "bevyhub")]
	pub use crate::bevyhub_plugins::*;
	pub use crate::build_observer_hooks;
	pub use crate::events::*;
	pub use crate::extensions::*;
	pub use crate::graph::*;
	pub use crate::lifecycle::components::*;
	pub use crate::lifecycle::lifecycle_plugin::*;
	pub use crate::lifecycle::lifecycle_systems_plugin::*;
	#[cfg(feature = "net")]
	pub use crate::net::*;
	pub use crate::observers::*;
	// pub use crate::lifecycle::*;
	pub use crate::reflect::*;
	pub use crate::tree::*;
	pub use beet_flow_macros::*;
}
