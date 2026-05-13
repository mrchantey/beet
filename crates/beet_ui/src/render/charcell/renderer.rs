use super::*;
use beet_core::prelude::*;
use bevy::math::UVec2;

impl CharcellPlugin {
	/// Render a bundle with the default terminal size, returning ANSI output.
	pub fn render_oneshot(bundle: impl Bundle) -> String {
		Self::render_impl(terminal_ext::size(), bundle, false)
	}

	/// Render a bundle with a custom size, returning ANSI output.
	pub fn render_oneshot_sized(size: UVec2, bundle: impl Bundle) -> String {
		Self::render_impl(size, bundle, false)
	}

	/// Render a bundle with a custom size, returning plain text output.
	pub fn render_oneshot_plain_sized(
		size: UVec2,
		bundle: impl Bundle,
	) -> String {
		Self::render_impl(size, bundle, true)
	}

	fn render_impl(size: UVec2, bundle: impl Bundle, plain: bool) -> String {
		let mut world = CharcellPlugin::world();
		let entity = world.spawn((DoubleBuffer::new(size), bundle)).id();
		world.run_schedule(PostUpdate);
		world
			.entity(entity)
			.get::<DoubleBuffer>()
			.map(|db| {
				if plain {
					db.render_plain()
				} else {
					db.render()
				}
			})
			.unwrap_or_default()
	}
}
