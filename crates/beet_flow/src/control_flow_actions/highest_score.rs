use crate::prelude::*;
use bevy::prelude::*;
use bevy::utils::HashMap;
use std::cmp::Ordering;


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
pub struct ScoreValue(pub f32);

impl ScoreValue {
	pub const PASS: Self = Self(1.0);
	pub const NEUTRAL: Self = Self(0.5);
	pub const FAIL: Self = Self(0.0);
}

#[derive(
	Debug, Default, Copy, Clone, PartialEq, PartialOrd, Component, Reflect,
)]
pub struct RequestScore;

impl RunPayload for RequestScore {
	type Result = ScoreValue;
}
impl ResultPayload for ScoreValue {
	type Run = RequestScore;
}

/// The score flow is a utility ai selector.
/// Children should provide a score on request, see [`ScoreProvider`].
///
#[action(on_start, on_receive_score)]
#[derive(Default, Deref, DerefMut, Component, Reflect)]
#[reflect(Default, Component)]
#[require(BubbleResult)]
// TODO SparseSet
pub struct HighestScore(HashMap<Entity, ScoreValue>);

fn on_start(
	ev: Trigger<OnRun>,
	mut commands: Commands,
	mut query: Query<(&mut HighestScore, &Children)>,
) {
	let (mut action, children) = query
		.get_mut(ev.action)
		.expect(&expect_action::to_have_action(&ev));

	action.clear();

	for child in children.iter() {
		commands
			.entity(*child)
			.trigger(OnRunAction::local(RequestScore));
	}
}

fn on_receive_score(
	ev: Trigger<OnChildResult<ScoreValue>>,
	mut commands: Commands,
	mut query: Query<(&mut HighestScore, &Children)>,
) {
	let (mut action, children) = query
		.get_mut(ev.action)
		.expect(&expect_action::to_have_action(&ev));

	action.insert(ev.child, ev.payload);

	if action.len() == children.iter().len() {
		let (highest, _) = action
			.iter()
			.max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(Ordering::Equal))
			.expect(&expect_action::to_have_children(&ev));
		commands.entity(*highest).trigger(OnRun::local());
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());
		let world = app.world_mut();

		let on_run = collect_on_run(world);
		let on_result = collect_on_result(world);
		let on_request_score = observe_triggers::<OnRun<RequestScore>>(world);
		let on_score = observe_triggers::<OnResultAction<ScoreValue>>(world);

		world
			.spawn((Name::new("root"), HighestScore::default()))
			.with_child((
				Name::new("child1"),
				ReturnWith(ScoreValue::NEUTRAL),
				ReturnWith(RunResult::Success),
			))
			.with_child((
				Name::new("child2"),
				ReturnWith(ScoreValue::PASS),
				ReturnWith(RunResult::Success),
			))
			.flush_trigger(OnRun::local());
		expect(&on_request_score).to_have_been_called_times(4);
		expect(&on_score).to_have_been_called_times(2);

		#[rustfmt::skip]
		expect(on_run()).to_be(vec![
			"root".to_string(), 
			"child2".to_string()
		]);
		expect(on_result()).to_be(vec![
			("child2".to_string(), RunResult::Success),
			("root".to_string(), RunResult::Success),
		]);
	}
}
