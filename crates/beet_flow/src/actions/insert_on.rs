use crate::prelude::*;
use beet_core::prelude::*;



/// This action will insert the provided bundle when the specified action is triggered.
/// It is designed to work for both [`Run`] and [`End`] events.
/// This action also has a corresponding [`RemoveOn`] action.
/// ## Example
/// Inserts the `Running` bundle when the `OnRun` event is triggered.
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// World::new()
///		.spawn(InsertOn::<Run, Running>::default())
///		.trigger(RUN);
/// ```
#[action(insert::<E , B>)]
#[derive(Debug, Component, Reflect)]
#[reflect(Component)]
pub struct InsertOn<E: EntityEvent, B: Bundle + Clone> {
	/// The bundle to be cloned and inserted.
	pub bundle: B,
	/// The target entity to insert the bundle into.
	pub target_entity: TargetEntity,
	phantom: PhantomData<E>,
}

impl<E: EntityEvent, B: Bundle + Clone> InsertOn<E, B> {
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

impl<E: EntityEvent, B: Bundle + Clone + Default> Default for InsertOn<E, B> {
	fn default() -> Self {
		Self {
			bundle: default(),
			phantom: default(),
			target_entity: default(),
		}
	}
}

fn insert<E: EntityEvent, B: Bundle + Clone>(
	ev: On<E>,
	mut commands: Commands,
	query: Query<&InsertOn<E, B>>,
) -> Result {
	let action = query.get(ev.event_target())?;
	let target = action.target_entity.get_target(&ev);
	commands.entity(target).insert(action.bundle.clone());
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
		let world = app.world_mut();

		let entity = world
			.spawn(InsertOn::<Run, Running>::default())
			.trigger_entity(RUN)
			.flush()
			.id();
		world.get::<Running>(entity).xpect_some();
	}
	#[test]
	fn on_result() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());
		let world = app.world_mut();

		let entity = world
			.spawn((InsertOn::<End, Running>::default(), EndOnRun::success()))
			.trigger_entity(RUN)
			.flush()
			.id();
		world.get::<Running>(entity).xpect_some();
	}
}
