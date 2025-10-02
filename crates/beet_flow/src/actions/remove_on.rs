use crate::prelude::*;
use beet_core::prelude::*;
use std::marker::PhantomData;


/// This action will remove the specified bundle when the specified action is triggered.
/// It is designed to work for both [`Run`] and [`End`] events.
/// This action also has a corresponding [`InsertOn`] action.
/// ## Example
/// Removes the `Running` bundle when the `OnResult` event is triggered.
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// World::new()
///		.spawn((
/// 		Running,
/// 		EndOnRun(SUCCESS),
/// 		RemoveOn::<End, Running>::default()
/// 	))
///		.trigger_entity(RUN);
/// ```
#[action(remove::<E , B>)]
#[derive(Debug, Component, Reflect)]
#[reflect(Component)]
pub struct RemoveOn<E: EntityEvent, B: Bundle> {
	/// The target entity to remove the bundle from.
	pub target_entity: TargetEntity,
	phantom: PhantomData<(E, B)>,
}

impl<E: EntityEvent, B: Bundle> Default for RemoveOn<E, B> {
	fn default() -> Self {
		Self {
			phantom: default(),
			target_entity: default(),
		}
	}
}

impl<E: EntityEvent, B: Bundle> RemoveOn<E, B> {
	/// Specify the target entity for this action.
	pub fn new_with_target(target_entity: TargetEntity) -> Self {
		Self {
			target_entity,
			..default()
		}
	}
}

fn remove<E: EntityEvent, B: Bundle>(
	ev: On<E>,
	mut commands: Commands,
	query: Query<&RemoveOn<E, B>>,
) -> Result {
	let action = query.get(ev.event_target())?;
	let target = action.target_entity.get_target(&ev);
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
			.spawn((Running::default(), RemoveOn::<Run, Running>::default()))
			.trigger_entity(RUN)
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
				RemoveOn::<End, Running>::default(),
				EndOnRun(SUCCESS),
			))
			.trigger_entity(RUN)
			.flush()
			.id();
		world.get::<Running>(entity).xpect_none();
	}
}
