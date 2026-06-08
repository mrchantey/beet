//! Layout phase: assign [`LayoutRect`] top-down (pre-order).
//!
//! Each node answers: *"Given the rect I've been granted, how do I distribute
//! space to my children?"*
use super::*;
use crate::style::Display;
use beet_core::prelude::*;
use bevy::math::URect;
use bevy::math::UVec2;

use super::query::CharcellNodeData;
use super::query::CharcellQuery;

/// The definite screen rect a node occupies, including margin.
///
/// Written by the layout phase, read by the paint phase.
/// This is the single source of truth for where a node lives on screen.
#[derive(Component, Copy, Clone, Debug, Default, PartialEq)]
pub struct LayoutRect(pub URect);

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
		let mut layout_rects = HashMap::<Entity, URect>::new();
		layout_rects
			.insert(root, URect::new(0, 0, viewport_size.x, viewport_size.y));

		// Read phase: use CharcellQuery to distribute rects to children.
		// `managed` holds the structural rows/wrappers a table laid out itself, so
		// the loop doesn't re-flow them as plain blocks.
		{
			let charcell = params.p0();
			let mut managed = HashSet::<Entity>::default();
			for &entity in &ordered {
				if managed.contains(&entity) {
					continue;
				}
				let Some(&node_rect) = layout_rects.get(&entity) else {
					continue;
				};
				let Ok(node) = charcell.unresolved_node(entity) else {
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
	container_rect: URect,
	viewport: UVec2,
	layout_rects: &mut HashMap<Entity, URect>,
) -> Result {
	let box_model = BoxModel::from_node(node, viewport);
	let content_rect = box_model.content_rect(container_rect);
	let containing = UVec2::new(content_rect.width(), content_rect.height());
	// a list item's marker occupies a left gutter; its children are inset past
	// it (the marker itself paints into the gutter in the paint phase).
	let gutter = marker_gutter(node, query);
	let child_min_x = (content_rect.min.x + gutter).min(content_rect.max.x);
	let full_width = content_rect.max.x.saturating_sub(child_min_x);
	let mut child_y = content_rect.min.y;
	let children: Vec<_> = node.child_nodes(query).collect();
	let last = children.len().saturating_sub(1);
	for (i, child) in children.iter().enumerate() {
		if child_y >= content_rect.max.y {
			break;
		}
		// an explicit (percent/absolute) width takes the child off full-bleed block
		// flow, clamped to the available width; otherwise it fills the content box.
		let (explicit_w, explicit_h) =
			explicit_box_size(child, viewport, containing);
		let child_width = explicit_w.unwrap_or(full_width).min(full_width);
		// height resolved at the assigned width, not the wider measured width, so
		// a narrowed column reserves every wrapped row instead of clipping the tail.
		let child_height = explicit_h
			.unwrap_or_else(|| resolve_height(child, query, child_width, viewport));
		// the last child keeps its full box so its own bottom-margin inset never
		// clips content; the container was already measured one margin shorter
		// (see `node_bottom_margin`), so that empty trailing-margin row simply
		// spills into the container's bottom padding rather than reserving a gap.
		let bottom = match i == last {
			true => child_y + child_height,
			false => (child_y + child_height).min(content_rect.max.y),
		};
		let child_rect = URect::new(
			child_min_x,
			child_y,
			(child_min_x + child_width).min(content_rect.max.x),
			bottom,
		);
		layout_rects.insert(child.entity, child_rect);
		child_y += child_height.max(1);
	}
	Ok(())
}

/// Inline flow: place children left-to-right, wrapping rows when width is exceeded.
///
/// Each row's height equals the tallest child in that row.
pub fn inline_layout_rects(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	container_rect: URect,
	viewport: UVec2,
	layout_rects: &mut HashMap<Entity, URect>,
) -> Result {
	let box_model = BoxModel::from_node(node, viewport);
	let content_rect = box_model.content_rect(container_rect);
	let max_width = content_rect.width();

	// Form rows: greedily pack children left-to-right, wrapping as needed
	let mut rows: Vec<Vec<(Entity, UVec2)>> = Vec::new();
	let mut current_row: Vec<(Entity, UVec2)> = Vec::new();
	let mut current_row_width = 0u32;

	for child in node.child_nodes(query) {
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
			let child_rect = URect::new(
				child_x,
				row_y,
				(child_x + size.x).min(content_rect.max.x),
				(row_y + size.y).min(content_rect.max.y),
			);
			layout_rects.insert(entity, child_rect);
			child_x += size.x;
		}
		row_y += row_height.max(1);
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::style::*;

	fn render(bundle: impl Bundle) -> String {
		Buffer::render_oneshot_plain_sized(UVec2::new(20, 10), bundle)
			.trim_lines()
	}

	#[beet_core::test]
	fn inline_places_children_side_by_side() {
		let out = render((
			LayoutStyle {
				display: Display::Inline,
				..default()
			},
			children![rsx_direct! {"A"}, rsx_direct! {"B"}, rsx_direct! {"C"}],
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
				children![
					rsx_direct! {"Hello"},
					rsx_direct! {"World"},
					rsx_direct! {"Foo"},
				],
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
			.filter(|(pos, cell)| pos.y == y && cell.style.background == Some(bg))
			.count()
	}

	/// A percentage `width` resolves against the containing block in the layout
	/// pass (the measure pass leaves it content-sized), so a `width: 50%` block in
	/// block flow occupies half its container's content width on the terminal.
	/// A half-width block child whose `width` is the given [`Length`], filled with
	/// `bg`. Wrapped in an explicit block container so the child is block-level
	/// (an inline-level node would not get a box fill, matching CSS).
	fn sized_block(width: Length, bg: Color) -> impl Bundle {
		(LayoutStyle::default(), children![(
			LayoutStyle::default(),
			BoxStyle {
				width: Some(width),
				..default()
			},
			VisualStyle {
				background: Some(bg),
				..default()
			},
			children![rsx_direct! {"x"}],
		)])
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

	/// No rendered line may exceed the buffer width, otherwise the terminal
	/// soft-wraps and content appears a column too wide (see the `layout` example).
	#[beet_core::test]
	fn never_renders_wider_than_buffer() {
		let bordered =
			BoxStyle::default().with_border(Spacing::all(Length::Rem(1.)));
		let width = 12;
		// a grow box filling the line, an unbreakable run, and wide chars that
		// would straddle the right edge all stay within the buffer width.
		let bundle = (LayoutStyle::flex_row().column_gap(Length::Rem(1.)), children![
			(rsx_direct! { "中文日本語ＡＢＣ" }, bordered.clone()),
			(
				rsx_direct! { "Supercalifragilistic" },
				bordered.clone(),
				LayoutStyle::default().with_flex_grow(1)
			),
		]);
		Buffer::render_oneshot_sized(UVec2::new(width, 6), bundle)
			.lines()
			.map(display_width)
			.all(|line_width| line_width <= width as usize)
			.xpect_true();
	}
}
