//! Extension methods for Bevy plugins.

use crate::prelude::*;
use bevy::app::Plugins;

/// Extension trait adding test helper methods to plugins.
#[extend::ext(name=PluginsExt)]
pub impl<T, M> T
where
	T: Plugins<M>,
{
	/// Creates a [`World`] with this plugin applied.
	fn into_world(self) -> World {
		let mut app = App::new();
		app.add_plugins(self);
		std::mem::take(app.world_mut())
	}
	/// Creates a [`World`] with the `Default` instance of this plugin applied.
	fn world() -> World
	where
		T: Default,
	{
		T::default().into_world()
	}
}
