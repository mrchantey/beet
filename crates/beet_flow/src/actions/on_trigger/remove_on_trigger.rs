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
	type TriggerEvent = E;
	type TriggerBundle = TrigBundle;
	type Params = ();
	fn handle(
		commands: &mut Commands,
		_trigger: &Trigger<Self::TriggerEvent, Self::TriggerBundle>,
		(entity, comp): (Entity, &OnTrigger<Self>),
	) {
		// log::info!("RemoveOnTrigger: {:?}", std::any::type_name::<T>());
		comp.target.remove::<T>(commands, entity);
	}
}

#[cfg(test)]
mod test {
	use super::RemoveOnTrigger;
	use crate::prelude::*;
	use sweet::prelude::*;
	use bevy::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins(
			ActionPlugin::<RemoveOnTrigger<OnRun, Running>>::default(),
		);
		let world = app.world_mut();

		let entity = world
			.spawn((Running, RemoveOnTrigger::<OnRun, Running>::default()))
			.flush_trigger(OnRun)
			.id();
		expect(world.entities().len()).to_be(2);
		expect(&*world).not().to_have_component::<Running>(entity);
	}
}
