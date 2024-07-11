use crate::prelude::*;
use bevy::prelude::*;

pub type ContinueRun = InsertWhileRunning<Running>;

/// 1. Adds the provided component when [`OnRun`] is called
/// 2. Removes the component when [`OnRunResult`] is called
#[derive(Bundle, Reflect)]
pub struct InsertWhileRunning<T: Default + GenericActionComponent> {
	add: InsertOnTrigger<OnRun, T>,
	remove: RemoveOnTrigger<OnRunResult, T>,
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
