use crate::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;



/// Inserts the provided `Bundle` on the [`TriggerOnTrigger::target`] when
/// the `EventIn` is triggered on one of the [`TriggerOnTrigger::sources`].
pub type InsertOnTrigger<Params, TriggerEvent, TriggerBundle = ()> =
	OnTrigger<InsertHandler<Params, TriggerEvent, TriggerBundle>>;

#[derive(Reflect)]
pub struct InsertHandler<Params, TriggerEvent, TriggerBundle>(
	#[reflect(ignore)] PhantomData<(Params, TriggerEvent, TriggerBundle)>,
);


impl<
		Params: Default + Bundle + Reflect + Clone,
		TriggerEvent: Event,
		TriggerBundle: Bundle,
	> OnTriggerHandler for InsertHandler<Params, TriggerEvent, TriggerBundle>
{
	type TriggerEvent = TriggerEvent;
	type TriggerBundle = TriggerBundle;
	type Params = Params;
	fn handle(
		commands: &mut Commands,
		_trigger: &Trigger<Self::TriggerEvent, Self::TriggerBundle>,
		(entity, action): (Entity, &OnTrigger<Self>),
	) {
		action
			.target
			.insert(commands, entity, action.params.clone());
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins(ActionPlugin::<InsertOnRun<Running>>::default());
		let world = app.world_mut();

		let entity = world
			.spawn(InsertOnRun::<Running>::default())
			.flush_trigger(OnRun)
			.id();
		// each action component type spawns a global observer (that's the +1)
		expect(world.entities().len()).to_be(2 + 1);
		expect(&*world).to_have_component::<Running>(entity);
	}

	#[test]
	fn with_map() {
		let mut app = App::new();
		app.add_plugins(
			ActionPlugin::<InsertOnTrigger<Running, OnRun>>::default(),
		);
		let world = app.world_mut();

		let entity = world
			.spawn(InsertOnTrigger::<Running, OnRun>::default())
			.flush_trigger(OnRun)
			.id();

		// each action component type spawns a global observer (that's the +1)
		expect(world.entities().len()).to_be(2 + 1);
		expect(world.get::<Running>(entity)).to_be_some();
	}
}
