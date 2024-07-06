use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;



/// Received by this app, containing the raw text of a ron file for
/// deserialization and spawning
#[derive(Debug, Clone, Serialize, Deserialize, Event, Reflect)]
pub struct ClearEntities;
/// Received by this app, containing the raw text of a ron file for
/// deserialization and spawning
#[derive(Debug, Clone, Component, Reflect)]
pub struct NeverClear;

pub fn handle_clear_entities(world: &mut World) {
	for entity in world
		.query_filtered::<Entity, Without<NeverClear>>()
		.iter(world)
		.collect::<Vec<_>>()
		.into_iter()
	{
		world.despawn(entity);
	}
}
