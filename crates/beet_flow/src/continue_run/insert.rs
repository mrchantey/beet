use crate::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;



/// This action will insert the provided bundle when the specified action is triggered.
/// It is designed to work for both [`OnRun`] and [`OnResult`] events.
/// This action also has a corresponding [`Remove`] action.
/// ## Example
/// Inserts the `Running` bundle when the `OnRun` event is triggered.
/// ```
/// # use beet_flow::doctest::*;
/// # let mut world = world();
/// world
///		.spawn(Insert::<OnRun, Running>::default())
///		.trigger(OnRun::local());
/// ```
#[action(insert::<E , B>)]
#[derive(Debug, Component, Reflect)]
#[reflect(Component)]
pub struct Insert<E: ObserverEvent, B: Bundle + Clone> {
	/// The bundle to be cloned and inserted.
	pub bundle: B,
	/// The target entity to insert the bundle into.
	pub target_entity: TargetEntity,
	phantom: PhantomData<E>,
}

impl<E: ObserverEvent, B: Bundle + Clone> Insert<E, B> {
	/// Specify the bundle to be inserted
	pub fn new(bundle: B) -> Self {
		Self {
			bundle,
			phantom: PhantomData,
			target_entity: TargetEntity::default(),
		}
	}
	/// Specify the bundle to be inserted and the target entity.
	pub fn new_with_target(bundle: B, target_entity: TargetEntity) -> Self {
		Self {
			bundle,
			phantom: PhantomData,
			target_entity,
		}
	}
}

impl<E: ObserverEvent, B: Bundle + Clone + Default> Default for Insert<E, B> {
	fn default() -> Self {
		Self {
			bundle: B::default(),
			phantom: PhantomData,
			target_entity: TargetEntity::default(),
		}
	}
}

fn insert<E: ObserverEvent, B: Bundle + Clone>(
	ev: Trigger<E>,
	mut commands: Commands,
	query: Query<&Insert<E, B>>,
) {
	let action = query
		.get(ev.action())
		.expect(&expect_action::to_have_action(&ev));
	let target = action.target_entity.get_target(&*ev);
	commands.entity(target).insert(action.bundle.clone());
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
			.spawn(Insert::<OnRun, Running>::default())
			.flush_trigger(OnRun::local())
			.id();
		world.get::<Running>(entity).xpect_some();
	}
	#[test]
	fn on_result() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());
		let world = app.world_mut();

		let entity = world
			.spawn((
				Insert::<OnResult, Running>::default(),
				ReturnWith(RunResult::Success),
			))
			.flush_trigger(OnRun::local())
			.id();
		world.get::<Running>(entity).xpect_some();
	}
}
