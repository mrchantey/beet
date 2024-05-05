use crate::prelude::*;
use anyhow::Result;
use beet::prelude::*;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use std::marker::PhantomData;

pub struct DomSim<T: BeetModule> {
	pub phantom: PhantomData<T>,
}

impl<T: BeetModule> Default for DomSim<T> {
	fn default() -> Self {
		Self {
			phantom: PhantomData,
		}
	}
}

impl<T: BeetModule> DomSim<T> {}


impl<T: BeetModule> Plugin for DomSim<T> {
	fn build(&self, app: &mut App) {
		let (send, recv) = flume::unbounded();

		app /*-*/
			.add_plugins(TimePlugin)
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
				update_dom_text.run_if(has_renderer),
				despawn_elements.run_if(has_renderer),
				)
				.chain()
				.after(PostTickSet)
			)
		/*-*/;
	}
}

fn log_error(val: In<Result<()>>) {
	if let Err(e) = val.0 {
		log::error!("{e}");
	}
}


pub fn run_render_systems_once(world: &mut World) {
	if has_renderer(world) {
		world.run_system_once(create_elements);
		world.run_system_once(update_positions);
		world.run_system_once(update_dom_text);
		world.run_system_once(despawn_elements);
	}
}
