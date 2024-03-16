use bevy::prelude::*;
use parking_lot::RwLock;
use std::sync::Arc;

type CommandRequest = Box<dyn 'static + Send + Sync + FnOnce(&mut Commands)>;

#[derive(Clone, Resource, Deref, DerefMut)]
pub struct CommandHandler(pub Arc<RwLock<Vec<CommandRequest>>>);

impl Default for CommandHandler {
	fn default() -> Self { Self(default()) }
}

impl CommandHandler {
	pub fn new() -> Self { Self::default() }

	pub fn push(
		&self,
		func: impl 'static + Send + Sync + FnOnce(&mut Commands),
	) {
		self.write().push(Box::new(func));
	}

	pub fn system(mut commands: Commands, spawn_handler: Res<CommandHandler>) {
		let mut handlers = spawn_handler.write();
		let handlers: &mut Vec<CommandRequest> = handlers.as_mut();
		let funcs = std::mem::take(handlers);
		for func in funcs.into_iter() {
			func(&mut commands);
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
		app.add_systems(PreUpdate, CommandHandler::system);
		let handler = CommandHandler::default();
		app.insert_resource(handler.clone());

		let val = mock_value();

		let val2 = val.clone();
		handler
			.push(move |commands| val2.push(commands.spawn(MyStruct(8)).id()));

		app.update();

		let entity = val.pop().unwrap();
		expect(&app).component(entity)?.to_be(&MyStruct(8))?;

		Ok(())
	}
}
