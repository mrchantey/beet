use crate::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;

/// when [`<In>`] is called, trigger [`<Out>`]
#[derive(Action, Reflect)]
#[reflect(Default, Component)]
#[global_observers(on_global_trigger::<In,Out>)]
pub struct TriggerOnGlobalTrigger<
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
	for TriggerOnGlobalTrigger<In, Out>
{
	fn default() -> Self { Self::new(Out::default()) }
}

impl<In: GenericActionEvent, Out: Default + GenericActionEvent>
	TriggerOnGlobalTrigger<In, Out>
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

fn on_global_trigger<
	In: GenericActionEvent,
	Out: Default + GenericActionEvent,
>(
	_trigger: Trigger<In>,
	query: Query<(Entity, &TriggerOnGlobalTrigger<In, Out>)>,
	mut commands: Commands,
) {
	for (entity, action) in query.iter() {
		action
			.target
			.trigger(&mut commands, entity, action.out.clone());
	}
}
