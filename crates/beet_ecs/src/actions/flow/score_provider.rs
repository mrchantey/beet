use crate::prelude::*;
use bevy::prelude::*;

pub type ScoreValue = f32;

/// A constant score provider.
#[derive(Default, Deref, DerefMut, Component, Action, Reflect)]
#[reflect(Default, Component)]
#[category(ActionCategory::ChildBehaviors)]
#[observers(provide_score)]
pub struct ScoreProvider(pub ScoreValue);

impl ScoreProvider {
	pub const FAIL: ScoreProvider = ScoreProvider(0.0);
	pub const PASS: ScoreProvider = ScoreProvider(1.0);
	pub const NEUTRAL: ScoreProvider = ScoreProvider(0.5);

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
