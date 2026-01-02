use crate::prelude::*;
use bevy::app::Plugins;


#[extend::ext(name=PluginsExt)]
pub impl<T, M> T
where
	T: Plugins<M>,
{
	/// Test helper to create a [`World`] with this plugin
	fn into_world(self) -> World {
		let mut app = App::new();
		app.add_plugins(self);
		std::mem::take(app.world_mut())
	}
	/// Test helper to create a [`World`] with this plugin's `Default`
	fn world() -> World
	where
		T: Default,
	{
		T::default().into_world()
	}
}
