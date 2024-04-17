//! This file contains the shared [`Protocol`] that defines the messages that can be sent between the client and server.
//!
//! You will need to define the [`Components`], [`Messages`] and [`Inputs`] that make up the protocol.
//! You can use the `#[protocol]` attribute to specify additional behaviour:
//! - how entities contained in the message should be mapped from the remote world to the local world
//! - how the component should be synchronized between the `Confirmed` entity and the `Predicted`/`Interpolated` entity
use bevy::ecs::entity::MapEntities;
use bevy::prelude::default;
use bevy::prelude::Bundle;
use bevy::prelude::Component;
use bevy::prelude::Deref;
use bevy::prelude::DerefMut;
use bevy::prelude::Entity;
use bevy::prelude::EntityMapper;
use bevy::prelude::Vec2;
use derive_more::Add;
use derive_more::Mul;
use lightyear::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use std::ops::Mul;


// Player
#[derive(Bundle)]
pub struct PlayerBundle {
	id: PlayerId,
	position: PlayerPosition,
}

impl PlayerBundle {
	pub fn new(id: ClientId, position: Vec2) -> Self {
		// Generate pseudo random color from client id.
		let _h = (((id.to_bits().wrapping_mul(30)) % 360) as f32) / 360.0;
		let _s = 0.8;
		let _l = 0.5;
		Self {
			id: PlayerId(id),
			position: PlayerPosition(position),
		}
	}
}

// Components

#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayerId(ClientId);

#[derive(
	Component,
	Serialize,
	Deserialize,
	Clone,
	Debug,
	PartialEq,
	Deref,
	DerefMut,
	Add,
	Mul,
)]
pub struct PlayerPosition(Vec2);

impl Mul<f32> for &PlayerPosition {
	type Output = PlayerPosition;

	fn mul(self, rhs: f32) -> Self::Output { PlayerPosition(self.0 * rhs) }
}

// Example of a component that contains an entity.
// This component, when replicated, needs to have the inner entity mapped from the Server world
// to the client World.
// This can be done by adding a `#[message(custom_map)]` attribute to the component, and then
// deriving the `MapEntities` trait for the component.
#[derive(Component, Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct PlayerParent(Entity);

impl MapEntities for PlayerParent {
	fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
		self.0 = entity_mapper.map_entity(self.0);
	}
}

#[component_protocol(protocol = "MyProtocol")]
pub enum Components {
	#[protocol(sync(mode = "once"))]
	PlayerId(PlayerId),
	#[protocol(sync(mode = "full"))]
	PlayerPosition(PlayerPosition),
}

// Channels

#[derive(Channel)]
pub struct Channel1;

// Messages

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Message1(pub usize);

#[message_protocol(protocol = "MyProtocol")]
pub enum Messages {
	Message1(Message1),
}

// Inputs

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Direction {
	pub up: bool,
	pub down: bool,
	pub left: bool,
	pub right: bool,
}

impl Direction {
	pub fn is_none(&self) -> bool {
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

impl UserAction for Inputs {}

// Protocol

protocolize! {
	Self = MyProtocol,
	Message = Messages,
	Component = Components,
	Input = Inputs,
}

pub fn protocol() -> MyProtocol {
	let mut protocol = MyProtocol::default();
	protocol.add_channel::<Channel1>(ChannelSettings {
		mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
		..default()
	});
	protocol
}
