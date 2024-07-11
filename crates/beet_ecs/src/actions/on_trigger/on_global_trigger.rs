use crate::prelude::*;
use bevy::prelude::*;


/// Insert given component on a global trigger.
pub type InsertOnGlobalTrigger<Event, Params, TriggerBundle = ()> =
	OnGlobalTrigger<
		InsertHandler<DefaultMapFunc<Event, Params, TriggerBundle>>,
	>;
/// Map to an insert on global trigger.
pub type InsertMappedOnGlobalTrigger<M> = OnGlobalTrigger<InsertHandler<M>>;

/// Remove given component on a global trigger.
pub type RemoveOnGlobalTrigger<Event, Params, TriggerBundle = ()> =
	OnGlobalTrigger<RemoveHandler<Event, Params, TriggerBundle>>;

/// Trigger the given event on a global trigger.
pub type TriggerOnGlobalTrigger<Event, Params, TriggerBundle = ()> =
	OnGlobalTrigger<
		TriggerHandler<DefaultMapFunc<Event, Params, TriggerBundle>>,
	>;

/// Map to a trigger event on global trigger.
pub type TriggerMappedOnGlobalTrigger<M> = OnGlobalTrigger<TriggerHandler<M>>;

#[derive(Component, Action, Reflect)]
#[reflect(Default, Component)]
#[global_observers(on_trigger::<Handler>)]
pub struct OnGlobalTrigger<Handler: OnTriggerHandler>(pub OnTrigger<Handler>);


impl<Handler: OnTriggerHandler> Default for OnGlobalTrigger<Handler>
where
	Handler::Params: Default,
{
	fn default() -> Self { Self::new(default()) }
}

impl<Handler: OnTriggerHandler> OnGlobalTrigger<Handler> {
	pub fn new(params: Handler::Params) -> Self { Self(OnTrigger::new(params)) }
	pub fn with_target(self, target: impl Into<TriggerTarget>) -> Self {
		Self(self.0.with_target(target))
	}
}

fn on_trigger<Handler: OnTriggerHandler>(
	trigger: Trigger<Handler::Event, Handler::TriggerBundle>,
	query: Query<(Entity, &OnGlobalTrigger<Handler>)>,
	mut commands: Commands,
) {
	for (entity, action) in query.iter() {
		Handler::handle(&mut commands, &trigger, (entity, &action.0));
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
		app.add_plugins((ActionPlugin::<
			TriggerOnGlobalTrigger<OnRun, OnRunResult>,
		>::default(),));
		let world = app.world_mut();
		let func = observe_run_results(world);

		world.spawn(TriggerOnGlobalTrigger::<OnRun, OnRunResult>::new(
			OnRunResult::failure(),
		));
		// global trigger
		world.trigger(OnRun);
		world.flush();
		expect(&func).to_have_returned_nth_with(0, &RunResult::Failure)?;
		Ok(())
	}
}
