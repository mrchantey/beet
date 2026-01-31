//! General purpose types used by actions in beet_flow.
//!
//! This module provides the foundational types for the control flow system:
//! - [`Outcome`] and [`GetOutcome`]: The primary request/response event pair
//! - [`Score`] and [`GetScore`]: Utility AI scoring events
//! - [`Running`]: Marker for long-running actions
//! - [`AgentQuery`]: Resolves the agent entity for an action
mod debug_flow_plugin;
mod outcome;
mod ready;
mod schedule_label_ext;
pub use debug_flow_plugin::*;
pub use ready::*;
pub use schedule_label_ext::*;
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
mod agent;
pub use agent::*;
mod target_entity;
pub use target_entity::*;
