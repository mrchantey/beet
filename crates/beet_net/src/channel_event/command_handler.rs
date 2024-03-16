use crate::prelude::*;
use bevy::prelude::*;
use parking_lot::RwLock;
use std::sync::Arc;

type CommandRequest<T> = Box<dyn Send + FnOnce(&mut Commands) -> T>;
pub type CommandChannels<T> = Vec<ResponseChannel<CommandRequest<T>, T>>;

#[derive(Clone, Resource, Deref, DerefMut)]
pub struct CommandHandler<T>(pub Arc<RwLock<CommandChannels<T>>>);

impl<T> Default for CommandHandler<T> {
	fn default() -> Self { Self(default()) }
}

impl<T: 'static + Send> CommandHandler<T> {
	pub fn new() -> Self { Self::default() }

	pub fn add(&self) -> RequestChannel<CommandRequest<T>, T> {
		let (req, res) = reqres_channel();
		self.write().push(res);
		req
	}

	pub fn system(
		mut commands: Commands,
		spawn_handler: Res<CommandHandler<T>>,
	) {
		spawn_handler.write().retain_mut(|channel| {
			channel.try_respond(|func| func(&mut commands)).is_ok()
		});
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
		app.add_systems(PreUpdate, CommandHandler::<Entity>::system);
		let handler = CommandHandler::default();
		app.insert_resource(handler.clone());

		let req = handler.add();
		req.start_request(Box::new(|commands| {
			commands.spawn(MyStruct(8)).id()
		}))?;

		app.update();

		let entity = req.block_on_response()?;
		expect(&app).component(entity)?.to_be(&MyStruct(8))?;

		Ok(())
	}
}
