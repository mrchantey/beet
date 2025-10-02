use crate::prelude::*;
use beet_core::prelude::*;

#[derive(Debug, Clone, EntityEvent)]
pub struct Run<T = RequestEndResult> {
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

#[derive(
	Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Reflect,
)]
pub struct RequestEndResult;

pub const RUN: RequestEndResult = RequestEndResult;

impl IntoEntityEvent for RequestEndResult {
	type Event = Run<RequestEndResult>;
	fn into_entity_event(self, entity: Entity) -> Self::Event {
		Run::new(entity, self)
	}
}


impl RunPayload for RequestEndResult {
	type End = EndResult;
}
impl EndPayload for EndResult {
	type Run = RequestEndResult;
}
pub trait RunPayload: 'static + Send + Sync + IntoEntityEvent {
	type End: EndPayload<Run = Self>;
}

pub trait EndPayload: 'static + Send + Sync + IntoEntityEvent {
	type Run: RunPayload<End = Self>;
}
