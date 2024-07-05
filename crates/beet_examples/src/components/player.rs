use crate::prelude::*;
use beet::prelude::*;
use bevy::prelude::*;

#[derive(Debug, Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct Player;
pub fn set_player_sentence(
	mut commands: Commands,
	// mut npc_events: EventWriter<OnNpcMessage>,
	mut events: EventReader<OnUserMessage>,
	query: Query<Entity, With<Player>>,
) {
	for ev in events.read() {
		for entity in query.iter() {
			commands.entity(entity).insert(Sentence::new(ev.0.clone()));
		}
		// npc_events.send(OnNpcMessage("ok".to_string()));
	}
}
