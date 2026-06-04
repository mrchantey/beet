use super::*;
use crate::parse::PostParseTree;
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
		// resolve styles + decorations and drive the layout/paint pipeline
		world.run_schedule(PostParseTree);
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

	/// Spawn `bundle` under a fresh flex buffer, then run the [`PostParseTree`]
	/// pipeline to resolve styles, decorate, lay out, and paint the tree.
	fn populate(width: u32, bundle: impl Bundle) -> Self {
		let mut world = CharcellPlugin::world();
		let entity = world.spawn((FlexBuffer::new(width), bundle)).id();
		world.run_schedule(PostParseTree);
		world.entity_mut(entity).take::<FlexBuffer>().unwrap()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::prelude::*;

	#[beet_core::test]
	fn post_parse_tree_drives_pipeline() {
		let mut world = CharcellPlugin::world();
		let root = world
			.spawn((
				Buffer::new(UVec2::new(20, 5)).into_double_buffer(),
				rsx_direct! { <div><h1>"Title"</h1><p>"Body"</p></div> },
			))
			.id();
		// the schedule resolves styles, decorates, lays out, and paints
		world.run_schedule(PostParseTree);

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
	fn non_visual_tags_skipped_with_material() {
		// beet_site composes CharcellPlugin (which brings StylePlugin) plus
		// MaterialStylePlugin, whose rule set replaces the one StylePlugin seeds.
		// The user-agent `non_visual_rule` must survive that replace so
		// <head>/<style> resolve to display:none and never paint into the terminal.
		let mut world = (
			CharcellPlugin,
			crate::style::material::MaterialStylePlugin::default(),
		)
			.into_world();
		let entity = world
			.spawn((
				Buffer::new(UVec2::new(40, 6)).into_double_buffer(),
				rsx_direct! {
					<div>
						<head><style>"body { color: red; }"</style></head>
						<p>"Visible"</p>
					</div>
				},
			))
			.id();
		world.run_schedule(PostParseTree);
		world
			.get::<DoubleBuffer>(entity)
			.unwrap()
			.current_buffer()
			.render_plain()
			.xpect_contains("Visible")
			.xnot()
			.xpect_contains("color: red");
	}

	#[beet_core::test]
	fn auto_grow_oneshot_is_unbounded() {
		// 30 preformatted lines exceed a typical short buffer; auto-grow keeps
		// them all instead of clipping to a fixed height.
		let text = (0..30)
			.map(|i| i.to_string())
			.collect::<Vec<_>>()
			.join("\n");
		let out =
			FlexBuffer::render_oneshot(20, rsx_direct! { <pre>{text}</pre> });
		out.lines().count().xpect_eq(30);
	}

	#[beet_core::test]
	fn paints_render_ref_content() {
		// a RenderRef holder is transparent: the charcell pipeline measures,
		// lays out, and paints the referenced entity in the holder's place,
		// without it being parented under the buffer tree.
		let mut world = CharcellPlugin::world();
		let content = world.spawn(rsx_direct! { <p>"transcluded"</p> }).id();
		let root = world
			.spawn((FlexBuffer::new(40), children![(RenderRef::new(content),)]))
			.id();
		world.run_schedule(PostParseTree);
		world
			.entity_mut(root)
			.take::<FlexBuffer>()
			.unwrap()
			.render_plain()
			.xpect_contains("transcluded");
	}

	#[beet_core::test]
	fn anchor_emits_osc8_link() {
		// `apply_hyperlinks` promotes the `<a href>` to a `Hyperlink`, which the
		// inline flow wraps around the link's painted columns as OSC-8.
		let out = FlexBuffer::render_oneshot(
			40,
			rsx_direct! { <p>"See "<a href="https://beet.org">"the docs"</a>"."</p> },
		);
		out.as_str()
			.xpect_contains("\x1b]8;;https://beet.org\x1b\\")
			.xpect_contains("the docs")
			.xpect_contains("\x1b]8;;\x1b\\");
	}
}
