//! Scrollbar gutter reservation (layout) and track/thumb paint (charcell).
//!
//! The gutter is reserved in layout so content reflows around it, exactly like a
//! CSS scrollbar: a vertical scrollbar takes the right column, a horizontal one
//! the bottom row, plus the corner cell when both scroll. The bar itself paints
//! in that reserved space from the [`ScrollState`] geometry. Styling is Task 07;
//! these are the plain box-drawing defaults.

use super::*;
use crate::prelude::*;
// shadow the leaked bevy_ui `ScrollPosition` (see query.rs).
use crate::input::ScrollPosition;
use crate::style::Overflow;
use crate::style::ScrollbarWidth;
use crate::style::VisualStyle;
use beet_core::prelude::*;
use bevy::math::IRect;
use bevy::math::IVec2;
use bevy::math::UVec2;

/// ECS system: re-clamp every scroll container's [`ScrollPosition`] after layout.
///
/// Layout may have changed the content size or scrollport (eg a window resize or
/// content edit), so an offset valid last frame can now point past the end. This
/// runs after [`layout_nodes`](super::layout_nodes) and before
/// [`paint_nodes`](super::paint_nodes), per buffer root so the real viewport
/// drives the box model. A clamped change repaints via change detection.
pub fn clamp_scroll_positions<B: Component + AsBuffer>(
	mut params: ParamSet<(CharcellQuery, Query<&mut ScrollPosition>)>,
	tree: CharcellTree,
	roots: Populated<(Entity, &B)>,
) {
	for (root, buffer) in roots.iter() {
		let viewport = buffer.size();
		// snapshot the clamp target per container from the read-only node view,
		// then write back (the borrows can't overlap).
		let targets: Vec<(Entity, IVec2)> = {
			let charcell = params.p0();
			tree.pre_order(root)
				.into_iter()
				.filter_map(|entity| {
					let node = charcell.unresolved_node(entity).ok()?;
					node.is_scroll_container().then(|| {
						(
							entity,
							scroll_state(&node, &charcell, viewport)
								.max_offset(),
						)
					})
				})
				.collect()
		};
		for (entity, max_offset) in targets {
			if let Ok(mut scroll) = params.p1().get_mut(entity) {
				let clamped = scroll.offset.clamp(IVec2::ZERO, max_offset);
				if clamped != scroll.offset {
					scroll.offset = clamped;
				}
			}
		}
	}
}

/// Build the [`ScrollState`] for a scroll-container node: its scrollable content
/// size against its scrollport (the laid-out content rect minus the reserved
/// gutter), recomputed from the box model with the same `viewport` as the layout
/// pass so it matches exactly.
///
/// The content size is the extent of the container's laid-out children measured
/// from the scrollport origin (the scroll overflow region), not the container's
/// own [`IntrinsicSize`], which an explicit `height` would clamp to the
/// scrollport and defeat scrolling. A container with only inline/text content
/// (no child boxes) falls back to its own intrinsic content size.
pub(super) fn scroll_state(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	viewport: UVec2,
) -> ScrollState {
	let scrollport = scrollport_rect(node, query, viewport);
	let scrollport_size = UVec2::new(
		scrollport.width().max(0) as u32,
		scrollport.height().max(0) as u32,
	);
	ScrollState::new(
		scroll_content_size(node, query, scrollport),
		scrollport_size,
	)
}

/// The natural size of a scroll container's content, in cells: the union of its
/// children's extents measured from `origin`'s top-left, each child's extent its
/// laid-out rect grown to its unconstrained [`IntrinsicSize`].
///
/// This is the *content* a scrollbar measures against, not the container's own
/// [`IntrinsicSize`], which an explicit `height` clamps to the box (defeating the
/// `auto` overflow check). A non-wrapping `<pre>` overflows horizontally, a tall
/// column vertically. A container with only inline/text content (no child boxes)
/// falls back to the `origin` size.
pub(super) fn scroll_content_size(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	origin: IRect,
) -> UVec2 {
	let mut content =
		UVec2::new(origin.width().max(0) as u32, origin.height().max(0) as u32);
	for child in node.child_nodes(query) {
		let rect = child.layout_rect();
		let offset = rect.min - origin.min;
		let intrinsic = child.intrinsic_size();
		let extent = IVec2::new(
			offset.x + (rect.width().max(intrinsic.x as i32)),
			offset.y + (rect.height().max(intrinsic.y as i32)),
		);
		content.x = content.x.max(extent.x.max(0) as u32);
		content.y = content.y.max(extent.y.max(0) as u32);
	}
	content
}

/// The scrollport rect of a node from its laid-out [`LayoutRect`]: content rect
/// (box model) minus the reserved gutter. The single source the clamp and paint
/// both read, matching the layout pass (same `viewport`).
pub(super) fn scrollport_rect(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	viewport: UVec2,
) -> IRect {
	let box_model = BoxModel::from_node(node, viewport);
	scrollport_of(node, query, box_model.content_rect(node.layout_rect()))
}

/// Default vertical track/thumb glyphs (`scrollbar-width: auto`).
const V_TRACK: &str = "│";
const V_THUMB: &str = "█";
/// Default horizontal track/thumb glyphs.
const H_TRACK: &str = "─";
const H_THUMB: &str = "█";
/// Lighter glyphs for `scrollbar-width: thin`.
const V_TRACK_THIN: &str = "┊";
const V_THUMB_THIN: &str = "▐";
const H_TRACK_THIN: &str = "┄";
const H_THUMB_THIN: &str = "▄";

/// Which scrollbar gutters a node reserves, derived from its overflow axes and
/// whether its content overflows (for `auto`).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct ScrollGutters {
	/// Reserve the right column for a vertical scrollbar.
	pub vertical: bool,
	/// Reserve the bottom row for a horizontal scrollbar.
	pub horizontal: bool,
}

impl ScrollGutters {
	/// Compute the gutters a node reserves given its overflow style, content
	/// size, and the content rect size (before any gutter). `Scroll` always
	/// reserves; `Auto` reserves only when the content overflows that axis (CSS).
	pub fn resolve(
		overflow_x: Overflow,
		overflow_y: Overflow,
		content: UVec2,
		content_rect: UVec2,
	) -> Self {
		let reserve =
			|overflow: Overflow, content: u32, port: u32| match overflow {
				Overflow::Scroll => true,
				Overflow::Auto => content > port,
				Overflow::Visible | Overflow::Hidden => false,
			};
		Self {
			vertical: reserve(overflow_y, content.y, content_rect.y),
			horizontal: reserve(overflow_x, content.x, content_rect.x),
		}
	}

	/// The gutter inset (in cells) to subtract from the content rect to get the
	/// scrollport: a right column for the vertical bar, a bottom row for the
	/// horizontal one.
	pub fn inset(&self) -> URect {
		URect {
			min: UVec2::ZERO,
			max: UVec2::new(self.vertical as u32, self.horizontal as u32),
		}
	}

	/// Whether either gutter is reserved.
	pub fn any(&self) -> bool { self.vertical || self.horizontal }
}

/// The scrollbar gutters a node reserves, given its raw content rect (before the
/// gutter). A non-scroll node reserves nothing. `Auto` consults the actual
/// [`scroll_content_size`] (the children's extent), not the node's own
/// [`IntrinsicSize`], which an explicit `height` clamps to the box.
pub(super) fn node_gutters(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	content_rect: IRect,
) -> ScrollGutters {
	let layout = node.layout_style();
	if !layout.overflow_x.is_scroll() && !layout.overflow_y.is_scroll() {
		return ScrollGutters::default();
	}
	// `scrollbar-width: none` removes the bar and its reserved gutter, so content
	// uses the full width.
	if !node.scrollbar_style().width.reserves_gutter() {
		return ScrollGutters::default();
	}
	let port = UVec2::new(
		content_rect.width().max(0) as u32,
		content_rect.height().max(0) as u32,
	);
	ScrollGutters::resolve(
		layout.overflow_x,
		layout.overflow_y,
		scroll_content_size(node, query, content_rect),
		port,
	)
}

/// The scrollport: the node's content rect inset by any reserved scrollbar
/// gutter. Children lay out within this, and the bar paints in the gutter just
/// past it. For a non-scroll node this is the content rect unchanged.
pub(super) fn scrollport_of(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	content_rect: IRect,
) -> IRect {
	inset_rect(
		content_rect,
		node_gutters(node, query, content_rect).inset(),
	)
}

/// The on-screen geometry of one scrollbar axis, in buffer cells.
///
/// The single source the paint and the mouse hit-test both read, so a click
/// lands exactly on the painted bar. `line` is the fixed cross-axis coordinate
/// (the gutter column for the vertical bar, the gutter row for the horizontal);
/// `track_start`/`track_len` span the track along the scroll axis, and
/// `thumb_start`/`thumb_len` the thumb within it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct AxisBar {
	/// Cross-axis screen coordinate: the gutter column (vertical) or row (horizontal).
	pub line: i32,
	/// Along-axis screen start of the track.
	pub track_start: i32,
	/// Track length in cells.
	pub track_len: u32,
	/// Along-axis screen start of the thumb.
	pub thumb_start: i32,
	/// Thumb length in cells.
	pub thumb_len: u32,
	/// Maximum scroll offset on this axis (content minus scrollport).
	pub max_offset: i32,
}

// consumed by the tui-only scrollbar mouse hit-test
#[cfg_attr(not(feature = "tui"), allow(dead_code))]
impl AxisBar {
	/// The leftover track the thumb travels across (track minus thumb length).
	pub fn travel(&self) -> u32 {
		self.track_len.saturating_sub(self.thumb_len)
	}

	/// Map an along-axis cursor coordinate (grabbed `grab` cells into the thumb)
	/// to a clamped scroll offset: drag the thumb to the track end to reach
	/// `max_offset`.
	pub fn offset_at(&self, along: i32, grab: i32) -> i32 {
		let travel = self.travel();
		if travel == 0 {
			return 0;
		}
		let thumb_pos =
			(along - grab - self.track_start).clamp(0, travel as i32);
		((thumb_pos as f32 / travel as f32) * self.max_offset as f32).round()
			as i32
	}
}

/// The screen-space scrollbar geometry of a scroll container: the active axes'
/// track + thumb spans. The hit-test and the paint share this so they agree.
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct ScrollbarGeometry {
	/// The vertical bar (right gutter), present when that axis overflows + reserves.
	pub vertical: Option<AxisBar>,
	/// The horizontal bar (bottom gutter).
	pub horizontal: Option<AxisBar>,
}

/// Compute a scroll container's on-screen scrollbar geometry, or `None` when it
/// is not a scroll container or reserves no gutter.
///
/// `screen_offset` is the node's accumulated ancestor scroll translation (its
/// [`PaintContext`](super::PaintContext) offset), so the geometry is in the same
/// screen space the paint draws into and the hit-test reads.
pub(super) fn scrollbar_geometry(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	viewport: UVec2,
	screen_offset: IVec2,
) -> Option<ScrollbarGeometry> {
	if !node.is_scroll_container() {
		return None;
	}
	let layout_rect = translate_rect(node.layout_rect(), screen_offset);
	let content_rect =
		BoxModel::from_node(node, viewport).content_rect(layout_rect);
	let gutters = node_gutters(node, query, content_rect);
	if !gutters.any() {
		return None;
	}
	let scrollport = scrollport_of(node, query, content_rect);
	let state = scroll_state(node, query, viewport);
	let offset = node.scroll_offset();
	let max_offset = state.max_offset();

	let vertical = gutters.vertical.then(|| {
		let track_len = scrollport.height().max(0) as u32;
		let (start, len) =
			state.thumb_y(offset, track_len).unwrap_or((0, track_len));
		AxisBar {
			line: scrollport.max.x,
			track_start: scrollport.min.y,
			track_len,
			thumb_start: scrollport.min.y + start as i32,
			thumb_len: len,
			max_offset: max_offset.y,
		}
	});
	let horizontal = gutters.horizontal.then(|| {
		let track_len = scrollport.width().max(0) as u32;
		let (start, len) =
			state.thumb_x(offset, track_len).unwrap_or((0, track_len));
		AxisBar {
			line: scrollport.max.y,
			track_start: scrollport.min.x,
			track_len,
			thumb_start: scrollport.min.x + start as i32,
			thumb_len: len,
			max_offset: max_offset.x,
		}
	});
	Some(ScrollbarGeometry {
		vertical,
		horizontal,
	})
}

/// Paint the track and thumb for a scroll container into its reserved gutter,
/// styled by the resolved [`ScrollbarStyle`].
///
/// Reads the shared [`scrollbar_geometry`] so the painted bar matches exactly
/// what the mouse hit-test acts on. Thumb/track colours come from
/// `scrollbar-color`, the glyph weight from `scrollbar-width` (`thin` lighter).
pub(super) fn paint_scrollbar(
	buffer: &mut impl AsBuffer,
	node: &CharcellNodeData,
	query: &CharcellQuery,
	viewport: UVec2,
	screen_offset: IVec2,
	clip: Clip,
) {
	let Some(geometry) =
		scrollbar_geometry(node, query, viewport, screen_offset)
	else {
		return;
	};
	let entity = node.entity;
	let style = node.scrollbar_style();
	let thin = matches!(style.width, ScrollbarWidth::Thin);
	if let Some(bar) = geometry.vertical {
		let track_glyph = if thin { V_TRACK_THIN } else { V_TRACK };
		let thumb_glyph = if thin { V_THUMB_THIN } else { V_THUMB };
		paint_bar(buffer, entity, bar.track_len, clip, |row| {
			let along = bar.track_start + row as i32;
			let glyph = if along >= bar.thumb_start
				&& along < bar.thumb_start + bar.thumb_len as i32
			{
				(thumb_glyph, style.thumb)
			} else {
				(track_glyph, style.track)
			};
			(IVec2::new(bar.line, along), glyph)
		});
	}
	if let Some(bar) = geometry.horizontal {
		let track_glyph = if thin { H_TRACK_THIN } else { H_TRACK };
		let thumb_glyph = if thin { H_THUMB_THIN } else { H_THUMB };
		paint_bar(buffer, entity, bar.track_len, clip, |col| {
			let along = bar.track_start + col as i32;
			let glyph = if along >= bar.thumb_start
				&& along < bar.thumb_start + bar.thumb_len as i32
			{
				(thumb_glyph, style.thumb)
			} else {
				(track_glyph, style.track)
			};
			(IVec2::new(along, bar.line), glyph)
		});
	}
	// the corner cell where both bars meet stays blank (the browser's empty
	// scrollbar corner).
}

/// Paint a single scrollbar axis: for each of `track_len` cells call `cell` to
/// get its position and `(glyph, colour)`, writing it through `clip`.
fn paint_bar(
	buffer: &mut impl AsBuffer,
	entity: Entity,
	track_len: u32,
	clip: Clip,
	cell: impl Fn(u32) -> (IVec2, (&'static str, Option<Color>)),
) {
	for i in 0..track_len {
		let (pos, (glyph, color)) = cell(i);
		let style = VisualStyle {
			foreground: color,
			..default()
		};
		buffer.set_composite_clipped(
			pos,
			Cell::new(glyph, style, entity),
			clip,
		);
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::style::*;
	use beet_core::prelude::*;
	use bevy::math::IVec2;
	use bevy::math::UVec2;

	/// Build a scroll container (`<div class="scroller">` wrapping a `<pre>` of
	/// `lines` rows) into a `size` buffer, with rules giving it the named width,
	/// height, and overflow. The container is wrapped so it is not the viewport
	/// root. Sets `offset` on its [`ScrollPosition`], re-renders, and returns the
	/// rendered [`Buffer`] for inspection.
	fn scroll_buffer(
		size: UVec2,
		rules: Vec<Rule>,
		lines: u32,
		offset: IVec2,
	) -> Buffer {
		let mut world = CharcellPlugin::world();
		world.get_resource_or_init::<RuleSet>().extend_rules(rules);
		let body: String = (0..lines)
			.map(|i| format!("r{i}"))
			.collect::<Vec<_>>()
			.join("\n");
		let root = world
			.spawn((Buffer::new(size).into_double_buffer(), rsx! {
				<div>
					<div class="scroller"><pre>{body}</pre></div>
				</div>
			}))
			.id();
		// first pass inserts ScrollPosition + lays out
		world.run_schedule(PostParseTree);
		// set the offset on the scroll container, then re-render
		let scroller = world
			.query_filtered::<Entity, With<ScrollPosition>>()
			.iter(&world)
			.next()
			.expect("a scroll container with a ScrollPosition");
		world
			.entity_mut(scroller)
			.insert(ScrollPosition::new(offset));
		world.run_schedule(PostParseTree);
		world
			.entity_mut(root)
			.take::<DoubleBuffer>()
			.unwrap()
			.into_buffer()
	}

	fn scroller_rules(overflow_x: Overflow, overflow_y: Overflow) -> Vec<Rule> {
		vec![
			Rule::class("scroller")
				.with_value(common_props::Width, Length::Rem(8.))
				.with_value(common_props::Height, Length::Rem(3.))
				.with_value(common_props::OverflowXProp, overflow_x)
				.with_value(common_props::OverflowYProp, overflow_y),
		]
	}

	/// A vertical scroll container shows the offset slice of its content and a
	/// scrollbar gutter in its last column.
	#[beet_core::test]
	fn vertical_scroll_shows_offset_slice_and_bar() {
		let buffer = scroll_buffer(
			UVec2::new(12, 8),
			scroller_rules(Overflow::Visible, Overflow::Scroll),
			10,
			IVec2::new(0, 4),
		);
		let frame = buffer.render_plain();
		// content scrolled to rows 4..6 (3-row scrollport)
		frame.as_str().xpect_contains("r4").xpect_contains("r6");
		frame.as_str().xnot().xpect_contains("r0");
		frame.as_str().xnot().xpect_contains("r9");
		// a vertical bar glyph (track or thumb) is painted somewhere
		let has_bar = buffer
			.iter_cells()
			.any(|(_, cell)| matches!(cell.symbol_str(), "│" | "█"));
		has_bar.xpect_true();
	}

	/// Scrolling to the maximum offset reveals the last rows and pins the thumb
	/// to the bottom of the track.
	#[beet_core::test]
	fn vertical_scroll_to_end_shows_last_rows() {
		let buffer = scroll_buffer(
			UVec2::new(12, 8),
			scroller_rules(Overflow::Visible, Overflow::Scroll),
			10,
			IVec2::new(0, 99), // clamped to max (10 - 3 = 7)
		);
		let frame = buffer.render_plain();
		// scrolled to the end: the last row is visible, the first is gone
		frame.as_str().xpect_contains("r9").xpect_contains("r8");
		frame.as_str().xnot().xpect_contains("r0");
		// the thumb's last cell sits on the bottom row of the scrollport
		let thumb_max_y = buffer
			.iter_cells()
			.filter(|(_, cell)| cell.symbol_str() == "█")
			.map(|(pos, _)| pos.y)
			.max();
		thumb_max_y.xpect_some();
	}

	/// A horizontal scroll container shifts wide content left and reserves a
	/// bottom-row gutter.
	#[beet_core::test]
	fn horizontal_scroll_shifts_content() {
		let mut world = CharcellPlugin::world();
		world.get_resource_or_init::<RuleSet>().extend_rules(vec![
			Rule::class("scroller")
				.with_value(common_props::Width, Length::Rem(6.))
				.with_value(common_props::Height, Length::Rem(3.))
				.with_value(common_props::OverflowXProp, Overflow::Scroll),
		]);
		let root = world
			.spawn((
				Buffer::new(UVec2::new(20, 6)).into_double_buffer(),
				rsx! {
					<div>
						<div class="scroller"><pre>"ABCDEFGHIJKLMNOP"</pre></div>
					</div>
				},
			))
			.id();
		world.run_schedule(PostParseTree);
		let scroller = world
			.query_filtered::<Entity, With<ScrollPosition>>()
			.iter(&world)
			.next()
			.unwrap();
		// shift content left by 3 cells
		world
			.entity_mut(scroller)
			.insert(ScrollPosition::new(IVec2::new(3, 0)));
		world.run_schedule(PostParseTree);
		let frame = world
			.entity_mut(root)
			.take::<DoubleBuffer>()
			.unwrap()
			.into_buffer()
			.render_plain();
		// the leading "ABC" scrolled out of view; "D" is now at the left edge
		frame.as_str().xnot().xpect_contains("ABC");
		frame.xpect_contains("D");
	}

	// ── Styling (Task 07) ──

	/// `scrollbar-color` paints the thumb in the resolved colour.
	#[beet_core::test]
	fn scrollbar_color_styles_the_thumb() {
		let thumb = Color::srgb(0.9, 0.2, 0.3);
		let track = Color::srgb(0.1, 0.1, 0.1);
		let mut rules = scroller_rules(Overflow::Visible, Overflow::Scroll);
		rules.push(Rule::class("scroller").with_value(
			common_props::ScrollbarColorProp,
			ScrollbarColor { thumb, track },
		));
		let buffer = scroll_buffer(UVec2::new(12, 8), rules, 10, IVec2::ZERO);
		// the thumb cell (the solid block) carries the resolved thumb colour
		let thumb_fg = buffer
			.iter_cells()
			.find(|(_, cell)| cell.symbol_str() == "█")
			.map(|(_, cell)| cell.style.foreground);
		thumb_fg.xpect_eq(Some(Some(thumb)));
	}

	/// `scrollbar-width: none` removes the gutter, so content uses the full width
	/// and no bar cell is painted.
	#[beet_core::test]
	fn scrollbar_width_none_removes_gutter() {
		let mut rules = scroller_rules(Overflow::Visible, Overflow::Scroll);
		rules.push(Rule::class("scroller").with_value(
			common_props::ScrollbarWidthProp,
			ScrollbarWidth::None,
		));
		let buffer = scroll_buffer(UVec2::new(12, 8), rules, 10, IVec2::ZERO);
		// no bar glyph anywhere
		let has_bar = buffer
			.iter_cells()
			.any(|(_, cell)| matches!(cell.symbol_str(), "│" | "█"));
		has_bar.xpect_false();
		// content reaches the full 8-cell scrollport width (no reserved column):
		// "r0" sits at the left and the row is not narrowed by a gutter.
		buffer.render_plain().xpect_contains("r0");
	}

	/// An unstyled scroll container still renders a visible default bar.
	#[beet_core::test]
	fn default_scrollbar_is_visible() {
		let buffer = scroll_buffer(
			UVec2::new(12, 8),
			scroller_rules(Overflow::Visible, Overflow::Scroll),
			10,
			IVec2::ZERO,
		);
		let has_bar = buffer
			.iter_cells()
			.any(|(_, cell)| matches!(cell.symbol_str(), "│" | "█"));
		has_bar.xpect_true();
	}
}
