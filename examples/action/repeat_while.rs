//! # Repeat While - Conditional Looping
//!
//! "Repeat while a condition holds" composes from existing pieces:
//!
//! - [`Repeat`] calls its single child in a loop until the child fails
//! - A [`Sequence`] child whose first node is the loop condition
//! - [`SucceedTimes`] is a condition that passes N times, then fails
//!
//! When the condition fails the sequence fails, which stops the repeat.
//!
//! ```text
//! Repeat
//! └── Sequence
//!     ├── SucceedTimes(2)   condition: passes twice, then fails
//!     └── Log               the work performed each iteration
//! ```
//!
//! Run with:
//! ```sh
//! cargo run --example repeat_while --features action
//! ```
use beet::prelude::*;

#[beet::main]
async fn main() -> Result {
	let mut world = AsyncPlugin::world();
	let outcome = world
		.spawn((
			Name::new("root"),
			Repeat::new(),
			children![(
				Name::new("loop body"),
				Sequence::new(),
				children![
					(Name::new("condition"), SucceedTimes::new(2)),
					(Name::new("work"), Log::new("doing work")),
				],
			)],
		))
		.call::<(), Outcome>(())
		.await?;
	cross_log!("loop exited with {outcome:?}");
	Ok(())
}
