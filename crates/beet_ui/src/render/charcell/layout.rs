//! Layout phase: assign [`LayoutRect`] top-down (pre-order).
//!
//! Each node answers: *"Given the rect I've been granted, how do I distribute
//! space to my children?"*
use super::*;
use crate::style::Display;
use crate::style::Length;
use crate::style::Position;
use crate::style::PositionStyle;
use beet_core::prelude::*;
use bevy::math::IRect;
use bevy::math::IVec2;
use bevy::math::UVec2;

use super::query::CharcellNodeData;
use super::query::CharcellQuery;

/// The definite screen rect a node occupies, including margin, in signed cell
/// space.
///
/// Written by the layout phase, read by the paint phase. This is the single
/// source of truth for where a node lives on screen. Signed so off-screen,
/// negative-margin, and scrolled geometry are representable, exactly as the DOM
/// positions boxes; the paint pass drops cells with negative coordinates.
#[derive(Component, Copy, Clone, Debug, Default, PartialEq)]
pub struct LayoutRect(pub IRect);

/// ECS system: assign [`LayoutRect`] to all nodes top-down.
pub fn layout_nodes<B: Component + AsBuffer>(
	mut params: ParamSet<(CharcellQuery, Query<&mut LayoutRect>)>,
	tree: CharcellTree,
	roots: Populated<(Entity, &B)>,
) -> Result {
	for (root, buffer) in roots {
		let viewport_size = buffer.size();
		let ordered = tree.pre_order(root);

		// Root gets the full viewport rect
		let mut layout_rects = HashMap::<Entity, IRect>::new();
		layout_rects.insert(
			root,
			IRect::new(0, 0, viewport_size.x as i32, viewport_size.y as i32),
		);

		// Read phase: use CharcellQuery to distribute rects to children.
		// `managed` holds the structural rows/wrappers a table laid out itself, so
		// the loop doesn't re-flow them as plain blocks.
		{
			let charcell = params.p0();
			// child -> parent, for resolving an absolute element's containing block.
			let parents = parent_map(root, &charcell, &tree);
			let mut managed = HashSet::<Entity>::default();
			for &entity in &ordered {
				if managed.contains(&entity) {
					continue;
				}
				let Ok(node) = charcell.unresolved_node(entity) else {
					continue;
				};

				// position the node before laying out its children: a relative box
				// shifts (carrying its content), an absolute/fixed box is placed
				// against its containing block (its in-flow parent skipped it), so its
				// subtree then lays out within the positioned rect.
				position_node(
					entity,
					&node,
					&charcell,
					&parents,
					viewport_size,
					&mut layout_rects,
				);

				let Some(&node_rect) = layout_rects.get(&entity) else {
					continue;
				};

				match node.layout_style().display {
					Display::Flex => flex_layout_rects(
						&node,
						&charcell,
						node_rect,
						viewport_size,
						&mut layout_rects,
					)?,
					Display::Grid => grid_layout_rects(
						&node,
						&charcell,
						node_rect,
						viewport_size,
						&mut layout_rects,
					)?,
					Display::Table => table_layout_rects(
						&node,
						&charcell,
						node_rect,
						viewport_size,
						&mut layout_rects,
						&mut managed,
					)?,
					// a list item lays out as a block; its marker is drawn by the
					// decorator, so charcell treats `list-item` identically to `block`.
					// a table cell flows its own content as a block within its grid rect.
					Display::Block | Display::ListItem | Display::TableCell => {
						block_layout_rects(
							&node,
							&charcell,
							node_rect,
							viewport_size,
							&mut layout_rects,
						)?
					}
					Display::Inline => inline_layout_rects(
						&node,
						&charcell,
						node_rect,
						viewport_size,
						&mut layout_rects,
					)?,
					// removed from layout: skip the subtree so children get no
					// rects and are not drawn
					Display::None => {}
				}
			}
		}

		// Write phase: flush computed rects to ECS components
		for (entity, rect) in layout_rects {
			if let Ok(mut layout_rect) = params.p1().get_mut(entity) {
				layout_rect.set_if_neq(LayoutRect(rect));
			}
		}
	}
	Ok(())
}

/// Block flow: stack children top-to-bottom, each taking full parent width.
pub fn block_layout_rects(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	container_rect: IRect,
	viewport: UVec2,
	layout_rects: &mut HashMap<Entity, IRect>,
) -> Result {
	let box_model = BoxModel::from_node(node, viewport);
	// a scroll container reserves its scrollbar gutter, so children flow within
	// the scrollport (content rect minus gutter), reflowing around the bar.
	let content_rect =
		scrollport_of(node, query, box_model.content_rect(container_rect));
	let containing = UVec2::new(
		content_rect.width().max(0) as u32,
		content_rect.height().max(0) as u32,
	);
	// a list item's marker occupies a left gutter; its children are inset past
	// it (the marker itself paints into the gutter in the paint phase).
	let gutter = marker_gutter(node, query) as i32;
	let child_min_x = (content_rect.min.x + gutter).min(content_rect.max.x);
	let full_width = (content_rect.max.x - child_min_x).max(0) as u32;
	let mut child_y = content_rect.min.y;
	// a scroll container lays out its full content past the scrollport (the
	// scrollable overflow region), so children are not clipped to it.
	let scrolls = node.is_scroll_container();
	// out-of-flow (absolute/fixed) children are skipped here and placed in the
	// positioning pass, so in-flow siblings lay out as if they are absent.
	let children: Vec<_> = node.flow_child_nodes(query).collect();
	let last = children.len().saturating_sub(1);
	for (i, child) in children.iter().enumerate() {
		// stop once past the content box, unless this is a scroll container whose
		// overflow is meant to extend below the scrollport.
		if !scrolls && child_y >= content_rect.max.y {
			break;
		}
		// an explicit (percent/absolute) width takes the child off full-bleed block
		// flow, clamped to the available width; otherwise it fills the content box.
		// A kitty raster is a replaced element: like CSS `width: auto` on an
		// `<img>`, its box hugs the raster's cell width (aspect-derived from an
		// explicit height when one is given).
		let (explicit_w, explicit_h) =
			explicit_box_size(child, viewport, containing);
		let raster_w = child.kitty_image().map(|image| {
			image
				.cell_size_constrained(explicit_w, explicit_h, full_width)
				.x
		});
		let child_width = explicit_w
			.or(raster_w)
			.unwrap_or(full_width)
			.min(full_width);
		// height resolved at the assigned width, not the wider measured width, so
		// a narrowed column reserves every wrapped row instead of clipping the tail.
		let child_height = explicit_h.unwrap_or_else(|| {
			resolve_height(child, query, child_width, viewport)
		});
		// the last child keeps its full box so its own bottom-margin inset never
		// clips content; the container was already measured one margin shorter
		// (see `node_bottom_margin`), so that empty trailing-margin row simply
		// spills into the container's bottom padding rather than reserving a gap.
		// a scroll container never clamps a child to the scrollport.
		let bottom = if scrolls || i == last {
			child_y + child_height as i32
		} else {
			(child_y + child_height as i32).min(content_rect.max.y)
		};
		let child_rect = IRect::new(
			child_min_x,
			child_y,
			(child_min_x + child_width as i32).min(content_rect.max.x),
			bottom,
		);
		layout_rects.insert(child.entity, child_rect);
		child_y += child_height.max(1) as i32;
	}
	Ok(())
}

/// Inline flow: place children left-to-right, wrapping rows when width is exceeded.
///
/// Each row's height equals the tallest child in that row.
pub fn inline_layout_rects(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	container_rect: IRect,
	viewport: UVec2,
	layout_rects: &mut HashMap<Entity, IRect>,
) -> Result {
	let box_model = BoxModel::from_node(node, viewport);
	let content_rect =
		scrollport_of(node, query, box_model.content_rect(container_rect));
	let max_width = content_rect.width().max(0) as u32;

	// Form rows: greedily pack children left-to-right, wrapping as needed
	let mut rows: Vec<Vec<(Entity, UVec2)>> = Vec::new();
	let mut current_row: Vec<(Entity, UVec2)> = Vec::new();
	let mut current_row_width = 0u32;

	for child in node.flow_child_nodes(query) {
		let size = child.intrinsic_size();
		if !current_row.is_empty() && current_row_width + size.x > max_width {
			rows.push(std::mem::take(&mut current_row));
			current_row_width = 0;
		}
		current_row_width += size.x;
		current_row.push((child.entity, size));
	}
	if !current_row.is_empty() {
		rows.push(current_row);
	}

	// Assign rects: rows stack vertically, children in each row sit side-by-side
	let mut row_y = content_rect.min.y;
	for row in &rows {
		let row_height = row.iter().map(|(_, s)| s.y).max().unwrap_or(1);
		if row_y >= content_rect.max.y {
			break;
		}
		let mut child_x = content_rect.min.x;
		for &(entity, size) in row {
			let child_rect = IRect::new(
				child_x,
				row_y,
				(child_x + size.x as i32).min(content_rect.max.x),
				(row_y + size.y as i32).min(content_rect.max.y),
			);
			layout_rects.insert(entity, child_rect);
			child_x += size.x as i32;
		}
		row_y += row_height.max(1) as i32;
	}
	Ok(())
}

// ── Positioning ───────────────────────────────────────────────────────────────

/// Build a child -> parent map over the buffer tree, for resolving an absolute
/// element's containing block by walking ancestors.
fn parent_map(
	root: Entity,
	query: &CharcellQuery,
	tree: &CharcellTree,
) -> HashMap<Entity, Entity> {
	let mut parents = HashMap::<Entity, Entity>::default();
	for entity in tree.pre_order(root) {
		if query.unresolved_node(entity).is_err() {
			continue;
		}
		for child in tree.children_of(entity) {
			parents.insert(child, entity);
		}
	}
	parents
}

/// Place a positioned node, mutating its rect in `layout_rects`. A static node is
/// left in its flow rect. Called in pre-order before the node's children lay out,
/// so an absolute box (skipped by its in-flow parent) is positioned first and its
/// subtree then flows within the new rect.
fn position_node(
	entity: Entity,
	node: &CharcellNodeData,
	query: &CharcellQuery,
	parents: &HashMap<Entity, Entity>,
	viewport: UVec2,
	layout_rects: &mut HashMap<Entity, IRect>,
) {
	let style = node.position_style();
	match style.position {
		Position::Static => {}
		Position::Relative => {
			// translate the flow rect by (left - right, top - bottom); the flow slot
			// is preserved (siblings already laid out around the static position).
			let Some(&rect) = layout_rects.get(&entity) else {
				return;
			};
			let containing = containing_size(rect);
			let dx = inset_axis(
				style.left(),
				style.right(),
				viewport,
				containing.x,
				true,
			);
			let dy = inset_axis(
				style.top(),
				style.bottom(),
				viewport,
				containing.y,
				false,
			);
			layout_rects
				.insert(entity, translate_rect(rect, IVec2::new(dx, dy)));
		}
		Position::Absolute | Position::Fixed => {
			let block = match style.position {
				Position::Fixed => {
					IRect::new(0, 0, viewport.x as i32, viewport.y as i32)
				}
				// absolute: nearest positioned ancestor's padding box, else viewport
				_ => containing_block(
					entity,
					query,
					parents,
					layout_rects,
					viewport,
				),
			};
			layout_rects
				.insert(entity, absolute_rect(node, &style, block, viewport));
		}
		Position::Sticky => {
			sticky_clamp(entity, node, query, parents, viewport, layout_rects);
		}
	}
}

/// The containing block (padding box) for an absolute node: the nearest ancestor
/// with `position != static`, else the viewport. Reads the ancestor's
/// already-positioned rect from `layout_rects` via its box model padding box.
fn containing_block(
	entity: Entity,
	query: &CharcellQuery,
	parents: &HashMap<Entity, Entity>,
	layout_rects: &HashMap<Entity, IRect>,
	viewport: UVec2,
) -> IRect {
	let mut current = entity;
	while let Some(&parent) = parents.get(&current) {
		if let Ok(parent_node) = query.unresolved_node(parent) {
			if parent_node.position_style().is_positioned() {
				// read the parent's current (possibly just-positioned) rect from the
				// map, not its stale ECS component which is flushed only after layout.
				let rect = layout_rects
					.get(&parent)
					.copied()
					.unwrap_or_else(|| parent_node.layout_rect());
				return BoxModel::from_node(&parent_node, viewport)
					.inner_rect(rect);
			}
		}
		current = parent;
	}
	IRect::new(0, 0, viewport.x as i32, viewport.y as i32)
}

/// The rect of an absolute/fixed box within its containing `block`.
///
/// Each axis: with both insets set (and no explicit size) the box stretches
/// between them; with one inset it anchors to that edge at its content/explicit
/// size; with neither it stays at the block's start edge (static-ish fallback).
fn absolute_rect(
	node: &CharcellNodeData,
	style: &PositionStyle,
	block: IRect,
	viewport: UVec2,
) -> IRect {
	let box_model = BoxModel::from_node(node, viewport);
	let intrinsic = node.intrinsic_size();
	let block_w = (block.width().max(0)) as u32;
	let block_h = (block.height().max(0)) as u32;

	let (left, right) = (
		style
			.left()
			.map(|l| inset_cells(l, viewport, block_w, true)),
		style
			.right()
			.map(|l| inset_cells(l, viewport, block_w, true)),
	);
	let (top, bottom) = (
		style
			.top()
			.map(|l| inset_cells(l, viewport, block_h, false)),
		style
			.bottom()
			.map(|l| inset_cells(l, viewport, block_h, false)),
	);
	let explicit_w = box_model.width.map(|w| w + box_model.overhead().x);
	let explicit_h = box_model.height.map(|h| h + box_model.overhead().y);

	let (x0, x1) = axis_extent(
		block.min.x,
		block.max.x,
		left,
		right,
		explicit_w,
		intrinsic.x,
	);
	let (y0, y1) = axis_extent(
		block.min.y,
		block.max.y,
		top,
		bottom,
		explicit_h,
		intrinsic.y,
	);
	IRect::new(x0, y0, x1, y1)
}

/// Resolve one axis of an absolute box to `(start, end)` within `[block_min,
/// block_max]`, given the leading/trailing insets, an explicit size, and the
/// content size fallback.
fn axis_extent(
	block_min: i32,
	block_max: i32,
	lead: Option<i32>,
	trail: Option<i32>,
	explicit: Option<u32>,
	content: u32,
) -> (i32, i32) {
	match (lead, trail, explicit) {
		// both insets, no explicit size: stretch between them
		(Some(a), Some(b), None) => (block_min + a, block_max - b),
		// trailing inset wins the anchor when there's a size
		(_, Some(b), size) => {
			let len = size.unwrap_or(content) as i32;
			let end = block_max - b;
			(end - len, end)
		}
		// leading inset (or neither): anchor to the start edge
		(lead, None, size) => {
			let start = block_min + lead.unwrap_or(0);
			(start, start + size.unwrap_or(content) as i32)
		}
	}
}

/// Clamp a sticky node within its nearest scroll container's scrollport using
/// its insets and the container's scroll offset.
///
/// Sticky lays out in flow, then once the container scrolls past the node's flow
/// position the node pins to the scrollport edge (per the active inset), and
/// un-pins again at the container's far edge.
fn sticky_clamp(
	entity: Entity,
	node: &CharcellNodeData,
	query: &CharcellQuery,
	parents: &HashMap<Entity, Entity>,
	viewport: UVec2,
	layout_rects: &mut HashMap<Entity, IRect>,
) {
	let Some(&rect) = layout_rects.get(&entity) else {
		return;
	};
	// find the nearest scroll-container ancestor and its scroll offset.
	let mut current = entity;
	let scroller = loop {
		let Some(&parent) = parents.get(&current) else {
			return; // no scroll container: sticky behaves as relative-at-zero
		};
		if let Ok(parent_node) = query.unresolved_node(parent) {
			if parent_node.is_scroll_container() {
				break parent_node;
			}
		}
		current = parent;
	};
	let style = node.position_style();
	// the scroller's current rect from the map (its component is flushed only
	// after layout), then its scrollport (content rect minus gutter).
	let scroller_rect = layout_rects
		.get(&scroller.entity)
		.copied()
		.unwrap_or_else(|| scroller.layout_rect());
	let scrollport = scrollport_of(
		&scroller,
		query,
		BoxModel::from_node(&scroller, viewport).content_rect(scroller_rect),
	);
	let offset = scroller.scroll_offset();
	// the node's flow position is fixed in content space; as the container
	// scrolls (offset), the painted position shifts by -offset. Sticky pins the
	// painted top to `scrollport.min.y + top` once it would scroll above that.
	let mut sticky = rect;
	if let Some(top) = style.top() {
		let pin = scrollport.min.y + inset_cells(top, viewport, 0, false);
		// painted top after scroll = rect.min.y - offset.y; pin to `pin` if higher
		let painted_top = rect.min.y - offset.y;
		if painted_top < pin {
			// shift down so the painted top sits at the pin (add offset back + delta)
			let delta = pin - painted_top;
			sticky = translate_rect(sticky, IVec2::new(0, delta));
		}
	}
	if let Some(left) = style.left() {
		let pin = scrollport.min.x + inset_cells(left, viewport, 0, true);
		let painted_left = rect.min.x - offset.x;
		if painted_left < pin {
			let delta = pin - painted_left;
			sticky = translate_rect(sticky, IVec2::new(delta, 0));
		}
	}
	layout_rects.insert(entity, sticky);
}

/// One axis of a relative offset: `lead - trail` in cells, each `auto` (`None`)
/// resolving to 0, against the `containing` extent.
fn inset_axis(
	lead: Option<Length>,
	trail: Option<Length>,
	viewport: UVec2,
	containing: u32,
	x_axis: bool,
) -> i32 {
	let lead = lead
		.map(|l| inset_cells(l, viewport, containing, x_axis))
		.unwrap_or(0);
	let trail = trail
		.map(|l| inset_cells(l, viewport, containing, x_axis))
		.unwrap_or(0);
	lead - trail
}

/// The size of a rect as a `UVec2` (for resolving percentage insets).
fn containing_size(rect: IRect) -> UVec2 {
	UVec2::new(rect.width().max(0) as u32, rect.height().max(0) as u32)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::style::*;
	use bevy::math::IRect;
	use bevy::math::IVec2;

	fn render(bundle: impl Bundle) -> String {
		Buffer::render_oneshot_plain_sized(UVec2::new(20, 10), bundle)
			.trim_lines()
	}

	/// A [`LayoutRect`] holds a signed rect with a negative origin (off the
	/// top-left of the viewport), and the paint boundary drops the cells that fall
	/// outside the buffer while keeping the in-bounds remainder.
	#[beet_core::test]
	fn negative_origin_rect_clips_at_paint_boundary() {
		// round-trips a negative-origin rect unchanged
		let rect = IRect::new(-2, -1, 3, 2);
		LayoutRect(rect).0.xpect_eq(rect);

		// the paint boundary drops any cell with a negative component
		Clip::NONE.cell(IVec2::new(-1, 0)).xpect_eq(None);
		Clip::NONE.cell(IVec2::new(0, -1)).xpect_eq(None);
		Clip::NONE
			.cell(IVec2::new(1, 2))
			.xpect_eq(Some(UVec2::new(1, 2)));

		// filling that rect into a small buffer paints only the on-screen
		// quadrant (x in 0..3, y in 0..2), leaving the off-screen rows/cols blank
		let mut buffer = Buffer::new(UVec2::new(5, 5));
		buffer.fill_rect(
			rect,
			Cell::new("x", VisualStyle::default(), Entity::PLACEHOLDER),
			Clip::NONE,
		);
		let painted: Vec<UVec2> =
			buffer.iter_cells().map(|(pos, _)| pos).collect();
		painted
			.iter()
			.all(|pos| pos.x < 3 && pos.y < 2)
			.xpect_true();
		painted.contains(&UVec2::new(0, 0)).xpect_true();
		painted.contains(&UVec2::new(2, 1)).xpect_true();
	}

	/// The page chrome fills the fixed terminal viewport like the web: the body's
	/// flex column stretches the header/footer to full width (`align-items:
	/// stretch` filling the container, not just the content), `min-height: 100vh`
	/// plus the container's `flex-grow` pins the footer to the bottom row, and the
	/// container's row stretches the sidebar to full height so its right divider
	/// runs the whole rail. Regression for the terminal app rendering
	/// content-sized: a short header, a content-height sidebar border.
	#[beet_core::test]
	fn page_chrome_fills_viewport() {
		use crate::prelude::*;
		let mut world = (
			TemplatePlugin,
			DocumentPlugin,
			CharcellPlugin,
			crate::style::material::MaterialStylePlugin::default(),
		)
			.into_world();
		let root = world
			.spawn_template(rsx! {
				<html lang="en">
					<body {Classes::new([classes::PAGE])}>
						<header {Classes::new([classes::APP_BAR])}>"Header"</header>
						<div {Classes::new([classes::CONTAINER])}>
							<nav {Classes::new([classes::SIDEBAR])}>"Nav"</nav>
							<main>"Content"</main>
						</div>
						<footer>"Footer"</footer>
					</body>
				</html>
			})
			.unwrap()
			.id();
		let size = UVec2::new(60, 24);
		world
			.entity_mut(root)
			.insert(Buffer::new(size).into_double_buffer());
		world.run_schedule(crate::parse::PostParseTree);
		let rects = world
			.run_system_once(|q: Query<(&Element, &LayoutRect)>| {
				q.iter()
					.map(|(el, r)| (el.tag().to_string(), r.0))
					.collect::<HashMap<_, _>>()
			})
			.unwrap();
		let rect = |tag: &str| rects[tag];
		// header and footer span the full viewport width
		rect("header").width().xpect_eq(60);
		rect("footer").width().xpect_eq(60);
		// the footer pins to the bottom row of the 24-row viewport
		rect("footer").max.y.xpect_eq(24);
		// the sidebar rail runs the full height of the content row (its divider too)
		let container = rect("div");
		rect("nav").height().xpect_eq(container.height());
		(container.height() > 10).xpect_true();
	}

	/// The content measure applies in the terminal too: on a wide viewport a
	/// `main > *` child stops at the 70-cell measure and `align-items: center`
	/// insets it equally. End-to-end regression for the charcell cascade honouring
	/// the child combinator (previously web-only, so the terminal ignored it).
	#[beet_core::test]
	fn main_content_measure_caps_in_terminal() {
		use crate::prelude::*;
		let mut world = (
			TemplatePlugin,
			DocumentPlugin,
			CharcellPlugin,
			crate::style::material::MaterialStylePlugin::default(),
		)
			.into_world();
		let root = world
			.spawn_template(rsx! {
				<html lang="en">
					<body {Classes::new([classes::PAGE])}>
						<div {Classes::new([classes::CONTAINER])}>
							<main><section>"content"</section></main>
						</div>
					</body>
				</html>
			})
			.unwrap()
			.id();
		world
			.entity_mut(root)
			.insert(Buffer::new(UVec2::new(100, 24)).into_double_buffer());
		world.run_schedule(crate::parse::PostParseTree);
		let rects = world
			.run_system_once(|q: Query<(&Element, &LayoutRect)>| {
				q.iter()
					.map(|(el, r)| (el.tag().to_string(), r.0))
					.collect::<HashMap<_, _>>()
			})
			.unwrap();
		let (main, section) = (rects["main"], rects["section"]);
		// capped at the 70-cell measure, not main's ~100-cell width
		section.width().xpect_eq(70);
		// centred: inset from the left, with equal gaps either side (±1 for an
		// odd remainder).
		let (left, right) = (section.min.x - main.min.x, main.max.x - section.max.x);
		(left > 1).xpect_true();
		((left - right).abs() <= 1).xpect_true();
	}

	#[beet_core::test]
	fn inline_places_children_side_by_side() {
		let out = render((
			LayoutStyle {
				display: Display::Inline,
				..default()
			},
			children![rsx! {"A"}, rsx! {"B"}, rsx! {"C"}],
		));
		// All three children should appear on the same line
		let first_line = out.lines().next().unwrap_or("");
		first_line.xpect_contains("A");
		first_line.xpect_contains("B");
		first_line.xpect_contains("C");
	}

	#[beet_core::test]
	fn inline_wraps_when_overflowing() {
		let out = Buffer::render_oneshot_plain_sized(
			UVec2::new(10, 10),
			(
				LayoutStyle {
					display: Display::Inline,
					..default()
				},
				children![rsx! {"Hello"}, rsx! {"World"}, rsx! {"Foo"},],
			),
		)
		.trim_lines();
		// Should wrap — not all on one line since 5+5+3 = 13 > 10
		let lines: Vec<&str> = out.lines().collect();
		(lines.len() >= 2).xpect_true();
	}

	/// The width in cells of the background fill on row `y` of `buffer`.
	fn fill_width(buffer: &Buffer, bg: Color, y: u32) -> usize {
		buffer
			.iter_cells()
			.filter(|(pos, cell)| {
				pos.y == y && cell.style.background == Some(bg)
			})
			.count()
	}

	/// A block child carrying `box_style`, filled with `bg`. Wrapped in an explicit
	/// block container so the child is block-level (an inline-level node would not
	/// get a box fill, matching CSS).
	fn bg_block(box_style: BoxStyle, bg: Color) -> impl Bundle {
		(LayoutStyle::default(), children![(
			LayoutStyle::default(),
			box_style,
			VisualStyle {
				background: Some(bg),
				..default()
			},
			children![rsx! {"x"}],
		)])
	}

	/// A block child whose `width` is the given [`Length`], filled with `bg`.
	fn sized_block(width: Length, bg: Color) -> impl Bundle {
		bg_block(
			BoxStyle {
				width: Some(width),
				..default()
			},
			bg,
		)
	}

	/// A percentage `width` resolves against the containing block in the layout
	/// pass (the measure pass leaves it content-sized), so a `width: 50%` block in
	/// block flow occupies half its container's content width on the terminal.
	#[beet_core::test]
	fn percent_width_resolves_against_container() {
		let bg = Color::srgb(0.2, 0.4, 0.8);
		let buffer = Buffer::new(UVec2::new(20, 4))
			.populate(sized_block(Length::Percent(50.), bg));
		// the half-width fill covers 10 of the 20 columns on the first row
		fill_width(&buffer, bg, 0).xpect_eq(10);
	}

	/// A viewport-relative `width` resolves against the real viewport in both
	/// passes (the viewport is always known), so a `width: 50vw` block on a
	/// 20-column buffer is 10 columns wide.
	#[beet_core::test]
	fn viewport_width_resolves_against_viewport() {
		let bg = Color::srgb(0.8, 0.3, 0.2);
		let buffer = Buffer::new(UVec2::new(20, 4))
			.populate(sized_block(Length::ViewportWidth(50.), bg));
		fill_width(&buffer, bg, 0).xpect_eq(10);
	}

	/// `min-width` floors a box narrower than it, growing a two-cell width up to it.
	#[beet_core::test]
	fn min_width_floors_narrow_box() {
		let bg = Color::srgb(0.2, 0.4, 0.8);
		let buffer = Buffer::new(UVec2::new(20, 4)).populate(bg_block(
			BoxStyle {
				width: Some(Length::Rem(2.)),
				min_width: Some(Length::Rem(8.)),
				..default()
			},
			bg,
		));
		fill_width(&buffer, bg, 0).xpect_eq(8);
	}

	/// `max-height` caps a box, clamping an explicit six-row height to two rows.
	#[beet_core::test]
	fn max_height_caps_box() {
		let bg = Color::srgb(0.2, 0.4, 0.8);
		let buffer = Buffer::new(UVec2::new(20, 8)).populate(bg_block(
			BoxStyle {
				height: Some(Length::Rem(6.)),
				max_height: Some(Length::Rem(2.)),
				..default()
			},
			bg,
		));
		// rows 0-1 are painted; row 2 is past the two-row cap
		(fill_width(&buffer, bg, 1) > 0).xpect_true();
		fill_width(&buffer, bg, 2).xpect_eq(0);
	}

	/// No rendered line may exceed the buffer width, otherwise the terminal
	/// soft-wraps and content appears a column too wide (see the `layout` example).
	#[beet_core::test]
	fn never_renders_wider_than_buffer() {
		let bordered =
			BoxStyle::default().with_border(Spacing::all(Length::Rem(1.)));
		let width = 12;
		// a grow box filling the line, an unbreakable run, and wide chars that
		// would straddle the right edge all stay within the buffer width.
		let bundle = (
			LayoutStyle::flex_row().column_gap(Length::Rem(1.)),
			children![
				(rsx! { "中文日本語ＡＢＣ" }, bordered.clone()),
				(
					rsx! { "Supercalifragilistic" },
					bordered.clone(),
					LayoutStyle::default().with_flex_grow(1)
				),
			],
		);
		Buffer::render_oneshot_sized(UVec2::new(width, 6), bundle)
			.lines()
			.map(display_width)
			.all(|line_width| line_width <= width as usize)
			.xpect_true();
	}

	// ── Positioning ──

	use crate::prelude::*;

	/// Render `content` into a `size` buffer with `rules` applied, returning the
	/// plain frame. Positioning, like overflow, is authored through the rule set
	/// (resolution clobbers a hand-attached style on an element subtree).
	fn positioned_frame(
		size: UVec2,
		rules: Vec<Rule>,
		content: impl Bundle,
	) -> String {
		let mut world = CharcellPlugin::world();
		world.get_resource_or_init::<RuleSet>().extend_rules(rules);
		let root = world
			.spawn((Buffer::new(size).into_double_buffer(), content))
			.id();
		world.run_schedule(PostParseTree);
		world
			.entity_mut(root)
			.take::<DoubleBuffer>()
			.unwrap()
			.into_buffer()
			.render_plain()
	}

	/// The (col, row) of the first cell of `needle` in a plain frame.
	fn cell_of(frame: &str, needle: char) -> (usize, usize) {
		for (row, line) in frame.lines().enumerate() {
			if let Some(col) = line.find(needle) {
				return (col, row);
			}
		}
		panic!("'{needle}' not found in frame:\n{frame}");
	}

	/// `position: relative` shifts a box by its insets (x doubled, y not), while
	/// a sibling keeps its static slot.
	#[beet_core::test]
	fn relative_shifts_box_and_leaves_sibling() {
		let frame = positioned_frame(
			UVec2::new(20, 8),
			vec![
				Rule::class("rel")
					.with_value(common_props::PositionProp, Position::Relative)
					.with_value(common_props::InsetLeft, Length::Rem(2.))
					.with_value(common_props::InsetTop, Length::Rem(1.)),
			],
			rsx! {
				<div>
					<div class="rel">"A"</div>
					<div>"B"</div>
				</div>
			},
		);
		// A shifts right by 2rem*2 = 4 cells and down by 1 row from its static (0,0)
		cell_of(&frame, 'A').xpect_eq((4, 1));
		// B keeps its static slot on its own row (relative does not pull it up)
		let (_, b_row) = cell_of(&frame, 'B');
		(b_row >= 1).xpect_true();
	}

	/// `position: absolute` `top:0 right:0` lands at its positioned ancestor's
	/// top-right corner and leaves flow, so a following sibling occupies its old
	/// slot. (Placed top-right to avoid overlapping Y; the CSS paint order for
	/// overlapping positioned boxes is Task 06.)
	#[beet_core::test]
	fn absolute_anchors_to_positioned_ancestor() {
		let frame = positioned_frame(
			UVec2::new(20, 8),
			vec![
				Rule::class("anchor")
					.with_value(common_props::PositionProp, Position::Relative)
					.with_value(common_props::Height, Length::Rem(5.)),
				Rule::class("abs")
					.with_value(common_props::PositionProp, Position::Absolute)
					.with_value(common_props::InsetTop, Length::Rem(0.))
					.with_value(common_props::InsetRight, Length::Rem(0.)),
			],
			rsx! {
				<div class="anchor">
					<div class="abs">"X"</div>
					<div>"Y"</div>
				</div>
			},
		);
		// X anchors to the ancestor's top-right padding corner (col 19 of 20)
		cell_of(&frame, 'X').xpect_eq((19, 0));
		// Y took X's old in-flow slot (the first row), since X left flow
		let (y_col, y_row) = cell_of(&frame, 'Y');
		y_col.xpect_eq(0);
		y_row.xpect_eq(0);
	}

	/// `position: fixed` lands against the viewport regardless of ancestors.
	#[beet_core::test]
	fn fixed_anchors_to_viewport_bottom_left() {
		let frame = positioned_frame(
			UVec2::new(20, 6),
			vec![
				Rule::class("fix")
					.with_value(common_props::PositionProp, Position::Fixed)
					.with_value(common_props::InsetBottom, Length::Rem(0.))
					.with_value(common_props::InsetLeft, Length::Rem(0.)),
			],
			rsx! {
				<div>
					<div>"top"</div>
					<div class="fix">"F"</div>
				</div>
			},
		);
		// F sits at the bottom-left of the 6-row viewport (last row, col 0)
		let (col, row) = cell_of(&frame, 'F');
		col.xpect_eq(0);
		row.xpect_eq(5);
	}

	/// `position: sticky` `top: 0` inside a scroll container pins its laid-out rect
	/// so that, after the paint scroll translation, it stays at the scrollport top.
	///
	/// Asserted on the laid-out rect (not the painted frame): the sticky pins to
	/// `scroll_offset` rows down in layout, which the paint `-offset` translation
	/// cancels back to the scrollport top. The CSS paint order that keeps the
	/// header above the scrolled content is Task 06.
	#[beet_core::test]
	fn sticky_pins_within_scroll_container() {
		let mut world = CharcellPlugin::world();
		world.get_resource_or_init::<RuleSet>().extend_rules(vec![
			Rule::class("scroller")
				.with_value(common_props::Height, Length::Rem(4.))
				.with_value(common_props::OverflowYProp, Overflow::Scroll),
			Rule::class("stick")
				.with_value(common_props::PositionProp, Position::Sticky)
				.with_value(common_props::InsetTop, Length::Rem(0.)),
		]);
		world.spawn((
			Buffer::new(UVec2::new(16, 8)).into_double_buffer(),
			rsx! {
				<div>
					<div class="scroller">
						<div class="stick">"S"</div>
						<pre>"a\nb\nc\nd\ne\nf\ng\nh"</pre>
					</div>
				</div>
			},
		));
		world.run_schedule(PostParseTree);
		// locate the sticky element by its resolved position
		let stick = world
			.query::<(Entity, &PositionStyle)>()
			.iter(&world)
			.find(|(_, p)| p.position == Position::Sticky)
			.map(|(e, _)| e)
			.unwrap();
		let scroller = world
			.query_filtered::<Entity, With<ScrollPosition>>()
			.iter(&world)
			.next()
			.unwrap();
		// before scrolling, the sticky sits at its flow position (scrollport top)
		let flow_top = world.get::<LayoutRect>(stick).unwrap().0.min.y;
		// scroll down by 3 rows
		world
			.entity_mut(scroller)
			.insert(ScrollPosition::new(IVec2::new(0, 3)));
		world.run_schedule(PostParseTree);
		// the sticky's laid-out top shifts down by the scroll amount, so the paint
		// `-offset` translation lands it back at the scrollport top (pinned).
		let pinned_top = world.get::<LayoutRect>(stick).unwrap().0.min.y;
		(pinned_top - flow_top).xpect_eq(3);
	}
}
