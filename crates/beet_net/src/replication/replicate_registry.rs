use crate::prelude::*;
use bevy::prelude::*;
use bevy::utils::HashMap;
use serde::Deserialize;
use serde::Serialize;
use std::any::TypeId;

/// Unique identifier for components registered.
#[derive(
	Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, Deref,
)]
pub struct RegistrationId(usize);

impl RegistrationId {
	pub fn inner(&self) -> usize { self.0 }
	// #[cfg(test)]
	pub fn new_with(id: usize) -> Self { Self(id) }
}

#[derive(Default, Resource)]
pub struct ReplicateRegistry {
	id_incr: usize,

	types: HashMap<TypeId, RegistrationId>,

	#[cfg(debug_assertions)]
	type_names: HashMap<RegistrationId, String>,

	/// Map of remote to local entity ids
	pub entities: HashMap<Entity, Entity>,
	pub incoming_component_fns: HashMap<RegistrationId, ComponentFns>,
	pub incoming_resource_fns: HashMap<RegistrationId, ResourceFns>,
	pub incoming_event_fns: HashMap<RegistrationId, EventFns>,
	pub directions: HashMap<RegistrationId, ReplicateDirection>,
}

impl ReplicateRegistry {
	pub fn registration_id<T: 'static>(&self) -> RegistrationId {
		if let Some(value) = self.types.get(&TypeId::of::<T>()) {
			*value
		} else {
			let name = std::any::type_name::<T>();
			panic!("Type {} is not registered", name);
		}
	}

	#[cfg(debug_assertions)]
	pub fn types_to_json(&self) -> String {
		let types = self
			.types
			.values()
			.map(|v| {
				let name = self.type_names.get(v).unwrap();
				format!("  \"{name}\": {}", **v)
			})
			.collect::<Vec<String>>()
			.join(",\n");
		format!("{{\n{}\n}}", types)
	}

	pub fn entity_fns(
		&self,
		remote: Entity,
		id: RegistrationId,
	) -> Option<(Entity, &ComponentFns)> {
		if let Some(entity) = self.entities.get(&remote) {
			if let Some(fns) = self.incoming_component_fns.get(&id) {
				return Some((*entity, fns));
			}
		}
		None
	}

	fn next_id<T: 'static>(
		&mut self,
		direction: ReplicateDirection,
	) -> RegistrationId {
		let id = RegistrationId(self.id_incr);
		self.id_incr += 1;
		self.directions.insert(id, direction);
		self.types.insert(std::any::TypeId::of::<T>(), id);
		#[cfg(debug_assertions)]
		self.type_names
			.insert(id, std::any::type_name::<T>().to_string());
		id
	}

	pub fn register_component<T: Component>(
		&mut self,
		fns: ComponentFns,
		direction: ReplicateDirection,
	) -> RegistrationId {
		let id = self.next_id::<T>(direction);
		if direction.is_incoming() {
			self.incoming_component_fns.insert(id, fns);
		}
		id
	}
	pub fn register_resource<T: Resource>(
		&mut self,
		fns: ResourceFns,
		direction: ReplicateDirection,
	) -> RegistrationId {
		let id = self.next_id::<T>(direction);
		if direction.is_incoming() {
			self.incoming_resource_fns.insert(id, fns);
		}
		id
	}
	pub fn register_event<T: Event>(
		&mut self,
		fns: EventFns,
		direction: ReplicateDirection,
	) -> RegistrationId {
		let id = self.next_id::<T>(direction);
		if direction.is_incoming() {
			self.incoming_event_fns.insert(id, fns);
		}
		id
	}
}
