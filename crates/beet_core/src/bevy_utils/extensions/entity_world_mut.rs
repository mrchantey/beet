use bevy::prelude::*;


pub trait IntoEntityTrigger: 'static + Send + Sync {}
impl<T> IntoEntityTrigger for T where T: 'static + Send + Sync {}

#[derive(EntityEvent)]
pub struct EntityTrigger<T> {
	#[event_target]
	target: Entity,
	pub payload: T,
}
impl<T> EntityTrigger<T> {
	pub fn new(target: Entity, payload: T) -> Self { Self { target, payload } }
	// pub fn target(&self) -> Entity { self.target }
	pub fn take(self) -> T { self.payload }
}


impl<T> std::ops::Deref for EntityTrigger<T> {
	type Target = T;
	fn deref(&self) -> &Self::Target { &self.payload }
}
impl<T> std::ops::DerefMut for EntityTrigger<T> {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.payload }
}

#[extend::ext(name=EntityWorldMutExt)]
pub impl EntityWorldMut<'_> {
	fn entity_trigger<T: IntoEntityTrigger>(
		&mut self,
		payload: T,
	) -> &mut Self {
		let ev = EntityTrigger {
			target: self.id(),
			payload,
		};
		self.world_scope(move |world| {
			world.trigger(ev);
		});
		self
	}
}
