use crate::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;


pub type RemoveOnTrigger<Event, Params> =
	OnTrigger<RemoveHandler<Event, Params>>;

#[derive(Reflect)]
pub struct RemoveHandler<E,T>(#[reflect(ignore)] PhantomData<(E,T)>);


impl<E:Event,T:Bundle> OnTriggerHandler for RemoveHandler<E,T>
{
	type Event = E;
	type Params = ();
	fn handle(
		commands: &mut Commands,
		trigger: &Trigger<Self::Event>,
		comp: &OnTrigger<Self>,
	) {
		comp.target.remove::<T>(commands, trigger.entity());
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
