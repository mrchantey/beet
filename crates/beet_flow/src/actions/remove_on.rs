use crate::prelude::*;
use beet_core::prelude::*;
use std::marker::PhantomData;


/// This action will remove the specified bundle when the specified action is triggered.
/// It is designed to work for both [`Run`] and [`End`] events.
/// This action also has a corresponding [`InsertOn`] action.
/// ## Example
/// Removes the `Running` bundle when the `Outcome` event is triggered.
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// World::new()
///		.spawn((
/// 		Running,
/// 		EndWith(Outcome::Pass),
/// 		RemoveOn::<Outcome, Running>::default()
/// 	))
///		.trigger_target(GetOutcome);
/// ```
#[action(remove::<E , B>)]
#[derive(Debug, Component, Reflect)]
#[reflect(Component)]
pub struct RemoveOn<E: ActionEvent, B: Bundle> {
	/// The target entity to remove the bundle from.
	pub target_entity: TargetEntity,
	phantom: PhantomData<(E, B)>,
}

impl<E: ActionEvent, B: Bundle> Default for RemoveOn<E, B> {
	fn default() -> Self {
		Self {
			phantom: default(),
			target_entity: default(),
		}
	}
}

impl<E: ActionEvent, B: Bundle> RemoveOn<E, B> {
	/// Specify the target entity for this action.
	pub fn new_with_target(target_entity: TargetEntity) -> Self {
		Self {
			target_entity,
			..default()
		}
	}
}

fn remove<E: ActionEvent, B: Bundle>(
	ev: On<E>,
	mut commands: Commands,
	query: Query<&RemoveOn<E, B>>,
	agents: GlobalAgentQuery,
) -> Result {
	let action = query.get(ev.event_target())?;
	let target = action.target_entity.select_target(&ev, &agents);
	commands.entity(target).remove::<B>();
	Ok(())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn on_run() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());
		let world = app.world_mut();

		let entity = world
			.spawn((
				Running::default(),
				RemoveOn::<GetOutcome, Running>::default(),
			))
			.trigger_target(GetOutcome)
			.flush()
			.id();
		world.get::<Running>(entity).xpect_none();
	}
	#[test]
	fn on_result() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());
		let world = app.world_mut();

		let entity = world
			.spawn((
				Running::default(),
				RemoveOn::<Outcome, Running>::default(),
				EndWith(Outcome::Pass),
			))
			.trigger_target(GetOutcome)
			.flush()
			.id();
		world.get::<Running>(entity).xpect_none();
	}
}
