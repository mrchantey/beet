use crate::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;
use std::ops::Deref;
use std::ops::DerefMut;

/// Triggers the given event when this behavior starts Insertning.
#[derive(Debug, Clone, PartialEq, Action, Reflect)]
#[reflect(Component, ActionMeta)]
#[category(ActionCategory::World)]
#[systems(inset_on_trigger::<E, T>.in_set(TickSet))]
#[deprecated = "Use `TriggerOnTrigger` instead"]
pub struct InsertOnSend<E: GenericActionEvent, T: GenericActionComponent> {
	pub value: T,
	#[reflect(ignore)]
	phantom: PhantomData<E>,
}
impl<E: GenericActionEvent, T: GenericActionComponent> Deref
	for InsertOnSend<E, T>
{
	type Target = T;
	fn deref(&self) -> &Self::Target { &self.value }
}
impl<E: GenericActionEvent, T: GenericActionComponent> DerefMut
	for InsertOnSend<E, T>
{
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.value }
}

impl<E: GenericActionEvent, T: Default + GenericActionComponent> Default
	for InsertOnSend<E, T>
{
	fn default() -> Self {
		Self {
			value: T::default(),
			phantom: Default::default(),
		}
	}
}

impl<E: GenericActionEvent, T: GenericActionComponent> InsertOnSend<E, T> {
	pub fn new(value: impl Into<T>) -> Self {
		Self {
			value: value.into(),
			phantom: Default::default(),
		}
	}
}

fn inset_on_trigger<E: GenericActionEvent, T: GenericActionComponent>(
	mut commands: Commands,
	mut reader: EventReader<E>,
	query: Query<(Entity, &InsertOnSend<E, T>)>,
) {
	for _ev in reader.read() {
		// log::info!("EVENT");
		for (entity, trigger) in query.iter() {
			// log::info!("RECEIVED");
			commands.entity(entity).insert(trigger.value.clone());
		}
	}
}
