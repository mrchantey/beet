#![feature(result_flattening)]
pub mod action;
pub mod channel_event;
pub mod ecs_nodes;
pub mod edge;
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
	pub use crate::channel_event::*;
	pub use crate::ecs_nodes::actions::*;
	pub use crate::ecs_nodes::selectors::*;
	pub use crate::ecs_nodes::*;
	pub use crate::edge::*;
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


pub mod exports {
	// pub use bevy::ecs as bevy_ecs;
	pub use bevy;
	pub use bevy::ecs::schedule::ScheduleLabel;
	pub use bevy::ecs::system::EntityCommands;
	pub use bevy::prelude::*;
	pub use bevy::reflect as bevy_reflect;
	pub use bevy::reflect::FromReflect;
	pub use bevy::reflect::GetTypeRegistration;
	pub use bevy::reflect::Reflect;
	pub use bevy::reflect::TypePath;
	pub use bevy::reflect::TypeRegistry;
	pub use strum::IntoEnumIterator;
	pub use strum_macros::Display;
	pub use strum_macros::EnumIter;
}
