use crate::prelude::*;
use beet_core::prelude::*;

/// A score value for Utility AI evaluation, the output type of scoring actions.
///
/// By convention scores are normalized to the range `[0.0, 1.0]` where:
/// - `0.0` ([`Score::FAIL`]) indicates lowest priority
/// - `0.5` ([`Score::NEUTRAL`]) indicates neutral priority
/// - `1.0` ([`Score::PASS`]) indicates highest priority
///
/// # Example
/// ```
/// # use beet_action::prelude::*;
/// let score = Score(0.75);
/// # let _ = score;
/// ```
#[derive(
	Debug, Default, Copy, Clone, PartialEq, PartialOrd, Deref, Reflect, Component,
)]
#[reflect(Component, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Score(pub f32);

impl Score {
	/// The maximum score value (1.0), indicating highest priority.
	pub const PASS: Self = Self(1.0);
	/// A neutral score value (0.5).
	pub const NEUTRAL: Self = Self(0.5);
	/// The minimum score value (0.0), indicating lowest priority.
	pub const FAIL: Self = Self(0.0);

	/// Creates a new [`Score`] with the given value.
	pub fn new(score: f32) -> Self { Self(score) }
}

/// Wraps the scoring [`Action`] for a child of a [`HighestScore`] selector.
///
/// A scored child carries two actions: the [`Action`] it runs when selected,
/// and this provider, which the selector calls first to obtain a [`Score`].
#[derive(Component)]
pub struct ScoreProvider<Input = ()>(pub Action<Input, Score>)
where
	Input: 'static + Send + Sync;

impl<Input> Clone for ScoreProvider<Input>
where
	Input: 'static + Send + Sync,
{
	fn clone(&self) -> Self { Self(self.0.clone()) }
}

impl<Input> ScoreProvider<Input>
where
	Input: 'static + Send + Sync,
{
	/// Wrap any action returning a [`Score`].
	pub fn new<M>(action: impl IntoAction<M, In = Input, Out = Score>) -> Self {
		Self(action.into_action())
	}

	/// A provider that always returns the given fixed [`Score`].
	pub fn fixed(score: Score) -> Self {
		Self(Action::new_pure(move |_: ActionContext<Input>| score))
	}
}
