use super::*;
use beet_core::prelude::*;
use bevy::math::UVec2;

impl Buffer {
	/// Render a bundle with the default terminal size, returning ANSI output.
	pub fn render_oneshot(bundle: impl Bundle) -> String {
		Self::default().populate(bundle).render()
	}
	pub fn render_oneshot_sized(size: UVec2, bundle: impl Bundle) -> String {
		Self::new(size).populate(bundle).render()
	}

	/// Render a bundle with a custom size, returning plain text output.
	pub fn render_oneshot_plain_sized(
		size: UVec2,
		bundle: impl Bundle,
	) -> String {
		Self::new(size).populate(bundle).render_plain()
	}

	pub fn populate(self, bundle: impl Bundle) -> Self {
		let mut world = CharcellPlugin::world();
		let entity = world.spawn((self.into_double_buffer(), bundle)).id();
		world.run_schedule(PostUpdate);
		world
			.entity_mut(entity)
			.take::<DoubleBuffer>()
			.unwrap()
			.into_buffer()
	}
}
