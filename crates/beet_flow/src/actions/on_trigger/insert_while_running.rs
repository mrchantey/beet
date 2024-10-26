use crate::prelude::*;
use bevy::ecs::component::ComponentHooks;
use bevy::ecs::component::StorageType;
use bevy::prelude::*;


/// This will add the [`Running`] component to the behavior when [`OnRun`] is triggered.
/// It will be removed automatically when [`OnRunResult`] is triggered.
///
/// **Note**: This should usually be added as a required component
/// on continuous actions, not added to behaviors directly, because its easy to forget.
pub type ContinueRun = InsertWhileRunning<Running>;

/// 1. Adds the provided component when [`OnRun`] is triggered
/// 2. Removes the component when [`OnRunResult`] is triggered
// #[derive(Bundle,Reflect)]
#[derive(Reflect)]
pub struct InsertWhileRunning<T: Default + GenericActionComponent> {
	add: InsertOnTrigger<OnRun, T>,
	remove: RemoveOnTrigger<OnRunResult, T>,
}

impl<T: Default + GenericActionComponent> Component for InsertWhileRunning<T> {
	const STORAGE_TYPE: StorageType = StorageType::Table;

	fn register_component_hooks(hooks: &mut ComponentHooks) {
		// Whenever this component is removed, or an entity with
		// this component is despawned...
		hooks.on_add(|mut world, entity, _| {
			let action = world.get::<InsertWhileRunning<T>>(entity).unwrap();
			let add = action.add.clone();
			let remove = action.remove.clone();
			world
				.commands()
				.entity(entity)
				.insert((add, remove))
				.remove::<InsertWhileRunning<T>>();
		});
	}
}


impl<T: Default + GenericActionComponent> Default for InsertWhileRunning<T> {
	fn default() -> Self {
		Self {
			add: default(),
			remove: default(),
		}
	}
}
impl<T: Default + GenericActionComponent> InsertWhileRunning<T> {
	pub fn new(comp: T) -> Self {
		Self {
			add: InsertMappedOnTrigger::new(comp),
			remove: default(),
		}
	}
	pub fn with_target(self, target: impl Into<TriggerTarget>) -> Self {
		let target: TriggerTarget = target.into();
		Self {
			add: self.add.with_target(target.clone()),
			remove: self.remove.with_target(target),
			..self
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let mut app = App::new();
		app.add_plugins(ActionPlugin::<(
			InsertOnTrigger<OnRun, Running>,
			RemoveOnTrigger<OnRunResult, Running>,
		)>::default());
		let world = app.world_mut();

		let entity = world
			.spawn(ContinueRun::default())
			.flush_trigger(OnRun)
			.id();
		expect(world.entities().len()).to_be(3)?;
		expect(&*world).to_have_component::<Running>(entity)?;
		world
			.entity_mut(entity)
			.flush_trigger(OnRunResult::success());
		expect(&*world).not().to_have_component::<Running>(entity)?;
		Ok(())
	}
}
