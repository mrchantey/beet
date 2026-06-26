use crate::prelude::*;
use beet_core::prelude::*;

/// Utility AI selector, aka `UtilitySelector`.
///
/// Each child is scored via its [`ScoreProvider`], or a plain [`Score`]
/// component used as a fixed value. This selector scores every child, then runs
/// the highest-scoring one and returns its [`Outcome`].
///
/// ## Example
/// ```
/// # use beet_action::prelude::*;
/// # use beet_core::prelude::*;
/// # let mut world = AsyncPlugin::world();
/// world.spawn((HighestScore::new(), children![
/// 	(ScoreProvider::<()>::fixed(Score::NEUTRAL), Action::<(), Outcome>::new_fixed(Outcome::PASS)),
/// 	(ScoreProvider::<()>::fixed(Score::PASS), Action::<(), Outcome>::new_fixed(Outcome::PASS)),
/// ]));
/// ```
#[derive(Debug, Clone, Copy, Component, Reflect)]
#[require(HighestScoreAction<Input,Output>)]
#[reflect(Component, Default)]
pub struct HighestScore<Input = (), Output = ()>
where
	Input: 'static + Send + Sync + Clone,
	Output: 'static + Send + Sync,
{
	#[reflect(ignore)]
	_marker: PhantomData<fn() -> (Input, Output)>,
}

impl<Input, Output> Default for HighestScore<Input, Output>
where
	Input: 'static + Send + Sync + Clone,
	Output: 'static + Send + Sync,
{
	fn default() -> Self {
		Self {
			_marker: PhantomData,
		}
	}
}

impl HighestScore {
	/// Create a default `HighestScore<(), ()>`.
	pub fn new() -> Self { Self::default() }
}

/// Scores every child via its [`ScoreProvider`] or fixed [`Score`], then runs
/// the highest scorer.
///
/// ## Errors
///
/// Errors if the selector has no children, or a child has neither a
/// [`ScoreProvider`] nor a [`Score`].
#[action(default)]
#[derive(Component)]
pub async fn HighestScoreAction<Input, Output>(
	cx: ActionContext<Input>,
) -> Result<Outcome<Input, Output>>
where
	Input: 'static + Send + Sync + Clone,
	Output: 'static + Send + Sync,
{
	let children = cx
		.caller
		.get(|children: &Children| children.to_vec())
		.await
		.map_err(|_| bevyhow!("HighestScore has no children"))?;

	let world = cx.world().clone();
	let input = cx.input;

	let mut best: Option<(Entity, f32)> = None;
	for child in children {
		// score via the child's provider if present, else a plain `Score`
		// component used as a fixed value, else error.
		let score = if let Ok(provider) = world
			.entity(child)
			.get_cloned::<ScoreProvider<Input>>()
			.await
		{
			let Score(score) = world
				.entity(child)
				.call_detached(provider.0, input.clone())
				.await?;
			score
		} else if let Ok(score) =
			world.entity(child).get(|score: &Score| score.0).await
		{
			score
		} else {
			bevybail!(
				"HighestScore child {child:?} has no ScoreProvider or Score"
			);
		};

		if best.map(|(_, best)| score > best).unwrap_or(true) {
			best = Some((child, score));
		}
	}

	let (winner, _) =
		best.ok_or_else(|| bevyhow!("HighestScore has no scored children"))?;

	world
		.entity(winner)
		.call::<Input, Outcome<Input, Output>>(input)
		.await
}

#[cfg(test)]
mod tests {
	use super::*;

	fn pass() -> Action<(), Outcome<(), ()>> {
		Action::new_pure(|_: ActionContext| Outcome::Pass(()).xok())
	}

	#[beet_core::test]
	async fn picks_highest() {
		AsyncPlugin::world()
			.spawn((HighestScore::new(), children![
				(ScoreProvider::<()>::fixed(Score::NEUTRAL), pass()),
				(ScoreProvider::<()>::fixed(Score::PASS), pass()),
				(ScoreProvider::<()>::fixed(Score::FAIL), pass()),
			]))
			.call::<(), Outcome<(), ()>>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::Pass(()));
	}

	#[beet_core::test]
	async fn runs_winner_only() {
		// the winning child returns Fail, losers would return Pass
		fn fail() -> Action<(), Outcome<(), ()>> {
			Action::new_pure(|_: ActionContext| Outcome::Fail(()).xok())
		}
		AsyncPlugin::world()
			.spawn((HighestScore::new(), children![
				(ScoreProvider::<()>::fixed(Score::NEUTRAL), pass()),
				(ScoreProvider::<()>::fixed(Score::PASS), fail()),
			]))
			.call::<(), Outcome<(), ()>>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::Fail(()));
	}

	#[beet_core::test]
	async fn scores_plain_score_component() {
		// children carry a fixed `Score` directly, no `ScoreProvider`; the
		// 0.6 child wins and its action (Fail) runs over the 0.4 child (Pass).
		fn fail() -> Action<(), Outcome<(), ()>> {
			Action::new_pure(|_: ActionContext| Outcome::Fail(()).xok())
		}
		AsyncPlugin::world()
			.spawn((HighestScore::new(), children![
				(Score(0.4), pass()),
				(Score(0.6), fail()),
			]))
			.call::<(), Outcome<(), ()>>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::Fail(()));
	}

	#[beet_core::test]
	async fn missing_provider_errors() {
		AsyncPlugin::world()
			.spawn((HighestScore::new(), children![pass()]))
			.call::<(), Outcome<(), ()>>(())
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("no ScoreProvider");
	}
}
