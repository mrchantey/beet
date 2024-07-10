use crate::prelude::*;
use bevy::prelude::*;
use std::any::type_name;
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
	/// if set, triggers without a target, otherwise targets self
	pub target: TriggerTarget,
	#[reflect(ignore)]
	phantom: PhantomData<In>,
}

impl<In: GenericActionEvent, Out: Default + GenericActionEvent> Default
	for TriggerOnTrigger<In, Out>
{
	fn default() -> Self { Self::new(Out::default()) }
}

impl<In: GenericActionEvent, Out: Default + GenericActionEvent>
	TriggerOnTrigger<In, Out>
{
	pub fn new(out: Out) -> Self {
		Self {
			out,
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

fn on_trigger<In: GenericActionEvent, Out: Default + GenericActionEvent>(
	trigger: Trigger<In>,
	query: Query<&TriggerOnTrigger<In, Out>>,
	mut commands: Commands,
) {
	log::info!(
		"TRIGGERED\nin: {}\nout: {}",
		type_name::<In>(),
		type_name::<Out>()
	);
	let action = query
		.get(trigger.entity())
		.expect(expect_action::ACTION_QUERY_MISSING);
	action
		.target
		.trigger(&mut commands, trigger.entity(), action.out.clone());
}


// see `end_on_run` for tests
