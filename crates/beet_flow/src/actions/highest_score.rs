use crate::prelude::*;
use beet_core::prelude::*;
use bevy::platform::collections::HashMap;
use std::cmp::Ordering;

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
///			EndOnRun(Score::NEUTRAL),
///			EndOnRun(SUCCESS),
///		))
///		.with_child((
///			EndOnRun(Score::PASS),
///			EndOnRun(SUCCESS),
///		))
///		.trigger_payload(RUN);
/// ```
#[action(on_start, on_receive_score)]
#[derive(Default, Deref, DerefMut, Component, Reflect)]
#[reflect(Default, Component)]
#[require(PreventPropagateEnd<Score>)]
// TODO sparseset instead of hashmap
pub struct HighestScore(HashMap<Entity, Score>);

fn on_start(
	ev: On<Run>,
	mut commands: Commands,
	mut query: Query<(&mut HighestScore, &Children)>,
) -> Result {
	let (mut action, children) = query.get_mut(ev.event_target())?;
	action.clear();

	for child in children.iter() {
		commands.entity(child).trigger_payload(GetScore);
	}
	Ok(())
}

fn on_receive_score(
	ev: On<ChildEnd<Score>>,
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
		commands.entity(*highest).trigger_payload(RUN);
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
			observer_ext::observe_triggers::<Run<GetScore>>(&mut world);
		let on_score =
			observer_ext::observe_triggers::<End<Score>>(&mut world);

		world
			.spawn((Name::new("root"), HighestScore::default()))
			.with_child((
				Name::new("child1"),
				EndOnRun(Score::NEUTRAL),
				EndOnRun(SUCCESS),
			))
			.with_child((
				Name::new("child2"),
				EndOnRun(Score::PASS),
				EndOnRun(SUCCESS),
			))
			.trigger_payload(RUN)
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
