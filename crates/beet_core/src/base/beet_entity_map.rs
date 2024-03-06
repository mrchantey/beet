use beet_ecs::prelude::*;
use beet_net::prelude::*;
use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_ecs::prelude::*;
use bevy_utils::HashMap;
use serde::Deserialize;
use serde::Serialize;
use strum::IntoEnumIterator;

pub trait ActionPayload:
	Payload + ActionSuper + IntoEnumIterator + IntoAction
{
}
impl<T: Payload + ActionSuper + IntoEnumIterator + IntoAction> ActionPayload
	for T
{
}

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
/// Incrementable without `ResMut` beause uses AtomicUsize
#[derive(Default, Resource)]
pub struct BeetEntityMap {
	id_incr: u64,
	map: HashMap<BeetEntityId, Entity>,
}

impl BeetEntityMap {
	pub fn next(&mut self, entity: Entity) -> BeetEntityId {
		let id = self.id_incr;
		self.id_incr = self.id_incr.wrapping_add(1);
		let id = BeetEntityId(id);
		self.map.insert(id, entity);
		id
	}
}
