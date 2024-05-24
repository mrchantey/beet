use crate::prelude::*;
use bevy::prelude::*;
use bevy::utils::HashMap;
use serde::Deserialize;
use serde::Serialize;
use std::any::TypeId;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

/// Unique identifier for components registered.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct RegistrationId(usize);

impl RegistrationId {
	const ID_INCR: AtomicUsize = AtomicUsize::new(0);

	fn next() -> Self {
		let id = Self::ID_INCR.fetch_add(1, Ordering::SeqCst);
		RegistrationId(id)
	}

	pub fn inner(&self) -> usize { self.0 }

	#[cfg(test)]
	pub fn new_with(id: usize) -> Self { Self(id) }
}

#[derive(Default, Resource)]
pub struct Registrations {
	pub types: HashMap<TypeId, RegistrationId>,

	#[cfg(debug_assertions)]
	pub type_names: HashMap<RegistrationId, String>,

	/// Map of remote to local entity ids
	pub entities: HashMap<Entity, Entity>,
	pub components: HashMap<RegistrationId, ComponentFns>,
	pub resources: HashMap<RegistrationId, ResourceFns>,
	pub events: HashMap<RegistrationId, EventFns>,
}


impl Registrations {
	pub fn registration_id<T: 'static>(&self) -> RegistrationId {
		if let Some(value) = self.types.get(&TypeId::of::<T>()) {
			*value
		} else {
			let name = std::any::type_name::<T>();
			panic!("Type {} is not registered", name);
		}
	}

	pub fn entity_fns(
		&self,
		remote: Entity,
		id: RegistrationId,
	) -> Option<(Entity, &ComponentFns)> {
		if let Some(entity) = self.entities.get(&remote) {
			if let Some(fns) = self.components.get(&id) {
				return Some((*entity, fns));
			}
		}
		None
	}

	fn next_id<T: 'static>(&mut self) -> RegistrationId {
		let id = RegistrationId::next();
		self.types.insert(std::any::TypeId::of::<T>(), id);
		#[cfg(debug_assertions)]
		self.type_names
			.insert(id, std::any::type_name::<T>().to_string());
		id
	}

	pub fn register_component<T: Component>(
		&mut self,
		fns: ComponentFns,
	) -> RegistrationId {
		let id = self.next_id::<T>();
		self.components.insert(id, fns);
		id
	}
	pub fn register_resource<T: Resource>(
		&mut self,
		fns: ResourceFns,
	) -> RegistrationId {
		let id = self.next_id::<T>();
		self.resources.insert(id, fns);
		id
	}
	pub fn register_event<T: Event>(
		&mut self,
		fns: EventFns,
	) -> RegistrationId {
		let id = self.next_id::<T>();
		self.events.insert(id, fns);
		id
	}
}
