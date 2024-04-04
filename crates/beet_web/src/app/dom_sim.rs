use crate::prelude::*;
use anyhow::Result;
use beet::prelude::*;
use bevy::prelude::*;
use forky_web::SearchParams;
use std::marker::PhantomData;
use std::time::Duration;

pub struct DomSim<T: ActionList> {
	pub auto_flowers: Option<Duration>,
	pub bees: usize,
	pub flowers: usize,
	pub phantom: PhantomData<T>,
}

impl<T: ActionList> Default for DomSim<T> {
	fn default() -> Self {
		Self {
			auto_flowers: None,
			// test_container: None,
			bees: 1,
			flowers: 1,
			phantom: PhantomData,
		}
	}
}

impl<T: ActionList> DomSim<T> {
	pub fn with_url_params(mut self) -> Self {
		if let Some(bees) = SearchParams::get("bees") {
			self.bees = bees.parse().unwrap_or(1);
		}
		if let Some(flowers) = SearchParams::get("flowers") {
			self.flowers = flowers.parse().unwrap_or(1);
		}
		if let Some(auto_flowers) = SearchParams::get("auto-flowers") {
			let val: f64 = auto_flowers.parse().unwrap_or(1.0);
			self.auto_flowers = Some(Duration::from_secs_f64(val));
		}
		self
	}
}


impl<T: ActionList> Plugin for DomSim<T> {
	fn build(&self, app: &mut App) {
		let (send, recv) = flume::unbounded();

		app /*-*/
			.add_plugins(BeetMinimalPlugin)
			.add_plugins(DefaultBeetPlugins::<T>::new())
			.insert_resource(DomSimMessageSend(send.clone()))
			.insert_resource(DomSimMessageRecv(recv))
			.add_systems(Update,(
				message_handler.pipe(log_error),
				create_elements.run_if(has_renderer),
				)
				.chain()
				.before(PreTickSet)
			)
			.add_systems(Update,(
				update_positions.run_if(has_renderer),
				despawn_elements.run_if(has_renderer),
				)
				.chain()
				.after(PostTickSet)
			)
		/*-*/;

		if let Some(duration) = self.auto_flowers {
			flower_auto_spawn_with_duration(&mut app.world_mut() , duration);
		}

		for _ in 0..self.bees {
			spawn_bee(&mut app.world_mut());
		}
		for _ in 0..self.flowers {
			spawn_flower(&mut app.world_mut());
		}
	}
}

fn log_error(val: In<Result<()>>) {
	if let Err(e) = val.0 {
		log::error!("{e}");
	}
}
