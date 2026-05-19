//! # Utility AI - Score-Based Selection
//!
//! [`HighestScore`] is a selector: each child carries a [`ScoreProvider`]
//! alongside its action. The selector calls every provider, then runs only
//! the highest-scoring child and returns its [`Outcome`].
//!
//! Scores are usually computed from game state (distance, health, threat);
//! here they are fixed for clarity. See `malenia` for dynamic scoring.
//!
//! Run with:
//! ```sh
//! cargo run --example action_utility_ai --features action
//! ```
use beet::prelude::*;

#[beet::main]
async fn main() -> Result {
	let mut world = AsyncPlugin::world();
	let outcome = world
		.spawn((
			Name::new("selector"),
			HighestScore::new(),
			children![
				(
					Name::new("low score, skipped"),
					ScoreProvider::<()>::fixed(Score(0.4)),
					Log::new("this child does not run"),
				),
				(
					Name::new("high score, selected"),
					ScoreProvider::<()>::fixed(Score(0.6)),
					Log::new("this child runs"),
				),
			],
		))
		.call::<(), Outcome>(())
		.await?;
	cross_log!("selector finished: {outcome:?}");
	Ok(())
}
