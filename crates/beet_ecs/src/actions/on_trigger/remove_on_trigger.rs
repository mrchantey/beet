use crate::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;


pub type RemoveOnTrigger<Event, Params, TriggerBundle = ()> =
	OnTrigger<RemoveHandler<Event, Params, TriggerBundle>>;

#[derive(Reflect)]
pub struct RemoveHandler<E, T, B = ()>(
	#[reflect(ignore)] PhantomData<(E, T, B)>,
);


impl<E: Event, T: Bundle, TrigBundle: Bundle> OnTriggerHandler
	for RemoveHandler<E, T, TrigBundle>
{
	type Event = E;
	type TriggerBundle = TrigBundle;
	type Params = ();
	fn handle(
		commands: &mut Commands,
		_trigger: &Trigger<Self::Event, Self::TriggerBundle>,
		(entity, comp): (Entity, &OnTrigger<Self>),
	) {
		comp.target.remove::<T>(commands, entity);
	}
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
		let mut app = App::new();
		app.add_plugins(
			ActionPlugin::<RemoveOnTrigger<OnRun, Running>>::default(),
		);
		let world = app.world_mut();

		let entity = world
			.spawn((Running, RemoveOnTrigger::<OnRun, Running>::default()))
			.flush_trigger(OnRun)
			.id();
		expect(world.entities().len()).to_be(2)?;
		expect(&*world).not().to_have_component::<Running>(entity)?;
		Ok(())
	}
}
