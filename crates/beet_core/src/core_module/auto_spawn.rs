use crate::prelude::*;
use anyhow::Result;
use beet::action::ActionTypes;
use beet::reflect::BeetSceneSerde;
use bevy::prelude::*;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Component, Reflect)]
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

#[cfg(test)]
mod test {
	use std::time::Duration;

use crate::prelude::*;
	use anyhow::Result;
	use beet::{graph::BeetBuilder, node::Score};
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		pretty_env_logger::try_init().ok();
		let mut app = App::new();
		app /*-*/		
			.add_plugins(BeetMessagePlugin::<CoreModule>(default()))
			.add_plugins(BeetTypesPlugin::<CoreModule>(default()))
		/*-*/;

		let send = app.world_mut().resource::<BeetMessageSend>().clone();

		let prefab1 = BeetBuilder::new(Score::Weight(0.1))
			.into_scene::<CoreModule>();
		

		let auto_spawn = AutoSpawn::new(prefab1, Duration::from_secs(1))?;
		let prefab2 = BeetBuilder::new(auto_spawn.clone())
		.into_scene::<CoreModule>();
	
		let bincode = bincode::serialize(&prefab2)?;

		// log::info!("{:?}", bincode);
		send.send(BeetMessage::Spawn{bincode})?;

		expect(app.world().entities().len()).to_be(0)?;
		app.update();
		expect(app.world().entities().len()).to_be(1)?;

		let first = app.world().iter_entities().next().unwrap().id();

		expect(&app).component(first)?.to_be(&auto_spawn)?;

		Ok(())
	}
}
