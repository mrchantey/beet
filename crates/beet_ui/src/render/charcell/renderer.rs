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
		let mut world = World::new();
		let entity = world.spawn((DoubleBuffer::new(size), bundle)).id();

		// Insert required layout components directly (avoids Commands flush)
		let all_entities = {
			let mut stack = vec![entity];
			let mut all = Vec::new();
			while let Some(e) = stack.pop() {
				all.push(e);
				if let Some(children) = world.get::<Children>(e) {
					let kids: Vec<Entity> = children.iter().collect();
					stack.extend(kids);
				}
			}
			all
		};
		for e in all_entities {
			if world.get::<IntrinsicSize>(e).is_none() {
				world.entity_mut(e).insert(IntrinsicSize::default());
			}
			if world.get::<LayoutRect>(e).is_none() {
				world.entity_mut(e).insert(LayoutRect::default());
			}
		}

		// Run the render systems in order
		let _: Result<(), _> = world.run_system_once(measure_nodes);
		let _: Result<(), _> = world.run_system_once(layout_nodes);
		let _: Result<(), _> = world.run_system_once(paint_nodes);

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
