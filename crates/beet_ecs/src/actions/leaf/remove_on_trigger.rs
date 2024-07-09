use crate::prelude::*;
use bevy::prelude::*;
use leaf::component_target::ComponentTarget;
use std::marker::PhantomData;


/// Removes the provided component when [`<E>`] is triggered
#[derive(Action, Reflect)]
#[reflect(Default, Component)]
#[observers(on_trigger::<E,T>)]
pub struct RemoveOnTrigger<
	E: GenericActionEvent,
	T: Default + GenericActionComponent,
> {
	pub target: ComponentTarget,
	#[reflect(ignore)]
	phantom: PhantomData<(E, T)>,
}

impl<E: GenericActionEvent, T: Default + GenericActionComponent> Default
	for RemoveOnTrigger<E, T>
{
	fn default() -> Self { Self::new() }
}

impl<E: GenericActionEvent, T: Default + GenericActionComponent>
	RemoveOnTrigger<E, T>
{
	pub fn new() -> Self {
		Self {
			target: default(),
			phantom: default(),
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
	query: Query<&RemoveOnTrigger<E, T>>,
	mut commands: Commands,
) {
	let action = query
		.get(trigger.entity())
		.expect(expect_action::NO_ACTION_COMP);
	action.target.remove::<T>(&mut commands, trigger.entity());
}

#[cfg(test)]
mod test {
	use super::RemoveOnTrigger;
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let mut world = World::new();

		let entity = world
			.spawn((Running, RemoveOnTrigger::<OnRun, Running>::default()))
			.flush_trigger(OnRun)
			.id();
		expect(world.entities().len()).to_be(2)?;
		expect(&world).not().to_have_component::<Running>(entity)?;
		Ok(())
	}
}
