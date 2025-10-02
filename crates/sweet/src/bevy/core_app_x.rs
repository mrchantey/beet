use bevy::prelude::*;
use extend::ext;
use std::time::Duration;


#[ext(name=CoreAppExtSweet)]
/// Ease-of-use extensions for `bevy::App`
pub impl App {
	/// Insert a [Time] resource, useful for testing without [`MinimalPlugins`]
	fn insert_time(&mut self) -> &mut Self {
		self.insert_resource::<Time>(Time::default());
		self
	}
	/// Advance time then update.
	/// Note: Using this method with [`MinimalPlugins`] or other time management
	/// systems will produce unexpected results.
	fn update_with_duration(&mut self, duration: Duration) -> &mut Self {
		self.world_mut().resource_mut::<Time>().advance_by(duration);
		self.update();
		// reset the delta etc
		self.world_mut()
			.resource_mut::<Time>()
			.advance_by(Duration::ZERO);
		self
	}
	/// Advance time then update.
	/// Note: Using this method with [`MinimalPlugins`] or other time management
	/// systems will produce unexpected results.
	fn update_with_secs(&mut self, secs: u64) -> &mut Self {
		self.update_with_duration(Duration::from_secs(secs))
	}
	/// Advance time then update.
	/// Note: Using this method with [`MinimalPlugins`] or other time management
	/// systems will produce unexpected results.
	fn update_with_millis(&mut self, millis: u64) -> &mut Self {
		self.update_with_duration(Duration::from_millis(millis))
	}
	/// Method chaining utility, calls `update` and returns `self`.
	fn update_then(&mut self) -> &mut Self {
		self.update();
		self
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;

	#[derive(Default, Resource)]
	struct Foo(Vec<f32>);

	#[test]
	fn time() {
		let mut app = App::new();
		app.init_resource::<Foo>().insert_time().add_systems(
			Update,
			|time: Res<Time>, mut foo: ResMut<Foo>| {
				foo.0.push(time.delta_secs());
			},
		);
		app.update();
		app.update_with_millis(10);
		app.world_mut()
			.resource::<Time>()
			.delta_secs()
			.xpect_eq(0.0);
		app.update_with_secs(10);
		app.update();
		app.world_mut()
			.resource::<Foo>()
			.0
			.xpect_eq(vec![0., 0.01, 10., 0.]);
	}
}
