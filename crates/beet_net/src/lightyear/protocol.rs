use bevy::app::App;
use bevy::app::Plugin;
use bevy::prelude::default;
use bevy::prelude::Bundle;
use bevy::prelude::Component;
use bevy::prelude::Deref;
use bevy::prelude::DerefMut;
use bevy::prelude::Vec2;
use derive_more::Add;
use derive_more::Mul;
use lightyear::client::components::ComponentSyncMode;
use lightyear::prelude::client::Replicate;
use lightyear::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use std::ops::Mul;

pub type Color = usize;

// Player
#[derive(Bundle)]
pub(crate) struct PlayerBundle {
	id: PlayerId,
	position: PlayerPosition,
	color: PlayerColor,
	replicate: Replicate,
}

impl PlayerBundle {
	pub(crate) fn new(id: ClientId, position: Vec2) -> Self {
		Self {
			id: PlayerId(id),
			position: PlayerPosition(position),
			color: PlayerColor(0),
			replicate: Replicate::default(),
		}
	}
}

// Player
#[derive(Bundle)]
pub(crate) struct CursorBundle {
	id: PlayerId,
	position: CursorPosition,
	color: PlayerColor,
	replicate: Replicate,
}

impl CursorBundle {
	pub(crate) fn new(id: ClientId, position: Vec2, color: Color) -> Self {
		Self {
			id: PlayerId(id),
			position: CursorPosition(position),
			color: PlayerColor(color),
			replicate: Replicate::default(),
		}
	}
}

// Components

#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayerId(pub ClientId);

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

#[derive(Component, Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct PlayerColor(pub usize);

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
)]
pub struct CursorPosition(pub Vec2);

impl Mul<f32> for &CursorPosition {
	type Output = CursorPosition;

	fn mul(self, rhs: f32) -> Self::Output { CursorPosition(self.0 * rhs) }
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
		app.register_component::<PlayerId>(ChannelDirection::Bidirectional)
			.add_prediction(ComponentSyncMode::Once)
			.add_interpolation(ComponentSyncMode::Once);

		app.register_component::<PlayerPosition>(
			ChannelDirection::Bidirectional,
		)
		.add_prediction(ComponentSyncMode::Full)
		.add_interpolation(ComponentSyncMode::Full)
		.add_linear_interpolation_fn();

		app.register_component::<PlayerColor>(ChannelDirection::Bidirectional)
			.add_prediction(ComponentSyncMode::Once)
			.add_interpolation(ComponentSyncMode::Once);

		app.register_component::<CursorPosition>(
			ChannelDirection::Bidirectional,
		)
		.add_prediction(ComponentSyncMode::Full)
		.add_interpolation(ComponentSyncMode::Full)
		.add_linear_interpolation_fn();
		// channels
		app.add_channel::<Channel1>(ChannelSettings {
			mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
			..default()
		});
	}
}
