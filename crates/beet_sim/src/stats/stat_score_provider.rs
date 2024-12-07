use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Component, Reflect, Action)]
#[observers(provide_score)]
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
			StatValueGoal::High => 1. - curved_value,
			// vice versa
			StatValueGoal::Low => curved_value,
		}
	}
}


fn provide_score(
	trigger: Trigger<RequestScore>,
	mut commands: Commands,
	stat_map: Res<StatMap>,
	children: Query<&Children>,
	stats: Query<(&StatId, &StatValue)>,
	query: Query<(
		&StatScoreProvider,
		&StatId,
		&StatValueGoal,
		&Parent,
		&TargetEntity,
	)>,
) {
	let (score_provider, stat_id, target_value, parent, target_entity) = query
		.get(trigger.entity())
		.expect(expect_action::ACTION_QUERY_MISSING);

	let value =
		StatValue::find_by_id(**target_entity, children, stats, *stat_id)
			.expect(expect_action::TARGET_MISSING);

	let descriptor = stat_map.get(stat_id).unwrap();
	let score = score_provider.sample(
		value,
		*target_value,
		descriptor.global_range.clone(),
	);

	commands
		.entity(parent.get())
		.trigger(OnChildScore::new(trigger.entity(), score));
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use beet_flow::prelude::*;
	use beetmash::prelude::*;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	fn sample() -> Result<()> {
		let provider = StatScoreProvider::new();
		let range = StatValue::range(-3.0..7.0);
		let target = StatValueGoal::High;

		expect(provider.sample(StatValue(-3.), target, range.clone()))
			.to_be(1.0)?;
		expect(provider.sample(StatValue(2.0), target, range.clone()))
			.to_be(0.5)?;
		expect(provider.sample(StatValue(7.0), target, range.clone()))
			.to_be(0.0)?;

		let target = StatValueGoal::Low;
		expect(provider.sample(StatValue(-3.), target, range.clone()))
			.to_be(0.0)?;
		expect(provider.sample(StatValue(2.0), target, range.clone()))
			.to_be(0.5)?;
		expect(provider.sample(StatValue(7.0), target, range.clone()))
			.to_be(1.0)?;

		Ok(())
	}

	#[test]
	fn action() -> Result<()> {
		let mut app = App::new();

		app.add_plugins(
			ActionPlugin::<(ScoreFlow, StatScoreProvider)>::default(),
		)
		.insert_resource(StatMap::default_with_test_stats());

		let world = app.world_mut();

		let on_child_score = observe_triggers::<OnChildScore>(world);
		// let on_child_score =
		// 	observe_trigger_mapped(world, |trigger: Trigger<OnChildScore>| {
		// 		*trigger.event().value()
		// 	});

		let agent = world
			.spawn(())
			// 2 in range -5..5
			.with_child((StatMap::TEST_PLEASENTNESS_ID, StatValue(2.)))
			.id();

		world
			.spawn(ScoreFlow::default())
			.with_children(|parent| {
				parent.spawn((
					TargetEntity(agent),
					StatMap::TEST_PLEASENTNESS_ID,
					StatScoreProvider::default(),
				));
				parent.spawn((
					TargetEntity(agent),
					StatMap::TEST_PLEASENTNESS_ID,
					StatScoreProvider::default(),
					StatValueGoal::Low,
				));
			})
			.flush_trigger(OnRun);

		expect(&on_child_score).to_have_been_called_times(2)?;

		// expect(&on_child_score).to_have_returned_nth_with(0, &0.3)?;
		// expect(&on_child_score).to_have_returned_nth_with(1, &0.7)?;

		Ok(())
	}
}
