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
	SpawnBee,
	SpawnFlower,
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
			DomSimMessage::SpawnBee => {
				let behavior = forage().build(world).value;
				world.spawn(bee_bundle()).add_child(behavior);
			}
			DomSimMessage::SpawnFlower => {
				world.spawn(flower_bundle());
			}
			DomSimMessage::DespawnAll => {
				world.clear_entities();
			}
			DomSimMessage::Resize => {
				trigger_transform_change(world);
			}
		}
	}
	Ok(())
}
