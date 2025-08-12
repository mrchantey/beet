use crate::prelude::*;
use beet_utils::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::SystemId;
use bevy::prelude::*;
use flume::Receiver;
use flume::Sender;


impl<T> IntoBundle<Self> for Getter<T>
where
	T: 'static + Send + Sync + Clone + ToString,
{
	fn into_bundle(self)->impl Bundle {
		OnSpawn::new(move |entity|{
		entity.insert(TextNode::new(self.clone()().to_string()));
		let id = entity.id();
		let sender = entity
			.world_scope(|world| world.resource::<DirtySignals>().sender());
		let func = self.clone();
		effect(move || {
			// subscribe to changes
			let _ = func.clone()();
			// ignore errors if receiver dropped
			sender.send(id).ok();
			// request animation frame
		});
		let system_id = entity.world_scope(move |world| {
			world.register_system(move |mut query: Query<&mut TextNode>| {
				println!("all g");
				if let Ok(mut text) = query.get_mut(id) {
					text.0 = self.clone()().to_string();
				} else {
					// warn?
					warn!(
						"Effect expected an entity with a Text node, none found"
					);
				}
			})
		});

		entity.insert(Effect(system_id));
	})
	}
}


#[derive(Resource)]
pub struct DirtySignals {
	send: Sender<Entity>,
	recv: Receiver<Entity>,
}

impl Default for DirtySignals {
	fn default() -> Self {
		let (send, recv) = flume::unbounded();
		Self { send, recv }
	}
}

impl DirtySignals {
	pub fn sender(&self) -> Sender<Entity> { self.send.clone() }
}



pub fn update_signals(
	mut commands: Commands,
	dirty: ResMut<DirtySignals>,
	effects: Query<&Effect>,
) {
	while let Ok(entity) = dirty.recv.recv() {
		if let Ok(effect) = effects.get(entity) 
		{
			println!("updating signals");
			commands.run_system(effect.0);
		}
	}
}

#[derive(Component)]
struct Effect(SystemId);



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;




	#[test]
	fn works() {
		let mut world = World::default();
		world.init_resource::<DirtySignals>();
		
		let (get, set) = signal(5);
		let entity=	world.spawn(get.into_bundle()).id();
		world.entity(entity).get::<TextNode>().unwrap()
		.0.clone().xpect().to_be("5");
	println!("1");
	world.run_system_cached(update_signals).unwrap();
	println!("2");
	set(7);
	world.run_system_cached(update_signals).unwrap();
	println!("3");
		// world.entity(entity).get::<TextNode>().unwrap()
		// .0.clone().xpect().to_be("7");
	}
}
