use crate::prelude::*;
use bevy::prelude::*;
pub type ScoreValue = f32;

pub mod score{
    use super::flow::ScoreValue;
	pub const FAIL: ScoreValue = 0.0;
	pub const PASS: ScoreValue = 1.0;
	pub const NEUTRAL: ScoreValue = 0.5;
}


/// A constant score provider.
#[derive(Default, Deref, DerefMut, Component, Action, Reflect)]
#[reflect(Default, Component)]
#[category(ActionCategory::ChildBehaviors)]
#[observers(provide_score)]
pub struct ScoreProvider(pub ScoreValue);

impl ScoreProvider {
	pub const FAIL: ScoreProvider = ScoreProvider(score::FAIL);
	pub const PASS: ScoreProvider = ScoreProvider(score::PASS);
	pub const NEUTRAL: ScoreProvider = ScoreProvider(score::NEUTRAL);

	pub fn new(score: ScoreValue) -> Self { Self(score) }
}

fn provide_score(
	trigger: Trigger<RequestScore>,
	mut commands: Commands,
	query: Query<(&ScoreProvider, &Parent)>,
) {
	let (score_provider, parent) = query
		.get(trigger.entity())
		.expect(expect_action::ACTION_QUERY_MISSING);

	commands
		.entity(parent.get())
		.trigger(OnChildScore::new(trigger.entity(), **score_provider));
}
