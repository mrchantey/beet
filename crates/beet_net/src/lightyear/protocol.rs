//! This file contains the shared [`Protocol`] that defines the messages that can be sent between the client and server.
//!
//! You will need to define the [`Components`], [`Messages`] and [`Inputs`] that make up the protocol.
//! You can use the `#[protocol]` attribute to specify additional behaviour:
//! - how entities contained in the message should be mapped from the remote world to the local world
//! - how the component should be synchronized between the `Confirmed` entity and the `Predicted`/`Interpolated` entity
use bevy::ecs::entity::MapEntities;
use bevy::prelude::default;
use bevy::prelude::App;
use bevy::prelude::Bundle;
use bevy::prelude::Component;
use bevy::prelude::Entity;
use bevy::prelude::EntityMapper;
use bevy::prelude::Plugin;
use lightyear::client::components::ComponentSyncMode;
use lightyear::prelude::*;
use serde::Deserialize;
use serde::Serialize;

// Player
#[derive(Bundle)]
pub(crate) struct PlayerBundle {
	id: PlayerId,
}

impl PlayerBundle {
	pub(crate) fn new(id: ClientId) -> Self { Self { id: PlayerId(id) } }
}

// Components

#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayerId(ClientId);

// Example of a component that contains an entity.
// This component, when replicated, needs to have the inner entity mapped from the Server world
// to the client World.
// You will need to derive the `MapEntities` trait for the component, and register
// app.add_map_entities<PlayerParent>() in your protocol
#[derive(Component, Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct PlayerParent(Entity);

impl MapEntities for PlayerParent {
	fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
		self.0 = entity_mapper.map_entity(self.0);
	}
}

// Channels

#[derive(Channel)]
pub struct Channel1;

// Messages

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Message1(pub usize);

// Inputs

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Direction {
	pub(crate) up: bool,
	pub(crate) down: bool,
	pub(crate) left: bool,
	pub(crate) right: bool,
}

impl Direction {
	pub(crate) fn is_none(&self) -> bool {
		!self.up && !self.down && !self.left && !self.right
	}
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum Inputs {
	Direction(Direction),
	Delete,
	Spawn,
	None,
}

// Protocol
pub(crate) struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
	fn build(&self, app: &mut App) {
		// messages
		app.add_message::<Message1>(ChannelDirection::Bidirectional);
		// inputs
		app.add_plugins(InputPlugin::<Inputs>::default());
		// components
		app.register_component::<PlayerId>(ChannelDirection::ServerToClient)
			.add_prediction::<PlayerId>(ComponentSyncMode::Once)
			.add_interpolation::<PlayerId>(ComponentSyncMode::Once);

		// channels
		app.add_channel::<Channel1>(ChannelSettings {
			mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
			..default()
		});
	}
}
