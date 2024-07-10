use crate::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;

pub trait OnTriggerHandler: 'static + Send + Sync + Sized {
	type Event: Event;
	/// The bundle used for the Trigger, ie Trigger<E,B>
	type TriggerBundle: Bundle = ();
	type Params: 'static + Send + Sync + Default + Reflect;
	fn handle(
		commands: &mut Commands,
		ev: &Trigger<Self::Event, Self::TriggerBundle>,
		comp: &OnTrigger<Self>,
	);
}

pub trait MapFunc: 'static + Send + Sync {
	type Event: Event;
	type TriggerBundle: Bundle = ();
	type Params: 'static + Send + Sync + Clone + Default + Reflect;
	type Out: Bundle;
	fn map(
		trigger: &Trigger<Self::Event, Self::TriggerBundle>,
		params: &Self::Params,
	) -> Self::Out;
}
#[derive(Debug, Default, Clone, PartialEq, Reflect)]
pub struct DefaultMapFunc<E, T, B>(#[reflect(ignore)] PhantomData<(E, T, B)>);
impl<E: Event, T: Bundle + Clone + Default + Reflect, B: Bundle> MapFunc
	for DefaultMapFunc<E, T, B>
{
	type Event = E;
	type TriggerBundle = B;
	type Params = T;
	type Out = T;
	fn map(_ev: &Trigger<E, B>, params: &T) -> T { params.clone() }
}


/// Adds the provided component when [`<E>`] is triggered
#[derive(Action, Reflect)]
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
	query: Query<&OnTrigger<Handler>>,
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
