use bevy::prelude::*;
use parking_lot::RwLock;
use std::sync::Arc;

type WorldRequest = Box<dyn 'static + Send + Sync + FnOnce(&mut World)>;

#[derive(Clone, Resource, Deref, DerefMut)]
pub struct WorldHandler(pub Arc<RwLock<Vec<WorldRequest>>>);

impl Default for WorldHandler {
	fn default() -> Self { Self(default()) }
}

impl WorldHandler {
	pub fn new() -> Self { Self::default() }

	pub fn push(&self, func: impl 'static + Send + Sync + FnOnce(&mut World)) {
		self.write().push(Box::new(func));
	}

	pub fn system(world: &mut World) {
		let handlers = world.resource_mut::<WorldHandler>();
		let mut handlers = handlers.write();
		let funcs: &mut Vec<WorldRequest> = handlers.as_mut();
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
