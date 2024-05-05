#![feature(result_flattening)]
pub mod action;
pub mod beet_module;
pub mod channel_event;
pub mod ecs_module;
pub mod extensions;
pub mod graph;
pub mod inspector_options;
pub mod node;
pub mod reflect;
#[cfg(test)]
pub mod test;
pub mod tree;


// currently required for action_list! macro to work
extern crate self as beet;

pub mod prelude {
	pub use crate::action::*;
	pub use crate::beet_module::*;
	pub use crate::channel_event::*;
	pub use crate::ecs_module::actions::*;
	pub use crate::ecs_module::selectors::*;
	pub use crate::ecs_module::*;
	pub use crate::extensions::*;
	pub use crate::graph::*;
	pub use crate::inspector_options::*;
	pub use crate::node::*;
	pub use crate::reflect::*;
	#[cfg(test)]
	pub use crate::test::*;
	pub use crate::tree::*;
	pub use beet_ecs_macros::*;
}
