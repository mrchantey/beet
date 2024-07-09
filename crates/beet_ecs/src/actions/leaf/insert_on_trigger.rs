use crate::prelude::*;
use bevy::prelude::*;
use leaf::component_target::ComponentTarget;
use std::marker::PhantomData;


/// Adds the provided component when [`<E>`] is triggered
#[derive(Action, Reflect)]
#[reflect(Default, Component)]
#[observers(on_trigger::<E,T>)]
pub struct InsertOnTrigger<
	E: GenericActionEvent,
	T: Default + GenericActionComponent,
> {
	pub comp: T,
	pub target: ComponentTarget,
	#[reflect(ignore)]
	phantom: PhantomData<E>,
}

impl<E: GenericActionEvent, T: Default + GenericActionComponent> Default
	for InsertOnTrigger<E, T>
{
	fn default() -> Self { Self::new(T::default()) }
}

impl<E: GenericActionEvent, T: Default + GenericActionComponent>
	InsertOnTrigger<E, T>
{
	pub fn new(comp: T) -> Self {
		Self {
			comp,
			target: default(),
			phantom: PhantomData,
		}
	}
	pub fn with_target(self, target: impl Into<ComponentTarget>) -> Self {
		Self {
			target: target.into(),
			..self
		}
	}
}

fn on_trigger<E: GenericActionEvent, T: Default + GenericActionComponent>(
	trigger: Trigger<E>,
	query: Query<&InsertOnTrigger<E, T>>,
	mut commands: Commands,
) {
	let action = query
		.get(trigger.entity())
		.expect(expect_action::NO_ACTION_COMP);
	action
		.target
		.insert(&mut commands, trigger.entity(), action.comp.clone());
}

#[cfg(test)]
mod test {
	use super::InsertOnTrigger;
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let mut world = World::new();

		let entity = world
			.spawn(InsertOnTrigger::<OnRun, Running>::default())
			.flush_trigger(OnRun)
			.id();
		expect(world.entities().len()).to_be(2)?;
		expect(&world).to_have_component::<Running>(entity)?;
		Ok(())
	}
}
