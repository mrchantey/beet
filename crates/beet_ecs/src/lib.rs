pub mod action;
pub mod builtin_nodes;
pub mod edge;
pub mod extensions;
pub mod graph;
pub mod node;
pub mod ui;

// currently required for action_list! macro to work
extern crate self as beet;
extern crate self as beet_ecs;

pub mod prelude {

	pub use crate::action::*;
	pub use crate::builtin_nodes::actions::*;
	pub use crate::builtin_nodes::selectors::*;
	pub use crate::builtin_nodes::*;
	pub use crate::edge::*;
	pub use crate::extensions::*;
	pub use crate::graph::*;
	pub use crate::node::*;
	pub use crate::ui::*;
	pub use beet_ecs_macros::*;
}


pub mod exports {
	pub use bevy_ecs::prelude::*;
	pub use bevy_ecs::schedule::SystemConfigs;
	pub use bevy_ecs::system::EntityCommands;
	pub use serde;
	pub use serde::Deserialize;
	pub use serde::Serialize;
	pub use strum_macros::Display;
	pub use strum_macros::EnumIter;
}
