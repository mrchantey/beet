use crate::prelude::*;
use bevyhub::prelude::HandleWrapper;
use bevy::prelude::*;
use rand::Rng;
use std::time::Duration;

#[derive(Debug, Component, Reflect)]
#[reflect(Component)]
pub struct EmojiSwapper {
	timer: Timer,
	frequency_min: Duration,
	frequency_max: Duration,
	hexcodes: Vec<String>,
	index: usize,
}


impl EmojiSwapper {
	pub fn new(
		frequency_min: Duration,
		frequency_max: Duration,
		hexcodes: Vec<String>,
	) -> Self {
		Self {
			timer: Timer::new(frequency_max, TimerMode::Repeating),
			frequency_min,
			frequency_max,
			hexcodes,
			index: 0,
		}
	}
	fn random_duration(&self) -> Duration {
		let mut rng = rand::thread_rng();
		let duration_range = self.frequency_max - self.frequency_min;
		let random_duration =
			rng.gen_range(0..duration_range.as_millis() as u64);
		self.frequency_min + Duration::from_millis(random_duration)
	}
}



pub fn update_emoji_swapper(
	time: Res<Time>,
	map: Res<EmojiMap>,
	mut query: Populated<(&mut EmojiSwapper, &mut HandleWrapper<Image>)>,
) {
	for (mut swapper, mut handle) in query.iter_mut() {
		println!("swapper: {:?}", swapper);
		if swapper.timer.tick(time.delta()).just_finished() {
			let next_duration = swapper.random_duration();
			swapper.timer.set_duration(next_duration);
			swapper.index = (swapper.index + 1) % swapper.hexcodes.len();
			**handle =
				map.get(&swapper.hexcodes[swapper.index]).unwrap().clone();
		}
	}
}
