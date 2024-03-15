use crate::api::DespawnEntityHandler;
use crate::api::DespawnEntityPayload;
use anyhow::Result;
use bevy::prelude::*;
use bevy::utils::HashMap;
use core::fmt;
use serde::Deserialize;
use serde::Serialize;
use std::error::Error;
use std::sync::atomic::AtomicUsize;



static ID_INCR: AtomicUsize = AtomicUsize::new(0);


/// This is a crutch until we have proper pub sub topic keying
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
impl BeetEntityId {
	pub fn next() -> Self {
		BeetEntityId(
			ID_INCR.fetch_add(1, std::sync::atomic::Ordering::SeqCst) as u64
		)
	}
}


impl Into<BeetEntityId> for u64 {
	fn into(self) -> BeetEntityId { BeetEntityId(self) }
}

impl fmt::Display for BeetEntityId {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.0)
	}
}


#[derive(Default, Resource)]
pub struct BeetEntityMap {
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

	pub fn try_insert(
		&mut self,
		id: BeetEntityId,
		entity: Entity,
	) -> Result<(), EntityExistsError> {
		if self.map.contains_key(&id) {
			Err(EntityExistsError(id))
		} else {
			self.map.insert(id, entity);
			self.reverse_map.insert(entity, id);
			Ok(())
		}
	}

	pub fn map(&self) -> &HashMap<BeetEntityId, Entity> { &self.map }

	pub fn next(
		&mut self,
		entity: Entity,
	) -> Result<BeetEntityId, EntityExistsError> {
		let id = BeetEntityId::next();
		self.try_insert(id, entity)?;
		Ok(id)
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


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityExistsError(pub BeetEntityId);

impl fmt::Display for EntityExistsError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "Beet Map - Entity already exists: {}", self.0)
	}
}

impl Error for EntityExistsError {}
