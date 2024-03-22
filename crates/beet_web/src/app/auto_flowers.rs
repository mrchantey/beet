use crate::prelude::*;
use bevy::ecs as bevy_ecs;
use bevy::prelude::*;

#[derive(Resource, Deref, DerefMut)]
pub struct AutoFlowers(pub Timer);



pub fn auto_flowers_spawn(
	time: Res<Time>,
	mut timer: ResMut<AutoFlowers>,
	send: Res<DomSimMessageSend>,
) {
	if timer.tick(time.delta()).finished() {
		send.send(DomSimMessage::SpawnFlower).ok();
	}
}
