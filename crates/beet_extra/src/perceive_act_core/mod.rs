//! Shared perceive-act types the agent and every head/body client use in common.
//!
//! Split out of the `thread`-gated `perceive_act` module (the native agent, which pulls
//! `beet_thread`) so the wasm browser head ([`perceive_act_web`](crate::perceive_act_web))
//! reuses the same wire types and client primitives without dragging the native LLM in.
//! Rides only `json` + the socket core + `beet_action`/`beet_router`, all of which both
//! the `thread` and `perceive_act_web` feature sets provide.
mod tools;
pub use tools::*;
mod client;
pub use client::*;

use beet_core::prelude::*;

/// Registers the shared perceive-act types ([`Emotion`], [`WhoAmI`], [`ClientRole`]).
/// Added idempotently by both `PerceiveActPlugin` (the agent) and `PerceiveActWebPlugin`
/// (the browser head), so whichever is present registers the common types once.
#[derive(Default)]
pub struct PerceiveActCorePlugin;

impl Plugin for PerceiveActCorePlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<Emotion>()
			.register_type::<ClientRole>()
			.register_type::<WhoAmI>();
	}
}
