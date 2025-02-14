use crate::prelude::*;
use bevy::prelude::*;
use bevy::utils::HashMap;
use std::cmp::Ordering;

/// Wrapper for an f32, representing a score. This should be between 0 and 1.
///	## Example
/// ```rust
/// # use beet_flow::doctest::*;
/// # let mut world = world();
/// // create a passing score value
/// world.spawn(ReturnWith(ScoreValue(1.)));
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
pub struct ScoreValue(pub f32);

impl ScoreValue {
	/// Its best practice to keep scores between 0 and 1,
	/// so a passing score is 1
	pub const PASS: Self = Self(1.0);
	/// Its best practice to keep scores between 0 and 1,
	/// so a neutral score is 0.5
	pub const NEUTRAL: Self = Self(0.5);
	/// Its best practice to keep scores between 0 and 1,
	/// so a failing score is 0
	pub const FAIL: Self = Self(0.0);
	/// Create a new instance of `ScoreValue` with the provided score.
	pub fn new(score: f32) -> Self { Self(score) }
}


/// The payload for requesting a score,
/// for usage see [`HighestScore`].
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
/// Aka `UtilitySelector`, Runs the child with the highest score.
/// This action uses the principles of Utility AI.
/// The mechanisim for requesting and returning a score is the same
/// as that for requesting and returning a result, which is why
/// we are able to use [`ReturnWith`] for each case.
/// ## Tags
/// - [ControlFlow](ActionTag::ControlFlow)
///
/// ## Example
/// ```rust
/// # use beet_flow::doctest::*;
/// # let mut world = world();
/// world
///		.spawn(HighestScore::default())
///		.with_child((
///			ReturnWith(ScoreValue::NEUTRAL),
///			ReturnWith(RunResult::Success),
///		))
///		.with_child((
///			ReturnWith(ScoreValue::PASS),
///			ReturnWith(RunResult::Success),
///		))
///		.trigger(OnRun::local());
/// ```
#[action(on_start, on_receive_score)]
#[derive(Default, Deref, DerefMut, Component, Reflect)]
#[reflect(Default, Component)]
#[require(BubbleResult)]
// TODO sparseset instead of hashmap
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
		.get_mut(ev.parent)
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
