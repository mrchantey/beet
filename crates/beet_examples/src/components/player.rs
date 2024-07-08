use crate::prelude::*;
use crate::beet::prelude::*;
use bevy::prelude::*;

#[derive(Debug, Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct Player;
pub fn set_player_sentence(
	mut commands: Commands,
	mut events: EventReader<OnUserMessage>,
	query: Query<Entity, With<Player>>,
) {
	for ev in events.read() {
		for entity in query.iter() {
			commands.entity(entity).insert(Sentence::new(ev.0.clone()));
		}
	}
}
