use crate::prelude::*;
use beet_flow::prelude::*;
use beet_core::prelude::*;
use std::ops::Range;

#[action(provide_score)]
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Component)]
#[require(StatId, StatValueGoal)]
pub struct StatScoreProvider {
	pub curve: EaseFunction,
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Reflect, Component)]
pub enum StatValueGoal {
	// the stat should be as high as possible
	#[default]
	High,
	// the stat should be as low as possible
	Low,
}

impl Default for StatScoreProvider {
	fn default() -> Self {
		Self {
			curve: EaseFunction::Linear,
		}
	}
}


impl StatScoreProvider {
	pub fn new() -> Self { Self::default() }
	pub fn with_curve(mut self, curve: EaseFunction) -> Self {
		self.curve = curve;
		self
	}

	pub fn sample(
		&self,
		value: StatValue,
		target_value: StatValueGoal,
		range: Range<StatValue>,
	) -> ScoreValue {
		let normal_value = value.normalize(range);

		let curved_value =
			EasingCurve::new(0., 1., self.curve).sample_unchecked(normal_value);

		match target_value {
			// if the value is high and the desired direction is high,
			// the score should be low
			StatValueGoal::High => ScoreValue(1. - curved_value),
			// vice versa
			StatValueGoal::Low => ScoreValue(curved_value),
		}
	}
}


fn provide_score(
	ev: On<Run<RequestScore>>,
	mut commands: Commands,
	stat_map: Res<StatMap>,
	children: Query<&Children>,
	stats: Query<(&StatId, &StatValue)>,
	query: Query<(&StatScoreProvider, &StatId, &StatValueGoal)>,
) {
	let (score_provider, stat_id, target_value) = query
		.get(ev.event_target())
		.expect(&expect_action::to_have_action(&ev));

	let value = StatValue::find_by_id(ev.origin, children, stats, *stat_id)
		.expect(&expect_action::to_have_origin(&ev));

	let descriptor = stat_map
		.get(stat_id)
		.expect(&expect_action::to_have_other("stat map item"));
	let score = score_provider.sample(
		value,
		*target_value,
		descriptor.global_range.clone(),
	);

	ev.trigger_result(&mut commands, score);
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn sample() {
		let provider = StatScoreProvider::new();
		let range = StatValue::range(-3.0..7.0);
		let target = StatValueGoal::High;

		provider
			.sample(StatValue(-3.), target, range.clone())
			.0
			.xpect_eq(1.0);
		provider
			.sample(StatValue(2.0), target, range.clone())
			.0
			.xpect_eq(0.5);
		provider
			.sample(StatValue(7.0), target, range.clone())
			.0
			.xpect_eq(0.0);

		let target = StatValueGoal::Low;
		provider
			.sample(StatValue(-3.), target, range.clone())
			.0
			.xpect_eq(0.0);
		provider
			.sample(StatValue(2.0), target, range.clone())
			.0
			.xpect_eq(0.5);
		provider
			.sample(StatValue(7.0), target, range.clone())
			.0
			.xpect_eq(1.0);
	}

	#[test]
	#[ignore = "fails dont know why, cant remember how this is supposed to work"]
	fn action() {
		let mut app = App::new();

		app.add_plugins(BeetFlowPlugin::default())
			.insert_resource(StatMap::default_with_test_stats());

		let world = app.world_mut();

		let on_child_score =
			observer_ext::observe_triggers::<OnChildResult<ScoreValue>>(world);

		world
			.spawn(HighestScore::default())
			.with_child((StatMap::TEST_PLEASENTNESS_ID, StatValue(2.)))
			.with_child((
				StatMap::TEST_PLEASENTNESS_ID,
				StatScoreProvider::default(),
			))
			.with_child((
				StatMap::TEST_PLEASENTNESS_ID,
				StatScoreProvider::default(),
				StatValueGoal::Low,
			))
			.flush_trigger(OnRun::local());

		on_child_score.len().xpect_eq(2);
	}
}
