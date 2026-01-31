#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
#![deny(missing_docs)]
#![doc = include_str!("../README.md")]
//!
//! # Concepts - Action
//!
//! Actions are entities that respond to being run by eventually returning
//! either an [`Outcome::Pass`] or [`Outcome::Fail`]. They form the building
//! blocks of behavior trees and other control flow patterns.
//!
//! # Creating Actions
//!
//! The simplest way to create an action is with the `#[action]` attribute macro:
//!
//! ```ignore
//! # use beet_core::prelude::*;
//! # use beet_flow::prelude::*;
//! #[action(my_handler)]
//! #[derive(Component)]
//! struct MyAction;
//!
//! fn my_handler(ev: On<GetOutcome>, mut commands: Commands) {
//!     commands.entity(ev.target()).trigger_target(Outcome::Pass);
//! }
//! ```
//!
//! ## Built-in Actions
//!
//! Common control flow actions include:
//! - [`Sequence`]: Runs children in order until one fails
//! - [`Fallback`]: Runs children in order until one succeeds
//! - [`Parallel`]: Runs all children simultaneously
//! - [`HighestScore`]: Runs the highest-scoring child (Utility AI)
//! 
//! # Concepts - Agent
//!
//! It is common for actions to have a single "target" entity they operate on.
//! For example, each node in a behavior tree might modify a [`Transform`].
//! The agent is the entity that owns the behavior, not the action entities
//! themselves.
//!
//! Use [`AgentQuery`] to resolve the agent for a given action entity.
//!
//! # Agent Resolution
//!
//! The agent is resolved in this order:
//! 1. The first [`ActionOf`] relationship in ancestors (inclusive)
//! 2. The root ancestor if no [`ActionOf`] is found
//!
//! # Example
//!
//! ```ignore
//! # use beet_core::prelude::*;
//! # use beet_flow::prelude::*;
//! fn my_system(
//!     ev: On<GetOutcome>,
//!     agents: AgentQuery<&Transform>,
//! ) {
//!     // Get the transform of the agent, not the action entity
//!     if let Ok(transform) = agents.get(ev.target()) {
//!         // Use the agent's transform
//!     }
//! }
//! ```

#[cfg(feature = "bevy_default")]
#[allow(unused, reason = "docs")]
use crate::prelude::*;

mod actions;
mod types;

/// A prelude for beet_flow, re-exporting the most commonly used items.
pub mod prelude {
	pub use crate::actions::*;
	pub use crate::types::*;

	#[cfg(doc)]
	pub use beet_core::prelude::*;
}
