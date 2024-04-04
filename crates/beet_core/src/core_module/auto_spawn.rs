use crate::prelude::*;
use anyhow::Result;
use beet::action::ActionTypes;
use beet::reflect::BeetSceneSerde;
use bevy::prelude::*;
use std::time::Duration;

#[derive(Clone, Component, Reflect)]
#[reflect(Component)]
pub struct AutoSpawn {
	pub timer: Timer,
	pub scene_bincode: Vec<u8>,
}


impl AutoSpawn {
	pub fn new<T: ActionTypes>(
		scene: BeetSceneSerde<T>,
		interval: Duration,
	) -> Result<Self> {
		let scene_bincode = bincode::serialize(&scene)?;
		Ok(Self {
			timer: Timer::new(interval, TimerMode::Repeating),
			scene_bincode,
		})
	}
}


pub fn auto_spawn(
	time: Res<Time>,
	mut query: Query<&mut AutoSpawn>,
	send: Res<BeetMessageSend>,
) {
	for mut spawner in query.iter_mut() {
		if spawner.timer.tick(time.delta()).finished() {
			send.send(BeetMessage::Spawn {
				bincode: spawner.scene_bincode.clone(),
			})
			.ok();
		}
	}
}
