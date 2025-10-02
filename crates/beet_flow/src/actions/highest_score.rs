use crate::prelude::*;
use beet_core::prelude::*;
use bevy::platform::collections::HashMap;
use std::cmp::Ordering;

/// Wrapper for an f32, representing a score. This should be between 0 and 1.
///	## Example
/// ```rust
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// let mut world = World::new();
/// // create a passing score value
/// world.spawn(EndOnRun::<RequestScore, _>::new(ScoreValue(1.)));
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

impl IntoEntityEvent for ScoreValue {
	type Event = End<ScoreValue>;
	fn into_entity_event(self, entity: Entity) -> Self::Event {
		End::new(entity, self)
	}
}


/// The payload for requesting a score,
/// for usage see [`HighestScore`].
#[derive(
	Debug, Default, Copy, Clone, PartialEq, PartialOrd, Component, Reflect,
)]
pub struct RequestScore;

impl IntoEntityEvent for RequestScore {
	type Event = Run<RequestScore>;
	fn into_entity_event(self, entity: Entity) -> Self::Event {
		Run::new(entity, self)
	}
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
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// let mut world = World::new();
/// world
///		.spawn(HighestScore::default())
///		.with_child((
///			EndOnRun::<RequestScore, _>::new(ScoreValue::NEUTRAL),
///			EndOnRun::success(),
///		))
///		.with_child((
///			EndOnRun::<RequestScore, _>::new(ScoreValue::PASS),
///			EndOnRun::success(),
///		))
///		.trigger_entity(RUN);
/// ```
#[action(on_start, on_receive_score)]
#[derive(Default, Deref, DerefMut, Component, Reflect)]
#[reflect(Default, Component)]
#[require(PreventPropagateEnd<ScoreValue>)]
// TODO sparseset instead of hashmap
pub struct HighestScore(HashMap<Entity, ScoreValue>);

fn on_start(
	ev: On<Run>,
	mut commands: Commands,
	mut query: Query<(&mut HighestScore, &Children)>,
) -> Result {
	let (mut action, children) = query.get_mut(ev.event_target())?;
	action.clear();

	for child in children.iter() {
		commands.entity(child).trigger_entity(RequestScore);
	}
	Ok(())
}

fn on_receive_score(
	ev: On<ChildEnd<ScoreValue>>,
	mut commands: Commands,
	mut query: Query<(&mut HighestScore, &Children)>,
) -> Result {
	let (mut action, children) = query.get_mut(ev.event_target())?;
	action.insert(ev.child(), ev.value().clone());

	// all children have reported their score, run the highest scoring child
	if action.len() == children.iter().len() {
		let (highest, _) = action
			.iter()
			.max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(Ordering::Equal))
			.ok_or_else(|| expect_action::to_have_children(&ev))?;
		commands.entity(*highest).trigger_entity(RUN);
	}
	Ok(())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut world = BeetFlowPlugin::world();

		let on_run = collect_on_run(&mut world);
		let on_result = collect_on_result(&mut world);
		let on_request_score =
			observer_ext::observe_triggers::<Run<RequestScore>>(&mut world);
		let on_score =
			observer_ext::observe_triggers::<End<ScoreValue>>(&mut world);

		world
			.spawn((Name::new("root"), HighestScore::default()))
			.with_child((
				Name::new("child1"),
				EndOnRun::<RequestScore, _>::new(ScoreValue::NEUTRAL),
				EndOnRun::success(),
			))
			.with_child((
				Name::new("child2"),
				EndOnRun::<RequestScore, _>::new(ScoreValue::PASS),
				EndOnRun::success(),
			))
			.trigger_entity(RUN)
			.flush();
		on_request_score.len().xpect_eq(2);
		on_score.len().xpect_eq(4);

		#[rustfmt::skip]
		on_run.get().xpect_eq(vec![
			"root".to_string(),
			"child2".to_string()
		]);
		on_result.get().xpect_eq(vec![
			("child2".to_string(), SUCCESS),
			("root".to_string(), SUCCESS),
		]);
	}
}
