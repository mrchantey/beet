use crate::prelude::*;
use beet_core::prelude::*;


/// The payload for requesting a score, for example usage see [`HighestScore`].
#[derive(
	Debug, Default, Copy, Clone, PartialEq, PartialOrd, Component, Reflect,
)]
pub struct GetScore;

/// Wrapper for an f32, representing a score. This should be between 0 and 1.
///	## Example
/// ```rust
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// let mut world = World::new();
/// // create a passing score value
/// world.spawn(EndOnRun(Score(1.)));
/// ```
#[derive(
	Debug,
	Default,
	Copy,
	Clone,
	PartialEq,
	PartialOrd,
	Deref,
	DerefMut,
	Component,
	Reflect,
)]
pub struct Score(pub f32);

impl Score {
	/// Its best practice to keep scores between 0 and 1,
	/// so a passing score is 1
	pub const PASS: Self = Self(1.0);
	/// Its best practice to keep scores between 0 and 1,
	/// so a neutral score is 0.5
	pub const NEUTRAL: Self = Self(0.5);
	/// Its best practice to keep scores between 0 and 1,
	/// so a failing score is 0
	pub const FAIL: Self = Self(0.0);
	/// Create a new instance of `Score` with the provided score.
	pub fn new(score: f32) -> Self { Self(score) }
}

impl EventPayload for Score {
	type Event = End<Score>;
	fn into_event(self, entity: Entity) -> Self::Event {
		End::new(entity, self)
	}
}

impl EventPayload for GetScore {
	type Event = Run<GetScore>;
	fn into_event(self, entity: Entity) -> Self::Event {
		Run::new(entity, self)
	}
}

impl RunPayload for GetScore {
	type End = Score;
}
impl EndPayload for Score {
	type Run = GetScore;
}
