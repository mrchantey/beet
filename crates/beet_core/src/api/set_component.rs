use crate::prelude::*;
use anyhow::Result;
use beet_net::prelude::*;
use bevy_ecs::prelude::*;

#[derive(Resource)]
pub struct SetComponentHandler {
	pub send: Publisher<SerializedComponent>,
	pub recv: Subscriber<SerializedComponent>,
}

impl TopicHandler<SerializedComponent> for SetComponentHandler {
	fn topic() -> Topic {
		Topic::new(ENTITY_TOPIC, TopicScheme::PubSub, TopicMethod::Delete)
	}
}

impl SetComponentHandler {
	pub fn new(relay: &mut Relay) -> Result<Self> {
		Ok(Self {
			send: Self::publisher(relay)?,
			recv: Self::subscriber(relay)?,
		})
	}
}

pub fn handle_set_component(world: &mut World) -> Result<()> {
	for _message in world
		.resource_mut::<SetComponentHandler>()
		.recv
		.try_recv_all()?
	{
		todo!();
	}
	Ok(())
}
