use bevy::prelude::*;
use extend::ext;
use std::time::Duration;


#[ext(name=CoreAppExtSweet)]
/// Ease-of-use extensions for `bevy::App`
pub impl App {
	/// Insert a [Time] resource
	///
	fn insert_time(&mut self) -> &mut Self {
		self.insert_resource::<Time>(Time::default());
		self
	}
	/// Advance time *then* update.
	fn update_with_duration(&mut self, duration: Duration) -> &mut Self {
		let mut time = self.world_mut().resource_mut::<Time>();
		time.advance_by(duration);
		self.update();
		self
	}
	/// Advance time *then* update.
	fn update_with_secs(&mut self, secs: u64) -> &mut Self {
		self.update_with_duration(Duration::from_secs(secs))
	}
	/// Advance time *then* update.
	fn update_with_millis(&mut self, millis: u64) -> &mut Self {
		self.update_with_duration(Duration::from_millis(millis))
	}
}
