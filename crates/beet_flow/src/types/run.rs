use beet_core::prelude::*;

#[derive(EntityEvent)]
pub struct Run<T = ()> {
	#[event_target]
	target: Entity,
	value: T,
}
impl<T> From<Entity> for Run<T>
where
	T: Default,
{
	fn from(target: Entity) -> Self { Self::new(target, default()) }
}
impl<T> Run<T> {
	pub fn new(target: Entity, value: T) -> Self { Self { target, value } }
	pub fn target(&self) -> Entity { self.target }
	pub fn value(&self) -> &T { &self.value }
}

pub const RUN: IntoRun<()> = IntoRun(());

#[derive(Debug, Default, Clone)]
pub struct IntoRun<T = ()>(T);
impl<T> IntoRun<T> {
	pub fn new(value: T) -> Self { Self(value) }
}

impl<T: 'static + Send + Sync> IntoEntityEvent for IntoRun<T> {
	type Event = Run<T>;
	fn into_entity_event(self, entity: Entity) -> Self::Event {
		Run::new(entity, self.0)
	}
}
