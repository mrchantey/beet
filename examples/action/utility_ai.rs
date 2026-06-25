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
//! cargo run --example utility_ai --features action
//! ```
use beet::prelude::*;

fn main() -> AppExit {
	App::new()
		.add_plugins((MinimalPlugins, LogPlugin::default(), AsyncPlugin))
		.add_systems(Startup, setup)
		.run()
}

fn setup(async_commands: AsyncCommands) {
	async_commands.run(async |world: AsyncWorld| -> Result {
		let selector = world
			.with(|world: &mut World| {
				world
					.spawn((Name::new("selector"), HighestScore::new(), children![
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
					]))
					.id()
			})
			.await;
		let outcome = world.entity(selector).call::<(), Outcome>(()).await?;
		info!("selector finished: {outcome:?}");
		world.write_message(AppExit::Success).await;
		Ok(())
	});
}
