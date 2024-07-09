use crate::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;

pub type TriggerOnRun<T> = TriggerOnTrigger<OnRun, T>;

/// when [`<In>`] is called, trigger [`<Out>`]
#[derive(Action, Reflect)]
#[reflect(Default, Component)]
#[observers(on_trigger::<In,Out>)]
pub struct TriggerOnTrigger<
	In: GenericActionEvent,
	Out: Default + GenericActionEvent,
> {
	pub out: Out,
	#[reflect(ignore)]
	phantom: PhantomData<In>,
}

impl<In: GenericActionEvent, Out: Default + GenericActionEvent>
	TriggerOnTrigger<In, Out>
{
	pub fn new(out: Out) -> Self {
		Self {
			out,
			phantom: PhantomData,
		}
	}
}

fn on_trigger<In: GenericActionEvent, Out: Default + GenericActionEvent>(
	trigger: Trigger<In>,
	query: Query<&TriggerOnTrigger<In, Out>>,
	mut commands: Commands,
) {
	let action = query
		.get(trigger.entity())
		.expect(expect_action::NO_ACTION_COMP);
	commands.trigger_targets(action.out.clone(), trigger.entity());
}

impl<In: GenericActionEvent, Out: Default + GenericActionEvent> Default
	for TriggerOnTrigger<In, Out>
{
	fn default() -> Self {
		Self {
			out: Out::default(),
			phantom: PhantomData,
		}
	}
}

// see `end_on_run` for tests