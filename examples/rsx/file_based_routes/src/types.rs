use beet::exports::bevy::ecs as bevy_ecs;
use beet::prelude::*;
use serde::Deserialize;


/// The metadata at the top of a markdown article,
#[derive(Debug, Default, Clone, Component, Deserialize)]
pub struct Article {
	pub title: String,
	pub created: Option<String>,
}
