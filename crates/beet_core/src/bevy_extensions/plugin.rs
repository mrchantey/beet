use crate::prelude::*;


#[extend::ext]
pub impl<T> T
where
	T: Plugin,
{
	/// Create a [`World`] with this plugin
	fn into_world(self) -> World {
		let mut app = App::new();
		app.add_plugins(self);
		std::mem::take(app.world_mut())
	}
	/// Create a [`World`] with this plugins `Default`
	fn world() -> World
	where
		T: Default,
	{
		let mut app = App::new();
		app.add_plugins(T::default());
		std::mem::take(app.world_mut())
	}
}
