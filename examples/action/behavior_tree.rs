//! # Behavior Tree - Sequence Control Flow
//!
//! Control-flow nodes are just actions that call their children and await
//! the results. A [`Sequence`] runs its children in order, threading the
//! input through each, and stops at the first [`Outcome::Fail`].
//!
//! ## How It Works
//!
//! 1. The root entity gets a [`Sequence`] component
//! 2. Two children each carry a [`Log`] leaf action
//! 3. Calling the root runs child1 then child2; both pass, so the
//!    sequence passes
//!
//! Run with:
//! ```sh
//! cargo run --example behavior_tree --features action
//! ```
use beet::prelude::*;

#[beet::main]
async fn main() -> Result {
	let mut world = AsyncPlugin::world();
	let outcome = world
		.spawn((
			Name::new("root"),
			Sequence::new(),
			children![
				(Name::new("child1"), Log::new("running child1")),
				(Name::new("child2"), Log::new("running child2")),
			],
		))
		.call::<(), Outcome>(())
		.await?;
	cross_log!("sequence finished: {outcome:?}");
	Ok(())
}
