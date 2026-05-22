use super::*;
use crate::style::PostParseTree;
use beet_core::prelude::*;
use bevy::ecs::component::Mutable;
use bevy::math::UVec2;

/// Run the charcell pipeline once over every `B` buffer tree in `world`.
///
/// Drives `prepare → measure → layout → paint` directly via
/// [`run_system_cached`](World::run_system_cached), for renderers that own the
/// pipeline manually instead of relying on [`CharcellPlugin`]'s `PostUpdate`
/// schedule. Styles must already be resolved (eg via the [`PostParseTree`]
/// schedule); this only lays out and paints.
pub fn paint_charcell_trees<B: Component<Mutability = Mutable> + AsBuffer>(
	world: &mut World,
) -> Result {
	world.run_system_cached::<(), _, _>(prepare_charcell_tree::<B>)?;
	world.run_system_cached::<(), _, _>(measure_nodes::<B>)?;
	world.run_system_cached::<(), _, _>(layout_nodes::<B>)?;
	world.run_system_cached::<(), _, _>(paint_nodes::<B>)?;
	Ok(())
}

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

impl FlexBuffer {
	/// Render a bundle into an auto-growing buffer of fixed `width`, returning
	/// ANSI output of unbounded height (the stdout rendering path).
	pub fn render_oneshot(width: u32, bundle: impl Bundle) -> String {
		Self::populate(width, bundle).render()
	}

	/// As [`render_oneshot`](Self::render_oneshot) but plain text, no styling.
	pub fn render_oneshot_plain(width: u32, bundle: impl Bundle) -> String {
		Self::populate(width, bundle).render_plain()
	}

	/// Spawn `bundle` under a fresh flex buffer, resolve styles + decorations,
	/// then drive the charcell pipeline and return the painted buffer.
	fn populate(width: u32, bundle: impl Bundle) -> Self {
		let mut world = CharcellPlugin::world();
		let entity = world.spawn((FlexBuffer::new(width), bundle)).id();
		// resolve styles (display rules, syntax highlighting) and decorations
		// before manually driving the layout/paint pipeline over the flex tree.
		world.run_schedule(PostParseTree);
		paint_charcell_trees::<FlexBuffer>(&mut world).unwrap();
		world.entity_mut(entity).take::<FlexBuffer>().unwrap()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::prelude::*;

	#[beet_core::test]
	fn paint_charcell_trees_drives_pipeline() {
		let mut world = CharcellPlugin::world();
		let root = world
			.spawn((
				Buffer::new(UVec2::new(20, 5)).into_double_buffer(),
				rsx! { <div><h1>"Title"</h1><p>"Body"</p></div> },
			))
			.id();
		// resolve styles (display rules) before manually driving layout/paint
		world.run_schedule(PostParseTree);
		paint_charcell_trees::<DoubleBuffer>(&mut world).unwrap();

		let out = world
			.get::<DoubleBuffer>(root)
			.unwrap()
			.current_buffer()
			.render_plain();
		let lines = out
			.lines()
			.map(|line| line.trim_end())
			.filter(|line| !line.is_empty())
			.collect::<Vec<_>>();
		lines.xpect_eq(vec!["Title", "Body"]);
	}

	#[beet_core::test]
	fn auto_grow_oneshot_is_unbounded() {
		// 30 preformatted lines exceed a typical short buffer; auto-grow keeps
		// them all instead of clipping to a fixed height.
		let text =
			(0..30).map(|i| i.to_string()).collect::<Vec<_>>().join("\n");
		let out = FlexBuffer::render_oneshot(20, rsx! { <pre>{text}</pre> });
		out.lines().count().xpect_eq(30);
	}

	#[beet_core::test]
	fn anchor_emits_osc8_link() {
		// `apply_hyperlinks` promotes the `<a href>` to a `Hyperlink`, which the
		// inline flow wraps around the link's painted columns as OSC-8.
		let out = FlexBuffer::render_oneshot(
			40,
			rsx! { <p>"See "<a href="https://beet.org">"the docs"</a>"."</p> },
		);
		out.as_str()
			.xpect_contains("\x1b]8;;https://beet.org\x1b\\")
			.xpect_contains("the docs")
			.xpect_contains("\x1b]8;;\x1b\\");
	}
}
