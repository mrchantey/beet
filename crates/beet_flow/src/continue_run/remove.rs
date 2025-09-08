use crate::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;


/// This action will remove the specified bundle when the specified action is triggered.
/// It is designed to work for both [`OnRun`] and [`OnResult`] events.
/// This action also has a corresponding [`Insert`] action.
/// ## Example
/// Removes the `Running` bundle when the `OnResult` event is triggered.
/// ```
/// # use beet_flow::doctest::*;
/// # let mut world = world();
/// world
///		.spawn((
/// 		ReturnWith(RunResult::Success),
/// 		Remove::<OnResult, Running>::default()
/// 	))
///		.trigger(OnRun::local());
/// ```
#[action(remove::<E , B>)]
#[derive(Debug, Component, Reflect)]
#[reflect(Component)]
pub struct Remove<E: ObserverEvent, B: Bundle> {
	/// The target entity to remove the bundle from.
	pub target_entity: TargetEntity,
	phantom: PhantomData<(E, B)>,
}

impl<E: ObserverEvent, B: Bundle> Default for Remove<E, B> {
	fn default() -> Self {
		Self {
			phantom: PhantomData,
			target_entity: TargetEntity::default(),
		}
	}
}

impl<E: ObserverEvent, B: Bundle> Remove<E, B> {
	/// Specify the target entity for this action.
	pub fn new_with_target(target_entity: TargetEntity) -> Self {
		Self {
			target_entity,
			..default()
		}
	}
}

fn remove<E: ObserverEvent, B: Bundle>(
	ev: Trigger<E>,
	mut commands: Commands,
	query: Query<&Remove<E, B>>,
) {
	let action = query
		.get(ev.action())
		.expect(&expect_action::to_have_action(&ev));
	let target = action.target_entity.get_target(&*ev);
	commands.entity(target).remove::<B>();
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn on_run() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());
		let world = app.world_mut();

		let entity = world
			.spawn((Running::default(), Remove::<OnRun, Running>::default()))
			.flush_trigger(OnRun::local())
			.id();
		world.get::<Running>(entity).xpect().to_be_none();
	}
	#[test]
	fn on_result() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());
		let world = app.world_mut();

		let entity = world
			.spawn((
				Running::default(),
				Remove::<OnResult, Running>::default(),
				ReturnWith(RunResult::Success),
			))
			.flush_trigger(OnRun::local())
			.id();
		world.get::<Running>(entity).xpect().to_be_none();
	}
}
