use crate::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;


/// Adds the provided component when [`<E>`] is triggered
#[derive(Component, Action, Reflect)]
#[reflect(Default, Component)]
#[observers(on_trigger::<Handler>)]
pub struct OnTrigger<Handler: OnTriggerHandler> {
	pub params: Handler::Params,
	pub target: TriggerTarget,
	#[reflect(ignore)]
	phantom: PhantomData<Handler>,
}

impl<Handler: OnTriggerHandler> Default for OnTrigger<Handler>
where
	Handler::Params: Default,
{
	fn default() -> Self { Self::new(Handler::Params::default()) }
}

impl<Handler: OnTriggerHandler> OnTrigger<Handler> {
	pub fn new(params: Handler::Params) -> Self {
		Self {
			params,
			target: default(),
			phantom: PhantomData,
		}
	}
	pub fn with_target(self, target: impl Into<TriggerTarget>) -> Self {
		Self {
			target: target.into(),
			..self
		}
	}
}

fn on_trigger<Handler: OnTriggerHandler>(
	trigger: Trigger<Handler::Event, Handler::TriggerBundle>,
	query: Query<(Entity, &OnTrigger<Handler>)>,
	mut commands: Commands,
) {
	let action = query
		.get(trigger.entity())
		.expect(expect_action::ACTION_QUERY_MISSING);
	Handler::handle(&mut commands, &trigger, action);
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
		app.add_plugins(
			ActionPlugin::<InsertOnTrigger<OnRun, Running>>::default(),
		);
		let world = app.world_mut();

		let entity = world
			.spawn(InsertOnTrigger::<OnRun, Running>::default())
			.flush_trigger(OnRun)
			.id();
		expect(world.entities().len()).to_be(2)?;
		expect(&*world).to_have_component::<Running>(entity)?;
		Ok(())
	}
}
