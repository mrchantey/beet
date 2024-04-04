use crate::prelude::*;
use bevy::ecs as bevy_ecs;
use bevy::prelude::*;
use bevy::reflect as bevy_reflect;

#[derive(Deref, DerefMut, Component, Reflect)]
#[reflect(Component)]
pub struct AutoFlowers(pub Timer);



pub fn auto_flowers_spawn(
	time: Res<Time>,
	mut query: Query<&mut AutoFlowers>,
	send: Res<DomSimMessageSend>,
) {
	for mut timer in query.iter_mut() {
		if timer.tick(time.delta()).finished() {
			send.send(DomSimMessage::SpawnFlower).ok();
		}
	}
}
