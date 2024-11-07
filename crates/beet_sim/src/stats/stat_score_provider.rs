use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Component, Reflect, Action)]
#[observers(provide_score)]
#[reflect(Component)]
pub struct StatScoreProvider {
	pub stat_id: StatId,
	pub curve: EaseFunction,
	pub desired_direction: DesiredDirection,
}

#[derive(Debug, Default, Clone, PartialEq, Reflect, Action)]
pub enum DesiredDirection {
	#[default]
	Positive,
	Negative,
}


impl StatScoreProvider {
	pub fn new(stat_id: StatId) -> Self {
		Self {
			stat_id,
			curve: EaseFunction::Linear,
			desired_direction: default(),
		}
	}
	pub fn with_curve(mut self, curve: EaseFunction) -> Self {
		self.curve = curve;
		self
	}

	pub fn in_negative_direction(mut self) -> Self {
		self.desired_direction = DesiredDirection::Negative;
		self
	}

	pub fn sample(
		&self,
		value: StatValue,
		range: Range<StatValue>,
	) -> ScoreValue {
		let normal_value =
			(*value - *range.start) / (*range.end - *range.start);

		let curved_value =
			easing_curve(0., 1., self.curve).sample_unchecked(normal_value);

		match self.desired_direction {
			DesiredDirection::Positive => curved_value,
			DesiredDirection::Negative => 1.0 - curved_value,
		}
	}
}


fn provide_score(
	trigger: Trigger<RequestScore>,
	mut commands: Commands,
	stat_map: Res<StatMap>,
	children: Query<&Children>,
	stats: Query<(&StatId, &StatValue)>,
	query: Query<(&StatScoreProvider, &Parent, &TargetEntity)>,
) {
	let (score_provider, parent, target_entity) = query
		.get(trigger.entity())
		.expect(expect_action::ACTION_QUERY_MISSING);

	let value = StatValue::find_by_id(
		**target_entity,
		children,
		stats,
		score_provider.stat_id,
	)
	.expect(expect_action::TARGET_MISSING);

	let descriptor = stat_map.get(&score_provider.stat_id).unwrap();
	let score = score_provider.sample(value, descriptor.global_range.clone());

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
		let provider = StatScoreProvider::new(StatId::default());
		let range = StatValue::range(-3.0..7.0);

		expect(provider.sample(StatValue(-3.), range.clone())).to_be(0.0)?;
		expect(provider.sample(StatValue(2.0), range.clone())).to_be(0.5)?;
		expect(provider.sample(StatValue(7.0), range.clone())).to_be(1.0)?;

		let provider = provider.in_negative_direction();
		expect(provider.sample(StatValue(-3.), range.clone())).to_be(1.0)?;
		expect(provider.sample(StatValue(2.0), range.clone())).to_be(0.5)?;
		expect(provider.sample(StatValue(7.0), range.clone())).to_be(0.0)?;

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

		let on_child_score =
			observe_trigger_mapped(world, |trigger: Trigger<OnChildScore>| {
				*trigger.event().value()
			});

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
					StatScoreProvider::new(StatMap::TEST_PLEASENTNESS_ID),
				));
				parent.spawn((
					TargetEntity(agent),
					StatScoreProvider::new(StatMap::TEST_PLEASENTNESS_ID)
						.in_negative_direction(),
				));
			})
			.flush_trigger(OnRun);

		expect(&on_child_score).to_have_been_called_times(2)?;

		expect(&on_child_score).to_have_returned_nth_with(0, &0.7)?;
		expect(&on_child_score).to_have_returned_nth_with(1, &0.3)?;

		Ok(())
	}
}
