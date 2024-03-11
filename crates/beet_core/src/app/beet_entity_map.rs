use crate::api::DespawnEntityHandler;
use crate::api::DespawnEntityPayload;
use anyhow::Result;
use beet_ecs::prelude::*;
use beet_net::prelude::*;
use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_ecs::prelude::*;
use bevy_utils::HashMap;
use core::fmt;
use serde::Deserialize;
use serde::Serialize;
use std::error::Error;

pub trait ActionPayload: Payload + ActionSuper + ActionList {}
impl<T: Payload + ActionSuper + ActionList> ActionPayload for T {}

#[derive(
	Debug,
	Copy,
	Clone,
	Serialize,
	Deserialize,
	Deref,
	DerefMut,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Component,
)]
pub struct BeetEntityId(pub u64);

impl fmt::Display for BeetEntityId {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.0)
	}
}


#[derive(Default, Resource)]
pub struct BeetEntityMap {
	id_incr: u64,
	map: HashMap<BeetEntityId, Entity>,
	reverse_map: HashMap<Entity, BeetEntityId>,
}

impl BeetEntityMap {
	pub fn get(
		&self,
		id: BeetEntityId,
	) -> Result<&Entity, EntityNotFoundError> {
		self.map.get(&id).ok_or(EntityNotFoundError(id))
	}

	pub fn map(&self) -> &HashMap<BeetEntityId, Entity> { &self.map }

	pub fn next(&mut self, entity: Entity) -> BeetEntityId {
		let id = self.id_incr;
		self.id_incr = self.id_incr.wrapping_add(1);
		let id = BeetEntityId(id);
		self.map.insert(id, entity);
		self.reverse_map.insert(entity, id);
		id
	}
}

pub fn cleanup_beet_entity_map(
	mut entity_map: ResMut<BeetEntityMap>,
	handler: Res<DespawnEntityHandler>,
	mut removed: RemovedComponents<BeetEntityId>,
) -> Result<()> {
	for entity in removed.read() {
		if let Some(id) = entity_map.reverse_map.remove(&entity) {
			entity_map.map.remove(&id);
			handler.send.push(&DespawnEntityPayload::new(id))?;
		} else {
			log::warn!("Entity {entity:?} not found in beet entity map")
		}
	}
	Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityNotFoundError(pub BeetEntityId);

impl fmt::Display for EntityNotFoundError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "Beet Map - Entity not found: {}", self.0)
	}
}

impl Error for EntityNotFoundError {}
