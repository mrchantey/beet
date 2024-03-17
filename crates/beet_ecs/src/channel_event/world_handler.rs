use bevy::prelude::*;
use parking_lot::RwLock;
use std::sync::Arc;



pub type WorldFuncs = Vec<Box<dyn 'static + Send + Sync + FnOnce(&mut World)>>;


#[derive(Clone, Resource, Deref, DerefMut)]
pub struct WorldHandler(pub Arc<RwLock<WorldFuncs>>);

impl Default for WorldHandler {
	fn default() -> Self { Self(default()) }
}

impl WorldHandler {
	pub fn new() -> Self { Self::default() }


	/// Insert a callback to be called on the next world update
	pub fn push(&self, func: impl 'static + Send + Sync + FnOnce(&mut World)) {
		self.write().push(Box::new(func));
	}

	/// Insert a callback and block on the result
	pub fn request<O: 'static + Send>(
		&self,
		func: impl 'static + Send + Sync + FnOnce(&mut World) -> O,
	) -> O {
		let (send, recv) = flume::unbounded();
		self.write().push(Box::new(move |world| {
			send.send(func(world)).unwrap();
		}));
		recv.recv().unwrap()
	}

	/// Insert a callback and await the result
	pub async fn request_async<O: 'static + Send>(
		&self,
		func: impl 'static + Send + Sync + FnOnce(&mut World) -> O,
	) -> O {
		let (send, recv) = flume::unbounded();
		self.write().push(Box::new(move |world| {
			send.send(func(world)).unwrap();
		}));
		recv.recv_async().await.unwrap()
	}

	pub fn system(world: &mut World) {
		let handlers = world.resource_mut::<WorldHandler>();
		let mut handlers = handlers.write();
		let funcs: &mut WorldFuncs = handlers.as_mut();
		let funcs = std::mem::take(funcs);
		drop(handlers);
		for func in funcs.into_iter() {
			func(world);
		}
	}
}


#[cfg(test)]
mod test {
	use super::*;
	use anyhow::Result;
	use sweet::*;

	#[derive(Debug, PartialEq, Component)]
	struct MyStruct(pub i32);

	#[test]
	fn test_spawn_handler() -> Result<()> {
		let mut app = App::new();
		app.add_systems(PreUpdate, WorldHandler::system);
		let handler = WorldHandler::default();
		app.insert_resource(handler.clone());

		let val = mock_value();

		let val2 = val.clone();
		handler.push(move |world| val2.push(world.spawn(MyStruct(8)).id()));

		app.update();

		let entity = val.pop().unwrap();
		expect(&app).component(entity)?.to_be(&MyStruct(8))?;

		Ok(())
	}

}
