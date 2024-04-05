use crate::prelude::*;
use anyhow::Result;
use beet::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;

pub struct DomSim<T: ActionList> {
	pub phantom: PhantomData<T>,
}

impl<T: ActionList> Default for DomSim<T> {
	fn default() -> Self {
		Self {
			phantom: PhantomData,
		}
	}
}

impl<T: ActionList> DomSim<T> {}


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
	}
}

fn log_error(val: In<Result<()>>) {
	if let Err(e) = val.0 {
		log::error!("{e}");
	}
}
