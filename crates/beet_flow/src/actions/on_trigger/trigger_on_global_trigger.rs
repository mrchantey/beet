use crate::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;

/// Trigger the given event on a global trigger.
pub type TriggerOnGlobalTrigger<Params, TriggerEvent, TriggerBundle = ()> =
	OnTrigger<
		TriggerOnGlobalTriggerHandler<Params, TriggerEvent, TriggerBundle>,
	>;


#[derive(Reflect)]
pub struct TriggerOnGlobalTriggerHandler<Params, TriggerEvent, TriggerBundle>(
	#[reflect(ignore)] PhantomData<(Params, TriggerEvent, TriggerBundle)>,
);
impl<
		Params: Reflect + Event + Default + Clone,
		TriggerEvent: Event,
		TriggerBundle: Bundle,
	> OnTriggerHandler
	for TriggerOnGlobalTriggerHandler<Params, TriggerEvent, TriggerBundle>
{
	type Params = Params;
	type TriggerEvent = TriggerEvent;
	type TriggerBundle = TriggerBundle;

	fn default_source() -> ActionTarget { ActionTarget::Global }

	fn handle(
		commands: &mut Commands,
		_ev: &Trigger<Self::TriggerEvent, Self::TriggerBundle>,
		query: (Entity, &OnTrigger<Self>),
	) {
		commands.entity(query.0).trigger(query.1.params.clone());
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
		app.add_plugins(ActionPlugin::<
			TriggerOnGlobalTrigger<OnRunResult, OnRun>,
		>::default());
		let world = app.world_mut();
		let func = observe_triggers::<OnRunResult>(world);

		world.spawn(TriggerOnGlobalTrigger::<OnRunResult, OnRun>::new(
			OnRunResult::failure(),
		));
		world.flush();
		// global trigger
		world.trigger(OnRun);
		world.flush();
		expect(&func).to_have_returned_nth_with(0, &OnRunResult::failure());
	}
}
