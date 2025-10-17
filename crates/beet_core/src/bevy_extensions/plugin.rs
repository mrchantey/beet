use bevy::app::Plugins;

use crate::prelude::*;


#[extend::ext(name=PluginsExt)]
pub impl<T, M> T
where
	T: Plugins<M>,
{
	/// Create a [`World`] with this plugin
	fn into_world(self) -> World {
		let mut app = App::new();
		app.add_plugins(self);
		std::mem::take(app.world_mut())
	}
	/// Create a [`World`] with this plugin's `Default`
	fn world() -> World
	where
		T: Default,
	{
		let mut app = App::new();
		app.add_plugins(T::default());
		std::mem::take(app.world_mut())
	}
}
