use crate::prelude::*;
use anyhow::Result;
use beet::prelude::*;
use bevy::ecs as bevy_ecs;
use bevy::prelude::*;
use flume::Receiver;
use flume::Sender;


#[derive(Clone)]
pub enum DomSimMessage {
	DespawnAll,
	Resize,
}

#[derive(Resource, Deref, DerefMut)]
pub struct DomSimMessageRecv(pub Receiver<DomSimMessage>);
#[derive(Resource, Deref, DerefMut)]
pub struct DomSimMessageSend(pub Sender<DomSimMessage>);

pub fn message_handler(world: &mut World) -> Result<()> {
	let Ok(messages) = world.resource_mut::<DomSimMessageRecv>().try_recv_all()
	else {
		return Ok(()); // disconnected
	};

	for message in messages {
		match message {
			DomSimMessage::DespawnAll => {
				// dont despawn everything, we need the graph
				for entity in world
					.query_filtered::<Entity, With<Transform>>()
					.iter(world)
					.collect::<Vec<_>>()
				{
					world.despawn(entity);
				}
			}
			DomSimMessage::Resize => {
				trigger_transform_change(world);
			}
		}
	}
	Ok(())
}
