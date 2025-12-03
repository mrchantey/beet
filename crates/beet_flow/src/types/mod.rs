//! General purpose types used by actions in beet_flow.
mod action_context;
mod debug_flow_plugin;
mod outcome;
mod ready;
pub use action_context::*;
pub use debug_flow_plugin::*;
pub use ready::*;
mod continue_run;
pub use continue_run::*;
pub use outcome::*;
mod lifecycle;
pub use lifecycle::*;
pub mod expect_action;
mod score;
pub use score::*;
mod control_flow_plugin;
pub use control_flow_plugin::*;
mod run_timer;
pub use run_timer::*;
mod action_event;
mod async_action;
pub use action_event::*;
pub use async_action::*;
mod agent;
pub use agent::*;
mod target_entity;
pub use target_entity::*;


/// > A no-op struct used for documentation purposes
///
/// Actions are entities that respond to being run by
/// eventually returning either an [`Outcome::Pass`] or [`Outcome::Fail`].
/// A common example of an [`Action`] is an entity with a [`Sequence`] component.
///
/// ```rust
/// # use bevy::prelude::*;
/// # use beet_flow::prelude::*;
/// # let mut world = World::new();
/// let my_action = world.spawn(Sequence);
/// ```
pub struct Action;

/// > A no-op struct used for documentation purposes
/// It is common for actions to have a single 'target' entity,
/// for example each node in a behavior tree will do work on
/// the entity with the [`Transform`], so the [`ActionContext`]
/// tracks this entity as it traverses the tree.sd
pub struct Agent;
