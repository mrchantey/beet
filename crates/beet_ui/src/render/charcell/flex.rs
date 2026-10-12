use super::BoxModel;
use super::CellBounds;
use crate::style::AlignContent;
use crate::style::AlignItems;
use crate::style::AlignSelf;
use crate::style::Direction;
use crate::style::Display;
use crate::style::FlexBox;
use crate::style::FlexWrap;
use crate::style::JustifyContent;
use crate::style::Length;
use beet_core::prelude::*;
use bevy::math::IRect;
use bevy::math::UVec2;

use super::axis_cells;
use super::establishes_inline_flow;
use super::explicit_box_size;
use super::horizontal_gutter_rows;
use super::marker_gutter;
use super::measure_grid;
use super::measure_inline_flow;
use super::measure_scaled;
use super::measure_text;
use super::min_content_width;
use super::node_bottom_margin;
use super::query::CharcellNodeData;
use super::query::CharcellQuery;
use super::resolve_table_height;
use super::scrollport_of;

// ── Helper functions ──────────────────────────────────────────────────────────

/// Resolve the effective direction based on viewport dimensions.
pub(super) fn resolve_direction(
	direction: Direction,
	viewport: UVec2,
) -> Direction {
	match direction {
		Direction::ViewportMin => {
			if viewport.x <= viewport.y {
				Direction::Horizontal
			} else {
				Direction::Vertical
			}
		}
		Direction::ViewportMax => {
			if viewport.x >= viewport.y {
				Direction::Horizontal
			} else {
				Direction::Vertical
			}
		}
		_ => direction,
	}
}

/// Whether `flexbox`'s main axis runs vertically (a column) at `viewport`.
fn is_vertical(flexbox: &FlexBox, viewport: UVec2) -> bool {
	resolve_direction(flexbox.direction, viewport) == Direction::Vertical
}

/// The main-axis extent of `size`: its height in a column, else its width.
fn main_of(size: UVec2, vertical: bool) -> u32 {
	if vertical { size.y } else { size.x }
}

/// The cross-axis extent of `size`: its width in a column, else its height.
fn cross_of(size: UVec2, vertical: bool) -> u32 {
	if vertical { size.x } else { size.y }
}

/// The gap between adjacent items of a line, in cells.
fn main_gap(flexbox: &FlexBox, vertical: bool, viewport: UVec2) -> u32 {
	match vertical {
		true => flexbox.row_gap_cells(viewport),
		false => flexbox.column_gap_cells(viewport),
	}
}

/// The gap between adjacent lines, in cells.
fn cross_gap(flexbox: &FlexBox, vertical: bool, viewport: UVec2) -> u32 {
	match vertical {
		true => flexbox.column_gap_cells(viewport),
		false => flexbox.row_gap_cells(viewport),
	}
}

/// The total gap between `count` adjacent items of a line, in cells.
fn line_gaps(
	flexbox: &FlexBox,
	vertical: bool,
	viewport: UVec2,
	count: usize,
) -> u32 {
	main_gap(flexbox, vertical, viewport) * (count as u32).saturating_sub(1)
}

// ── Flex items ────────────────────────────────────────────────────────────────

/// The flex container an item is sized against.
struct FlexFrame<'a, 'w, 's> {
	query: &'a CharcellQuery<'w, 's>,
	viewport: UVec2,
	/// Whether the main axis runs vertically (a column).
	vertical: bool,
	/// The container's content extent on the main axis, which a percent
	/// `flex-basis` or min/max main size resolves against; `None` while it is
	/// not yet known (the measure pass, a column's height), leaving a percent
	/// `auto`.
	main: Option<u32>,
	/// The container's content extent on the cross axis, bounding a column
	/// item's width.
	cross: u32,
	/// Rows of the nearest scroll port (see [`resolve_height`]).
	port_rows: u32,
}

/// One flex item and its main-axis sizing.
struct FlexItem {
	entity: Entity,
	/// The item's outer size before flexing: its `width`/`height`, else its
	/// content. The cross axis is read as is; on the main axis the
	/// [`FlexMain::base`] supersedes it.
	size: UVec2,
	/// The specified main size (`width`/`height`), outer, which an automatic
	/// minimum never exceeds.
	specified: Option<u32>,
	main: FlexMain,
	/// Whether `main.min` still waits on the automatic minimum (CSS
	/// `min-width: auto`). Resolving it walks the item's subtree, so only a line
	/// that overflows does: a content-sized base already sits at or above it.
	auto_min: bool,
	order: i32,
}

/// The main-axis inputs to resolving one flex item's size.
///
/// Sizes are outer cells, the item's own margin, border and padding included,
/// since a line's free space is what its items' outer sizes leave over.
#[derive(Debug, Clone, Copy, PartialEq)]
struct FlexMain {
	/// The flex base size: the `flex-basis`, else the `width`/`height` or
	/// content size.
	base: u32,
	/// The main-axis margin, border and padding: an outer size less its inner
	/// size.
	overhead: u32,
	/// The min main size, a floor no flexing crosses.
	min: u32,
	/// The max main size, `u32::MAX` when unbounded.
	max: u32,
	grow: f32,
	shrink: f32,
}

impl FlexMain {
	/// The hypothetical main size: the base clamped into the min and max, the
	/// min winning a conflict as in CSS. Lines form from these, and whether a
	/// line grows or shrinks is decided by their sum.
	fn hypothetical(&self) -> u32 { self.base.min(self.max).max(self.min) }

	/// The inner flex base size, which weights `flex-shrink` into the scaled
	/// shrink factor so a large item gives up more of an overflow than a small
	/// one.
	fn inner_base(&self) -> u32 { self.base.saturating_sub(self.overhead) }
}

impl FlexItem {
	/// Read `child`'s flex properties, `size` being its outer size before
	/// flexing (see [`FlexItem::size`]).
	///
	/// Lengths resolve to cells the way the box model resolves a `width` or
	/// `height` (1rem ≈ 1 cell), and CSS sizes are content-box here, so the
	/// item's overhead is added back, as [`explicit_box_size`] does.
	fn new(child: &CharcellNodeData, size: UVec2, frame: &FlexFrame) -> Self {
		let vertical = frame.vertical;
		let layout = child.layout_style();
		let overhead = main_of(
			BoxModel::from_node(child, frame.viewport).overhead(),
			vertical,
		);
		let outer = |length: Option<Length>| {
			length
				.and_then(|length| {
					axis_cells(length, frame.viewport, frame.main, vertical)
				})
				.map(|cells| cells.saturating_add(overhead))
		};
		// the `width`/`height`, `min-` and `max-` sizes along the main axis
		let (specified, min, max) = child
			.box_style()
			.map(|style| match vertical {
				true => (style.height, style.min_height, style.max_height),
				false => (style.width, style.min_width, style.max_width),
			})
			.unwrap_or_default();
		let min = outer(min);
		let basis = outer(layout.flex_basis);
		let mut item = Self {
			entity: child.entity,
			size,
			specified: outer(specified),
			main: FlexMain {
				base: basis.unwrap_or(main_of(size, vertical)),
				overhead,
				min: min.unwrap_or(overhead),
				max: outer(max).unwrap_or(u32::MAX),
				grow: layout.flex_grow.max(0.),
				shrink: layout.flex_shrink.max(0.),
			},
			auto_min: min.is_none(),
			order: layout.flex_order,
		};
		// a definite basis can sit below the content, so its hypothetical size
		// needs the automatic minimum up front
		if basis.is_some() {
			item.resolve_auto_min(child, frame);
		}
		item
	}

	/// Settle an `auto` min main size to the automatic minimum, once.
	fn resolve_auto_min(
		&mut self,
		child: &CharcellNodeData,
		frame: &FlexFrame,
	) {
		if std::mem::take(&mut self.auto_min) {
			self.main.min = self.automatic_min(child, frame);
		}
	}

	/// CSS's automatic minimum size along the main axis, outer: nothing for a
	/// scroll container, which clips and scrolls whatever it cannot show (a
	/// chat transcript yielding to its composer), else the smaller of the
	/// specified size and the min-content size capped by the max, so an item
	/// never shrinks into breaking a word (a row) or clipping its rows (a
	/// column).
	fn automatic_min(
		&self,
		child: &CharcellNodeData,
		frame: &FlexFrame,
	) -> u32 {
		if child.layout_style().clips() || child.is_scroll_container() {
			return self.main.overhead;
		}
		let content = match (frame.vertical, self.specified) {
			(false, _) => min_content_width(child, frame.query, frame.viewport),
			// a column item's content height is its size before flexing, unless
			// a `height` stood in for it
			(true, None) => self.size.y,
			(true, Some(_)) => {
				resolve_content_height(
					child,
					frame.query,
					&BoxModel::from_node(child, frame.viewport),
					self.size.x.min(frame.cross),
					frame.viewport,
					frame.port_rows,
				) + self.main.overhead
			}
		}
		.min(self.main.max);
		self.specified
			.map_or(content, |specified| specified.min(content))
	}
}

/// The in-flow children of `node` as flex items in `order`, each sized before
/// flexing by `size_of` (see [`FlexItem::size`]).
fn flex_items(
	node: &CharcellNodeData,
	frame: &FlexFrame,
	size_of: impl Fn(&CharcellNodeData) -> UVec2,
) -> Vec<FlexItem> {
	let mut items = node
		.flow_child_nodes(frame.query)
		.map(|child| FlexItem::new(&child, size_of(&child), frame))
		.collect::<Vec<_>>();
	items.sort_by_key(|item| item.order);
	items
}

// ── Measure pass ──────────────────────────────────────────────────────────────

/// Measure pass: compute the intrinsic size of this flexbox and its children.
///
/// `sizes` contains pre-measured intrinsic sizes for all children (bottom-up).
/// Each item contributes its hypothetical main size, so a `flex-basis` sizes
/// the container as it sizes the item.
pub(crate) fn measure_flex(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	sizes: &HashMap<Entity, UVec2>,
	available: UVec2,
	viewport: UVec2,
) -> Result<UVec2> {
	let flexbox = node.flexbox();
	let vertical = is_vertical(flexbox, viewport);
	let frame = FlexFrame {
		query,
		viewport,
		vertical,
		main: None,
		cross: cross_of(available, vertical),
		port_rows: available.y,
	};
	// an absolute child is no flex item and adds nothing to the measure
	let items = flex_items(node, &frame, |child| {
		sizes
			.get(&child.entity)
			.copied()
			.expect("unreachable, postorder populated sizes hashmap")
	});
	let lines =
		form_lines(items, flexbox, main_of(available, vertical), viewport);
	// lines stack along the cross axis: the main extent is the longest line, the
	// cross extent the sum of each line's largest item
	let line_gap = cross_gap(flexbox, vertical, viewport);
	let (main, cross) = lines.iter().enumerate().fold(
		(0u32, 0u32),
		|(main, cross), (idx, line)| {
			let line_main = line
				.iter()
				.map(|item| item.main.hypothetical())
				.fold(0u32, u32::saturating_add)
				+ line_gaps(flexbox, vertical, viewport, line.len());
			let line_cross = line
				.iter()
				.map(|item| cross_of(item.size, vertical))
				.max()
				.unwrap_or(0);
			let gap = if idx > 0 { line_gap } else { 0 };
			(main.max(line_main), cross.saturating_add(gap + line_cross))
		},
	);
	match vertical {
		true => UVec2::new(cross, main),
		false => UVec2::new(main, cross),
	}
	.xok()
}

// ── Width-constrained height resolution ────────────────────────────────────────

/// Total height (rows, including the node's own box overhead) the node needs
/// when its border box is constrained to `width` columns.
///
/// The [measure pass](measure_node) sizes heights at the unconstrained viewport
/// width, so a node later laid out into a narrower column (eg a paragraph beside
/// a sidebar) wraps into more rows than were reserved, and paint clips the tail.
/// The layout pass calls this with each node's *assigned* width so the reserved
/// height matches what paint will flow.
///
/// `port_rows` is the height of the nearest scroll port this node renders into
/// (the viewport when nothing clips), bounding a raster to a box its port can
/// actually show.
pub(super) fn resolve_height(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	width: u32,
	viewport: UVec2,
	port_rows: u32,
) -> u32 {
	let box_model = BoxModel::from_node(node, viewport);
	let content_height = resolve_content_height(
		node, query, &box_model, width, viewport, port_rows,
	);
	// an explicit `height` overrides the resolved content height; `min-height`
	// floors it (eg `100vh` to fill the terminal window) and `max-height` caps it.
	// A raster already folded its explicit height into the aspect-contained value
	// (see `resolve_content_height`), so it keeps that (only `min`/`max-height`
	// still clamp) rather than re-imposing the raw height.
	let base = if node.kitty_image().is_some() {
		content_height
	} else {
		box_model.height.unwrap_or(content_height)
	};
	box_model.clamp_height(base) + box_model.overhead().y
}

/// The rows `node`'s content needs inside a border box `width` columns wide,
/// before its own `box_model` overhead and its `height`, `min-height` and
/// `max-height` apply (see [`resolve_height`]).
fn resolve_content_height(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	box_model: &BoxModel,
	width: u32,
	viewport: UVec2,
	port_rows: u32,
) -> u32 {
	let content_width = width.saturating_sub(box_model.overhead().x);
	// a clipping node is its descendants' scroll port, but only once it has a
	// height of its own: an auto-height clip sizes to its content and so bounds
	// nothing, leaving its own port in force.
	let child_port_rows =
		match node.is_scroll_container() || node.layout_style().clips() {
			true => box_model.height.unwrap_or(port_rows),
			false => port_rows,
		};
	let content_height = match node.layout_style().display {
		Display::None => 0,
		// a bare text leaf inherits its ancestor's display (eg `flex` inside a
		// flex footer): resolve it as the text run it is, never as a container
		_ if node.is_text_leaf() => measure_text(node, content_width).y,
		Display::Flex => resolve_flex_height(
			node,
			query,
			content_width,
			viewport,
			child_port_rows,
		),
		Display::Grid => {
			measure_grid(node, query, UVec2::new(content_width, 0), viewport).y
		}
		Display::Table => resolve_table_height(
			node,
			query,
			content_width,
			viewport,
			child_port_rows,
		),
		// a kitty raster contains within the assigned width *and* its scroll port's
		// rows: it honors an explicit `height` only while that height's aspect-width
		// still fits `content_width` and the height itself fits the port, and past
		// that (eg a `70vh` hero on a narrow terminal) falls to a width-driven fit
		// so the picture stays inside the port and aspect-correct rather than
		// overflowing. The generic explicit-`height` override in `resolve_height`
		// is skipped for rasters so it cannot re-impose the overflowing height.
		_ if let Some(image) = node.kitty_image() => {
			image
				.cell_size_constrained(
					box_model.width,
					box_model.height,
					CellBounds::new(content_width, port_rows),
				)
				.y
		}
		// block leaf whose content is generated (eg `<hr>`, `<img>` alt, a
		// `<select>`'s label). Before the value branch, as in `measure_node`: a
		// `<select>` carries its submission value (a type path wider than the
		// control) in [`Value`], and its marker is what paints.
		_ if let Some(marker) =
			node.marker().filter(|_| !node.has_child_nodes(query)) =>
		{
			measure_scaled(node.visual_style(), marker, content_width).y
		}
		// text leaf (eg a paragraph's text node) or a control
		_ if node.has_own_text() => measure_text(node, content_width).y,
		// container of inline content: flow descendants as wrapped text
		_ if establishes_inline_flow(node, query) => {
			measure_inline_flow(node, query, content_width).y
		}
		// block container: stack children, each flowed at the constrained width
		_ => resolve_block_height(
			node,
			query,
			content_width,
			viewport,
			child_port_rows,
		),
	};
	// a horizontal scrollbar takes a row below the content it scrolls
	content_height + horizontal_gutter_rows(node, query, content_width)
}

/// Block flow height: children stack, each laid out at the content width (less
/// any marker gutter), mirroring [`block_layout_rects`].
fn resolve_block_height(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	content_width: u32,
	viewport: UVec2,
	port_rows: u32,
) -> u32 {
	let child_width = content_width.saturating_sub(marker_gutter(node, query));
	let children: Vec<_> = node.flow_child_nodes(query).collect();
	let last = children.len().saturating_sub(1);
	children
		.iter()
		.enumerate()
		.map(|(i, child)| {
			let height =
				resolve_height(child, query, child_width, viewport, port_rows)
					.max(1);
			// the last child's trailing margin doesn't reserve a row (matches
			// `measure_block` and `block_layout_rects`).
			match i == last {
				true => {
					height.saturating_sub(node_bottom_margin(child, viewport))
				}
				false => height,
			}
		})
		.sum()
}

/// Flex height: form lines at the constrained main width, then sum line heights
/// (row) or take the tallest column (column), recursing through each item.
fn resolve_flex_height(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	content_width: u32,
	viewport: UVec2,
	port_rows: u32,
) -> u32 {
	let flexbox = node.flexbox();
	let vertical = is_vertical(flexbox, viewport);
	let frame = FlexFrame {
		query,
		viewport,
		vertical,
		// a column's height is what this resolves, so not yet known
		main: (!vertical).then_some(content_width),
		// a row's cross axis is height, which is unbounded
		cross: if vertical { content_width } else { u32::MAX },
		port_rows,
	};
	let items = flex_items(node, &frame, |child| {
		let size = child.intrinsic_size();
		// a column item's main size is its content height at its width, as in
		// `flex_layout_rects`
		match vertical {
			true => {
				let (explicit_w, _) = explicit_box_size(
					child,
					viewport,
					UVec2::new(content_width, viewport.y),
				);
				let width = column_item_width(
					child,
					query,
					flexbox,
					explicit_w,
					size.x,
					content_width,
				);
				UVec2::new(
					size.x,
					resolve_height(child, query, width, viewport, port_rows),
				)
			}
			false => size,
		}
	});
	let container_main = match vertical {
		true => viewport.y,
		false => content_width,
	};
	let mut lines = form_lines(items, flexbox, container_main, viewport);
	match vertical {
		// columns sit side by side: height is the tallest column
		true => lines
			.iter()
			.map(|line| {
				line.iter()
					.map(|item| item.main.hypothetical())
					.fold(0u32, u32::saturating_add)
					+ line_gaps(flexbox, vertical, viewport, line.len())
			})
			.max()
			.unwrap_or(0),
		// rows stack: total height is the sum of each line's tallest item
		false => lines
			.iter_mut()
			.enumerate()
			.map(|(idx, line)| {
				let gap = if idx > 0 {
					cross_gap(flexbox, vertical, viewport)
				} else {
					0
				};
				let sizes =
					resolve_line_sizes(flexbox, line, &frame, content_width);
				gap + sizes.iter().map(|size| size.y).max().unwrap_or(0)
			})
			.sum(),
	}
}

/// The width a column item lays out at, which its height resolves against:
/// its `explicit` width, else the container's whole `cross` when it stretches
/// across a single line, else its `intrinsic` width, each within `cross`.
///
/// A stretched item's intrinsic width is no bound on what it lays out at: a
/// table's scroll wrapper measures a few cells wide, and the table it holds,
/// laid out full width, wraps into rows a height resolved at the narrow width
/// never reserved, so the next item painted over them.
fn column_item_width(
	child: &CharcellNodeData,
	query: &CharcellQuery,
	flexbox: &FlexBox,
	explicit: Option<u32>,
	intrinsic: u32,
	cross: u32,
) -> u32 {
	let stretches = flexbox.wrap == FlexWrap::NoWrap
		&& resolve_align(child.entity, query, flexbox.align_items)
			== AlignItems::Stretch;
	match (explicit, stretches) {
		(Some(width), _) => width.min(cross),
		(None, true) => cross,
		(None, false) => intrinsic.min(cross),
	}
}

/// A line's flexed item sizes, with each row item's height resolved at its
/// assigned width rather than the width it was measured at.
///
/// The main axis is [resolved](resolve_line_main) from the items' flex
/// factors. The cross axis follows the frame: a row item's height is its
/// content at its assigned width, and a column item's width is clamped to the
/// container, which the fixed-width terminal bounds. An item measured at the
/// unconstrained viewport width would otherwise overflow the narrower column
/// and, under `align-items: center`, centre against a width larger than it
/// actually has (the homepage hero drifting past the sidebar).
fn resolve_line_sizes(
	flexbox: &FlexBox,
	line: &mut [FlexItem],
	frame: &FlexFrame,
	container_main: u32,
) -> Vec<UVec2> {
	resolve_line_main(flexbox, line, frame, container_main)
		.into_iter()
		.zip(line.iter())
		.map(|(main, item)| match frame.vertical {
			true => UVec2::new(item.size.x.min(frame.cross), main),
			false => {
				let height = frame
					.query
					.unresolved_node(item.entity)
					.map(|child| {
						resolve_height(
							&child,
							frame.query,
							main,
							frame.viewport,
							frame.port_rows,
						)
					})
					.unwrap_or(item.size.y);
				UVec2::new(main, height)
			}
		})
		.collect()
}

// ── Layout pass ───────────────────────────────────────────────────────────────

/// Layout pass: assign a [`LayoutRect`] to each flex child.
///
/// Reads pre-computed [`IntrinsicSize`] from the query and writes
/// each child's rect into `layout_rects`.
/// `port_rows` is the scroll port height bounding the children laid out here
/// (see [`resolve_height`]).
pub(crate) fn flex_layout_rects(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	container_rect: IRect,
	viewport: UVec2,
	layout_rects: &mut HashMap<Entity, IRect>,
	port_rows: u32,
) -> Result {
	let flexbox = node.flexbox();

	let box_model = BoxModel::from_node(node, viewport);
	let content_rect =
		scrollport_of(node, query, box_model.content_rect(container_rect));
	// flex math works in unsigned cell sizes; the signed content rect origin is
	// re-added when each child rect is placed.
	let available = UVec2::new(
		content_rect.width().max(0) as u32,
		content_rect.height().max(0) as u32,
	);

	let direction = resolve_direction(flexbox.direction, viewport);
	let vertical = direction == Direction::Vertical;
	let container_main = main_of(available, vertical);
	let container_cross = cross_of(available, vertical);
	let frame = FlexFrame {
		query,
		viewport,
		vertical,
		main: Some(container_main),
		cross: container_cross,
		port_rows,
	};
	// Each item's size before flexing is its measured intrinsic size, overriding
	// either axis it sizes explicitly (eg a percent `width`, which the measure
	// pass left content-sized) with that resolved size.
	let items = flex_items(node, &frame, |child| {
		let (explicit_w, explicit_h) =
			explicit_box_size(child, viewport, available);
		// a raster is a replaced element: size it against *this line's*
		// available width (the measure pass could only clamp to the viewport,
		// which overflows a container narrower than the screen), aspect-
		// containing so a tall `height` never pushes it past the line.
		if let Some(image) = child.kitty_image() {
			return image.cell_size_constrained(
				explicit_w,
				explicit_h,
				CellBounds::new(available.x, port_rows),
			);
		}
		let intrinsic = child.intrinsic_size();
		let width = explicit_w.unwrap_or(intrinsic.x);
		// a column item's base is its content height at its assigned width, as
		// a row item's cross size resolves (`resolve_line_sizes`): the measure's
		// viewport-wide guess is too tall for a block holding a raster the
		// column contains, and too short for text that wraps
		let height = match (vertical, explicit_h) {
			(_, Some(height)) => height,
			(true, None) => resolve_height(
				child,
				query,
				column_item_width(
					child,
					query,
					flexbox,
					explicit_w,
					intrinsic.x,
					available.x,
				),
				viewport,
				port_rows,
			),
			(false, None) => intrinsic.y,
		};
		UVec2::new(width, height)
	});
	let mut lines = form_lines(items, flexbox, container_main, viewport);

	// Final per-line item sizes: flexed along the main axis, with each item's
	// cross size (a row's heights) resolved at its assigned width so a
	// stretched line is tall enough for its wrapped content (see `resolve_height`).
	let final_per_line: Vec<Vec<UVec2>> = lines
		.iter_mut()
		.map(|line| resolve_line_sizes(flexbox, line, &frame, container_main))
		.collect();
	let mut line_cross_sizes: Vec<u32> = final_per_line
		.iter()
		.map(|sizes| line_cross_size(sizes, vertical))
		.collect();
	// a single flex line fills the container's cross size (CSS: a single-line
	// container's line cross size is the container's inner cross size), so
	// `align-items: stretch` fills the container — the full-width header/footer and
	// the full-height sidebar rail — not just the content. Wrapped (multi-line)
	// flows keep their per-line content cross size, distributed by `align-content`.
	if let [only] = line_cross_sizes.as_mut_slice() {
		*only = (*only).max(container_cross);
	}

	let line_positions = apply_align_content(
		flexbox,
		&line_cross_sizes,
		container_cross,
		viewport,
	);
	// `align-content: stretch` shares the cross space the lines leave among them
	let stretch_bonus = container_cross
		.saturating_sub(line_cross_sizes.iter().sum::<u32>())
		.saturating_sub(
			cross_gap(flexbox, vertical, viewport)
				* (line_cross_sizes.len() as u32).saturating_sub(1),
		) / (line_cross_sizes.len() as u32).max(1);

	match direction {
		// ── Row layout ──────────────────────────────────────────────────────
		Direction::Horizontal => {
			for (line_idx, line) in lines.iter().enumerate() {
				let line_y =
					content_rect.min.y + line_positions[line_idx] as i32;
				let line_h = if flexbox.align_content == AlignContent::Stretch {
					line_cross_sizes[line_idx] + stretch_bonus
				} else {
					line_cross_sizes[line_idx]
				};

				if line_y >= content_rect.max.y {
					break;
				}

				let final_sizes = &final_per_line[line_idx];
				let main_positions =
					apply_justify(flexbox, final_sizes, available.x, viewport);

				for (item_idx, (item, fsize)) in
					line.iter().zip(final_sizes.iter()).enumerate()
				{
					let entity = item.entity;
					let align =
						resolve_align(entity, query, flexbox.align_items);
					let child_h = match align {
						AlignItems::Stretch => line_h,
						_ => fsize.y.min(line_h),
					};
					let child_y = line_y
						+ cross_offset(entity, query, flexbox, child_h, line_h)
							as i32;
					let child_x =
						content_rect.min.x + main_positions[item_idx] as i32;
					let child_rect = IRect::new(
						child_x,
						child_y,
						child_x + fsize.x as i32,
						child_y + child_h as i32,
					);
					layout_rects.insert(entity, child_rect);
				}
			}
		}

		// ── Col layout ──────────────────────────────────────────────────────
		Direction::Vertical => {
			for (line_idx, line) in lines.iter().enumerate() {
				let line_x =
					content_rect.min.x + line_positions[line_idx] as i32;
				let line_w = if flexbox.align_content == AlignContent::Stretch {
					line_cross_sizes[line_idx] + stretch_bonus
				} else {
					line_cross_sizes[line_idx]
				};

				if line_x >= content_rect.max.x {
					break;
				}

				let final_sizes = &final_per_line[line_idx];
				let main_positions =
					apply_justify(flexbox, final_sizes, available.y, viewport);

				for (item_idx, (item, fsize)) in
					line.iter().zip(final_sizes.iter()).enumerate()
				{
					let entity = item.entity;
					let align =
						resolve_align(entity, query, flexbox.align_items);
					let child_w = match align {
						AlignItems::Stretch => line_w,
						_ => fsize.x.min(line_w),
					};
					let child_x = line_x
						+ cross_offset(entity, query, flexbox, child_w, line_w)
							as i32;
					let child_y =
						content_rect.min.y + main_positions[item_idx] as i32;
					let child_rect = IRect::new(
						child_x,
						child_y,
						child_x + child_w as i32,
						child_y + fsize.y as i32,
					);
					layout_rects.insert(entity, child_rect);
				}
			}
		}
		_ => {
			unreachable!("resolve_direction should eliminate viewport variants")
		}
	}
	Ok(())
}

// ── Line formation ────────────────────────────────────────────────────────────

/// Break `order`-sorted items into flex lines: a wrapping container starts a
/// new line where the next item's hypothetical main size would overflow
/// `container_main`, a nowrap one keeps them all on one.
fn form_lines(
	items: Vec<FlexItem>,
	flexbox: &FlexBox,
	container_main: u32,
	viewport: UVec2,
) -> Vec<Vec<FlexItem>> {
	let gap = main_gap(flexbox, is_vertical(flexbox, viewport), viewport);
	let mut lines: Vec<Vec<FlexItem>> = vec![];
	let mut current: Vec<FlexItem> = vec![];
	let mut main_used = 0u32;

	for item in items {
		let item_main = item.main.hypothetical();
		let gap_cost = if current.is_empty() { 0 } else { gap };
		let overflows = flexbox.wrap == FlexWrap::Wrap
			&& !current.is_empty()
			&& main_used.saturating_add(gap_cost).saturating_add(item_main)
				> container_main;

		if overflows {
			lines.push(std::mem::take(&mut current));
			main_used = 0;
		} else {
			main_used = main_used.saturating_add(gap_cost);
		}
		main_used = main_used.saturating_add(item_main);
		current.push(item);
	}
	if !current.is_empty() {
		lines.push(current);
	}
	lines
}

/// A line's cross size: its largest item's.
fn line_cross_size(sizes: &[UVec2], vertical: bool) -> u32 {
	sizes
		.iter()
		.map(|size| cross_of(*size, vertical))
		.max()
		.unwrap_or(0)
}

fn resolve_align(
	entity: Entity,
	query: &CharcellQuery,
	default_align: AlignItems,
) -> AlignItems {
	let align_self = query
		.unresolved_node(entity)
		.map(|n| n.layout_style().align_self.clone())
		.unwrap_or(AlignSelf::Auto);
	match align_self {
		AlignSelf::Auto => default_align,
		AlignSelf::Start => AlignItems::Start,
		AlignSelf::Center => AlignItems::Center,
		AlignSelf::End => AlignItems::End,
		AlignSelf::Stretch => AlignItems::Stretch,
		AlignSelf::Baseline => unimplemented!(),
	}
}

fn cross_offset(
	entity: Entity,
	query: &CharcellQuery,
	flexbox: &FlexBox,
	child_cross: u32,
	line_cross: u32,
) -> u32 {
	let align = resolve_align(entity, query, flexbox.align_items);
	match align {
		AlignItems::Start | AlignItems::Stretch => 0,
		AlignItems::Center => line_cross.saturating_sub(child_cross) / 2,
		AlignItems::End => line_cross.saturating_sub(child_cross),
		AlignItems::Baseline => unimplemented!(),
	}
}

/// Resolve a line's outer main sizes from its items' flex factors, against
/// `container_main` less the gaps between them.
fn resolve_line_main(
	flexbox: &FlexBox,
	line: &mut [FlexItem],
	frame: &FlexFrame,
	container_main: u32,
) -> Vec<u32> {
	let gaps = line_gaps(flexbox, frame.vertical, frame.viewport, line.len());
	let space = container_main.saturating_sub(gaps);
	// only an overflowing line shrinks its items toward their minimums
	let hypothetical = line
		.iter()
		.map(|item| item.main.hypothetical() as u64)
		.sum::<u64>();
	if hypothetical > space as u64 {
		for item in line.iter_mut() {
			if let Ok(child) = frame.query.unresolved_node(item.entity) {
				item.resolve_auto_min(&child, frame);
			}
		}
	}
	line.iter()
		.map(|item| item.main)
		.collect::<Vec<_>>()
		.xmap(|items| resolve_flexible_lengths(&items, space))
}

/// CSS flexbox's resolution of flexible lengths (css-flexbox-1 §9.7): the
/// outer main sizes of one line of `items` sharing `space` cells.
///
/// A line whose hypothetical sizes fit grows, its free space shared in
/// proportion to `flex-grow`. A line that overflows shrinks, the overflow taken
/// in proportion to `flex-shrink` times each item's inner base size, so a large
/// item gives up more than a small one and a `flex-shrink: 0` item none. Flex
/// factors summing below one claim only that fraction of the free space. An
/// item flexed past its min or max is frozen there and the rest re-shared,
/// until none violates its bounds. Sizes resolve in fractional cells and then
/// [round](round_conserving) to whole ones still summing to the line's total.
fn resolve_flexible_lengths(items: &[FlexMain], space: u32) -> Vec<u32> {
	let space = space as f64;
	let hypothetical_total = items
		.iter()
		.map(|item| item.hypothetical() as f64)
		.sum::<f64>();
	let growing = hypothetical_total < space;
	let factor = |item: &FlexMain| match growing {
		true => item.grow as f64,
		false => item.shrink as f64,
	};
	// how strongly an item draws on the free space
	let weight = |item: &FlexMain| match growing {
		true => item.grow as f64,
		false => item.shrink as f64 * item.inner_base() as f64,
	};
	// an inflexible item holds its hypothetical size: no factor for this
	// direction, or a bound already pushing it the opposite way
	let mut frozen = items
		.iter()
		.map(|item| {
			factor(item) == 0.
				|| (growing && item.base > item.hypothetical())
				|| (!growing && item.base < item.hypothetical())
		})
		.collect::<Vec<_>>();
	let mut target = items
		.iter()
		.map(|item| item.hypothetical() as f64)
		.collect::<Vec<_>>();
	// the space left once frozen items take their target and the rest their base
	let free_space = |target: &[f64], frozen: &[bool]| {
		space
			- items
				.iter()
				.zip(target.iter().zip(frozen))
				.map(|(item, (target, frozen))| match frozen {
					true => *target,
					false => item.base as f64,
				})
				.sum::<f64>()
	};
	let initial_free = free_space(&target, &frozen);

	while frozen.contains(&false) {
		let unfrozen = || {
			items
				.iter()
				.zip(&frozen)
				.filter(|(_, frozen)| !**frozen)
				.map(|(item, _)| item)
		};
		let mut free = free_space(&target, &frozen);
		let factor_sum = unfrozen().map(factor).sum::<f64>();
		if factor_sum < 1. && (initial_free * factor_sum).abs() < free.abs() {
			free = initial_free * factor_sum;
		}
		// share the free space by weight, clamping each share into its bounds
		let weight_sum = unfrozen().map(weight).sum::<f64>();
		let mut violation = vec![0.; items.len()];
		for (idx, item) in items.iter().enumerate() {
			if frozen[idx] {
				continue;
			}
			let share = match weight_sum > 0. {
				true => free * weight(item) / weight_sum,
				false => 0.,
			};
			let flexed = item.base as f64 + share;
			let clamped = flexed.min(item.max as f64).max(item.min as f64);
			violation[idx] = clamped - flexed;
			target[idx] = clamped;
		}
		// freeze the items clamped the dominant way, or all once none were
		let total = violation.iter().sum::<f64>();
		for (idx, violation) in violation.iter().enumerate() {
			if total == 0. || violation * total > 0. {
				frozen[idx] = true;
			}
		}
	}
	round_conserving(&target)
}

/// Round fractional cell sizes to whole cells summing to their rounded total:
/// each takes its floor, then the cells the floors dropped go to the largest
/// fractional parts, the earlier item first on a tie.
fn round_conserving(sizes: &[f64]) -> Vec<u32> {
	let total = sizes.iter().sum::<f64>().round() as u64;
	let mut cells = sizes
		.iter()
		.map(|size| size.floor() as u32)
		.collect::<Vec<_>>();
	let dropped =
		total.saturating_sub(cells.iter().map(|cell| *cell as u64).sum());
	let fraction = |idx: usize| sizes[idx] - sizes[idx].floor();
	let mut order = (0..sizes.len()).collect::<Vec<_>>();
	order.sort_by(|a, b| fraction(*b).total_cmp(&fraction(*a)));
	for idx in order.into_iter().take(dropped as usize) {
		cells[idx] += 1;
	}
	cells
}

fn apply_justify(
	flexbox: &FlexBox,
	final_sizes: &[UVec2],
	container_main: u32,
	viewport: UVec2,
) -> Vec<u32> {
	let vertical = is_vertical(flexbox, viewport);
	let gap = main_gap(flexbox, vertical, viewport);
	let gap_total = line_gaps(flexbox, vertical, viewport, final_sizes.len());

	let total_main: u32 =
		final_sizes.iter().map(|s| main_of(*s, vertical)).sum();

	let free = container_main.saturating_sub(total_main + gap_total);

	let mut positions = Vec::new();
	match flexbox.justify_content {
		JustifyContent::Start => {
			let mut pos = 0;
			for size in final_sizes {
				positions.push(pos);
				pos += main_of(*size, vertical) + gap;
			}
		}
		JustifyContent::End => {
			let mut pos = free;
			for size in final_sizes {
				positions.push(pos);
				pos += main_of(*size, vertical) + gap;
			}
		}
		JustifyContent::Center => {
			let mut pos = free / 2;
			for size in final_sizes {
				positions.push(pos);
				pos += main_of(*size, vertical) + gap;
			}
		}
		JustifyContent::SpaceBetween => {
			if final_sizes.len() <= 1 {
				positions.push(0);
			} else {
				let spacing = free / (final_sizes.len() as u32 - 1);
				let mut pos = 0;
				for size in final_sizes {
					positions.push(pos);
					pos += main_of(*size, vertical) + spacing;
				}
			}
		}
		JustifyContent::SpaceAround => {
			let spacing = free / final_sizes.len() as u32;
			let mut pos = spacing / 2;
			for size in final_sizes {
				positions.push(pos);
				pos += main_of(*size, vertical) + spacing;
			}
		}
		JustifyContent::SpaceEvenly => {
			let spacing = free / (final_sizes.len() as u32 + 1);
			let mut pos = spacing;
			for size in final_sizes {
				positions.push(pos);
				pos += main_of(*size, vertical) + spacing;
			}
		}
	}
	positions
}

fn apply_align_content(
	flexbox: &FlexBox,
	line_cross_sizes: &[u32],
	container_cross: u32,
	viewport: UVec2,
) -> Vec<u32> {
	let line_gap = cross_gap(flexbox, is_vertical(flexbox, viewport), viewport);
	let gap_total =
		line_gap * (line_cross_sizes.len() as u32).saturating_sub(1);
	let total_cross: u32 = line_cross_sizes.iter().sum();
	let free = container_cross.saturating_sub(total_cross + gap_total);

	let mut positions = Vec::new();
	match flexbox.align_content {
		AlignContent::Start => {
			let mut pos = 0;
			for &size in line_cross_sizes {
				positions.push(pos);
				pos += size + line_gap;
			}
		}
		AlignContent::End => {
			let mut pos = free;
			for &size in line_cross_sizes {
				positions.push(pos);
				pos += size + line_gap;
			}
		}
		AlignContent::Center => {
			let mut pos = free / 2;
			for &size in line_cross_sizes {
				positions.push(pos);
				pos += size + line_gap;
			}
		}
		AlignContent::SpaceBetween => {
			if line_cross_sizes.len() <= 1 {
				positions.push(0);
			} else {
				let spacing = free / (line_cross_sizes.len() as u32 - 1);
				let mut pos = 0;
				for &size in line_cross_sizes {
					positions.push(pos);
					pos += size + spacing;
				}
			}
		}
		AlignContent::SpaceAround => {
			let spacing = free / line_cross_sizes.len() as u32;
			let mut pos = spacing / 2;
			for &size in line_cross_sizes {
				positions.push(pos);
				pos += size + spacing;
			}
		}
		AlignContent::Stretch => {
			let bonus = free / line_cross_sizes.len() as u32;
			let mut pos = 0;
			for &size in line_cross_sizes {
				positions.push(pos);
				pos += size + bonus + line_gap;
			}
		}
	}
	positions
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::prelude::*;
	use crate::style::*;

	fn render(bundle: impl Bundle) -> String {
		Buffer::render_oneshot_sized(UVec2::new(40, 20), bundle)
			.trim_lines()
			.replace(" ", "+")
	}

	fn bordered() -> BoxStyle {
		BoxStyle::default().with_border(Spacing::all(Length::Px(1.)))
	}

	/// A thick border (eg the blockquote callout's left rule) draws with the
	/// heavy box-drawing glyph; a thin one stays light.
	#[beet_core::test]
	fn thick_border_draws_heavy() {
		render((LayoutStyle::flex_row(), children![(
			rsx! {"Q"},
			BoxStyle::default()
				.with_border(Spacing {
					left: Length::Px(3.),
					..Spacing::DEFAULT
				})
				// padding gives the callout the height a border needs to draw
				.with_padding(Spacing::all(Length::Rem(1.)))
		)]))
		.xpect_contains("┃")
		.xnot()
		.xpect_contains("│");
	}

	#[beet_core::test]
	fn justify_start() {
		render((
			LayoutStyle::flex_row()
				.justify_content(JustifyContent::Start)
				.column_gap(Length::Rem(1.)),
			children![
				(rsx! {"A"}, bordered()),
				(rsx! {"B"}, bordered()),
				(rsx! {"C"}, bordered()),
			],
		))
		.xpect_snapshot();
	}

	#[beet_core::test]
	fn justify_end() {
		render((
			LayoutStyle::flex_row()
				.justify_content(JustifyContent::End)
				.column_gap(Length::Rem(1.)),
			children![
				(rsx! {"A"}, bordered()),
				(rsx! {"B"}, bordered()),
				(rsx! {"C"}, bordered()),
			],
		))
		.xpect_snapshot();
	}

	#[beet_core::test]
	fn justify_center() {
		render((
			LayoutStyle::flex_row()
				.justify_content(JustifyContent::Center)
				.column_gap(Length::Rem(1.)),
			children![
				(rsx! {"A"}, bordered()),
				(rsx! {"B"}, bordered()),
				(rsx! {"C"}, bordered()),
			],
		))
		.xpect_snapshot();
	}

	#[beet_core::test]
	fn column_gap() {
		render((
			LayoutStyle::flex_row().column_gap(Length::Rem(3.)),
			children![(rsx! {"A"}, bordered()), (rsx! {"B"}, bordered()),],
		))
		.xpect_snapshot();
	}

	#[beet_core::test]
	fn flex_grow_distributes_space() {
		let output = render((
			LayoutStyle::flex_row().column_gap(Length::Rem(1.)),
			children![
				(rsx! {"A"}, bordered()),
				(
					rsx! {"B"},
					bordered(),
					LayoutStyle::default().with_flex_grow(1.)
				),
				(rsx! {"C"}, bordered()),
			],
		));
		output.xpect_snapshot();
		// B should be wider than A and C
		let lines: Vec<&str> = output.lines().collect();
		let top = lines[0];
		// count dashes in each box segment to verify B is wider
		let segments: Vec<&str> = top.split('+').collect();
		segments.len().xpect_eq(3); // should have 3 boxes
		let b_width = segments[1].len();
		let a_width = segments[0].len();
		(b_width > a_width).xpect_true();
	}

	#[beet_core::test]
	fn align_items_center() {
		render((
			LayoutStyle::flex_row()
				.align_items(AlignItems::Center)
				.column_gap(Length::Rem(1.)),
			children![
				(
					LayoutStyle::flex_col(),
					children![
						(rsx! {"Tall"}, bordered()),
						(rsx! {"Item"}, bordered()),
					],
					bordered()
				),
				(rsx! {"Short"}, bordered()),
			],
		))
		.xpect_snapshot();
	}

	#[beet_core::test]
	fn align_items_start() {
		render((
			LayoutStyle::flex_row()
				.align_items(AlignItems::Start)
				.column_gap(Length::Rem(1.)),
			children![
				(
					LayoutStyle::flex_col(),
					children![
						(rsx! {"Tall"}, bordered()),
						(rsx! {"Item"}, bordered()),
					],
					bordered()
				),
				(rsx! {"Short"}, bordered()),
			],
		))
		.xpect_snapshot();
	}

	#[beet_core::test]
	fn align_items_end() {
		render((
			LayoutStyle::flex_row()
				.align_items(AlignItems::End)
				.column_gap(Length::Rem(1.)),
			children![
				(
					LayoutStyle::flex_col(),
					children![
						(rsx! {"Tall"}, bordered()),
						(rsx! {"Item"}, bordered()),
					],
					bordered()
				),
				(rsx! {"Short"}, bordered()),
			],
		))
		.xpect_snapshot();
	}

	#[beet_core::test]
	fn nested_flex() {
		render(
			(LayoutStyle::flex_col().row_gap(Length::Rem(1.)), children![
				(
					LayoutStyle::flex_row().column_gap(Length::Rem(1.)),
					children![
						(rsx! {"A"}, bordered()),
						(rsx! {"B"}, bordered()),
					],
					bordered()
				),
				(
					LayoutStyle::flex_row().column_gap(Length::Rem(1.)),
					children![
						(rsx! {"C"}, bordered()),
						(rsx! {"D"}, bordered()),
					],
					bordered()
				),
			]),
		)
		.xpect_snapshot();
	}

	#[beet_core::test]
	fn padding_with_content() {
		render((LayoutStyle::flex_row(), children![(
			rsx! {"X"},
			BoxStyle::default()
				.with_border(Spacing::all(Length::Px(1.)))
				.with_padding(Spacing::all(Length::Rem(0.5)))
		)]))
		.xpect_snapshot();
	}

	#[beet_core::test]
	fn column_with_multiple_items() {
		render(
			(LayoutStyle::flex_col().row_gap(Length::Rem(1.)), children![
				(rsx! {"First"}, bordered()),
				(rsx! {"Second"}, bordered()),
				(rsx! {"Third"}, bordered()),
			]),
		)
		.xpect_snapshot();
	}

	#[beet_core::test]
	fn nested_column_in_row() {
		render((LayoutStyle::flex_row(), children![(
			LayoutStyle::flex_col(),
			children![
				(rsx! {"A"}, bordered()),
				(rsx! {"B"}, bordered()),
				(rsx! {"C"}, bordered()),
			],
			bordered()
		)]))
		.xpect_snapshot();
	}

	#[beet_core::test]
	fn column_without_gap() {
		render((LayoutStyle::flex_col(), children![
			(rsx! {"A"}, bordered()),
			(rsx! {"B"}, bordered()),
		]))
		.xpect_snapshot();
	}

	/// Nested rows and columns with background colors on leaf nodes.
	/// Verifies background ordering and multi-level flex layout.
	#[beet_core::test]
	fn nested_with_backgrounds() {
		let out = render((
			LayoutStyle::flex_col().row_gap(Length::Rem(1.)),
			children![
				(
					LayoutStyle::flex_row().column_gap(Length::Rem(1.)),
					children![
						(rsx! { "Left" }, bordered(), VisualStyle {
							background: Some(Color::srgb(0.2, 0.2, 0.5)),
							..default()
						},),
						(rsx! { "Right" }, bordered(), VisualStyle {
							background: Some(Color::srgb(0.5, 0.2, 0.2)),
							..default()
						},),
					],
					bordered()
				),
				(
					LayoutStyle::flex_row().column_gap(Length::Rem(1.)),
					children![(rsx! { "Below" }, bordered(), VisualStyle {
						background: Some(Color::srgb(0.2, 0.5, 0.2)),
						..default()
					},),],
					bordered()
				),
			],
		));
		// both rows should appear - header row and second row
		out.as_str().xpect_contains("┌"); // at least one top-left corner
		// ensure trim_lines worked (output should not include excess blank rows)
		(out.lines().count() <= 12).xpect_true();
		// both rows rendered: check for both content strings
		out.as_str().xpect_contains("Left");
		out.xpect_contains("Below");
	}

	// ── Flexible lengths ──

	/// Each element's laid-out rect, by tag, when `content` renders into a
	/// `size` buffer under `rules`, so the flex props reach the engine through
	/// the cascade as an authored page's do.
	fn rects(
		size: UVec2,
		rules: Vec<Rule>,
		content: impl Bundle,
	) -> HashMap<String, IRect> {
		let mut world = CharcellPlugin::world();
		world.get_resource_or_init::<RuleSet>().extend_rules(rules);
		world.spawn((Buffer::new(size).into_double_buffer(), content));
		world.run_schedule(PostParseTree);
		world
			.run_system_once(|query: Query<(&Element, &LayoutRect)>| {
				query
					.iter()
					.map(|(element, rect)| (element.tag().to_string(), rect.0))
					.collect::<HashMap<_, _>>()
			})
			.unwrap()
	}

	/// The `<section>` flex row every case lays its items out in.
	fn row_rule() -> Rule {
		Rule::tag("section")
			.with_value(common_props::DisplayProp, Display::Flex)
	}

	/// The widths of a `width`-cell flex row's `<nav>` and `<article>`, styled
	/// by `nav` and `article` and labelled `nav_label` and `"Article"`.
	fn pair_widths(
		width: u32,
		nav: Rule,
		article: Rule,
		nav_label: &'static str,
	) -> (i32, i32) {
		let rects = rects(
			UVec2::new(width, 8),
			vec![row_rule(), nav, article],
			rsx! { <section><nav>{nav_label}</nav><article>"Article"</article></section> },
		);
		(rects["nav"].width(), rects["article"].width())
	}

	fn basis(tag: &str, rem: f32) -> Rule {
		Rule::tag(tag).with_value(common_props::FlexBasisProp, Length::Rem(rem))
	}

	#[beet_core::test]
	fn basis_holds_with_room() {
		// content would size these 3 and 7 cells wide
		pair_widths(60, basis("nav", 30.), basis("article", 20.), "Nav")
			.xpect_eq((30, 20));
	}

	/// The 10-cell overflow comes out of each item in proportion to
	/// `flex-shrink` times its base: 30 gives up 6, 20 gives up 4.
	#[beet_core::test]
	fn shrink_weighs_overflow_by_basis() {
		pair_widths(40, basis("nav", 30.), basis("article", 20.), "Nav")
			.xpect_eq((24, 16));
	}

	#[beet_core::test]
	fn zero_shrink_holds_its_basis() {
		let nav =
			basis("nav", 30.).with_value(common_props::FlexShrinkProp, 0_f32);
		pair_widths(40, nav, basis("article", 20.), "Nav").xpect_eq((30, 10));
	}

	/// The 20 free cells are shared 1:3 on top of each basis.
	#[beet_core::test]
	fn grow_shares_free_space_from_basis() {
		let grow = |tag: &str, factor: f32| {
			basis(tag, 10.).with_value(common_props::FlexGrowProp, factor)
		};
		pair_widths(40, grow("nav", 1.), grow("article", 3.), "Nav")
			.xpect_eq((15, 25));
		// factors summing below one claim only that fraction of the free space
		pair_widths(40, grow("nav", 0.5), basis("article", 10.), "Nav")
			.xpect_eq((20, 10));
	}

	/// An item shrinks no further than its longest word (the automatic
	/// minimum), the rest of the overflow coming out of its sibling; a scroll
	/// container has no such floor and clips instead.
	#[beet_core::test]
	fn shrink_stops_at_min_content() {
		let word = "Supercalifragilistic";
		pair_widths(30, Rule::tag("nav"), basis("article", 20.), word)
			.xpect_eq((20, 10));
		let clipped = Rule::tag("nav")
			.with_value(common_props::OverflowXProp, Overflow::Hidden);
		pair_widths(30, clipped, basis("article", 20.), word)
			.xpect_eq((15, 15));
	}

	/// Three growers splitting 7 free cells resolve to 2⅓ each: rounding hands
	/// the dropped cell to the first rather than leaving the line short.
	#[beet_core::test]
	fn grow_rounding_conserves_the_line() {
		let grow = |tag: &str| {
			Rule::tag(tag).with_value(common_props::FlexGrowProp, 1_f32)
		};
		let rects = rects(
			UVec2::new(10, 4),
			vec![row_rule(), grow("nav"), grow("article"), grow("aside")],
			rsx! { <section><nav>"A"</nav><article>"B"</article><aside>"C"</aside></section> },
		);
		["nav", "article", "aside"]
			.map(|tag| rects[tag].width())
			.xpect_eq([4, 3, 3]);
	}

	/// A column too short for its items shrinks the scroll container, whose
	/// automatic minimum is zero, and holds the other at its content height:
	/// a chat transcript yields to the composer below it.
	#[beet_core::test]
	fn column_shrinks_the_scroll_container() {
		let rects = rects(
			UVec2::new(20, 12),
			vec![
				Rule::tag("section")
					.with_value(common_props::DisplayProp, Display::Flex)
					.with_value(
						common_props::FlexDirectionProp,
						Direction::Vertical,
					)
					.with_value(common_props::Height, Length::Rem(6.)),
				Rule::tag("nav")
					.with_value(common_props::OverflowYProp, Overflow::Hidden),
			],
			// the root fills the buffer, so the sized column sits beneath it
			rsx! {
				<div><section>
					<nav><pre>"1\n2\n3\n4\n5\n6\n7\n8\n9\n10"</pre></nav>
					<article><pre>"a\nb"</pre></article>
				</section></div>
			},
		);
		(rects["nav"].height(), rects["article"].height()).xpect_eq((4, 2));
	}

	/// Text directly in a flex container is an anonymous item with `flex: 0 1
	/// auto`: it resolves through its element's rules, yet never takes that
	/// element's own basis (a bare `<main>"Content"</main>` once laid its text
	/// out 72 rows tall in the column).
	#[beet_core::test]
	fn text_item_ignores_its_container_basis() {
		let rects = rects(
			UVec2::new(40, 12),
			vec![
				Rule::tag("section")
					.with_value(common_props::DisplayProp, Display::Flex)
					.with_value(
						common_props::FlexDirectionProp,
						Direction::Vertical,
					)
					.with_value(common_props::FlexBasisProp, Length::Rem(8.)),
			],
			rsx! { <div><section>"Text"</section></div> },
		);
		rects["section"].height().xpect_eq(1);
	}

	/// A tag-less node kept in the flow (here by matching its element's
	/// background, as a layout's `<Slot>` under the page body does) lays its
	/// children out with that element's container props, yet takes none of the
	/// element's own place in its parent's line.
	#[beet_core::test]
	fn fragment_lays_out_as_its_element() {
		let rects = rects(
			UVec2::new(40, 12),
			vec![
				Rule::tag("section")
					.with_value(common_props::DisplayProp, Display::Flex)
					.with_value(
						common_props::FlexDirectionProp,
						Direction::Vertical,
					)
					.with_value(
						common_props::AlignItemsProp,
						AlignItems::Stretch,
					)
					.with_value(common_props::FlexBasisProp, Length::Rem(8.))
					.with_value(
						common_props::BackgroundColor,
						Color::srgb(0.2, 0.2, 0.5),
					),
			],
			// the bundle child is a tag-less node holding the header
			rsx! {
				<div><section>
					{children![rsx! { <header>"Header"</header> }]}
				</section></div>
			},
		);
		// stretched across the column by the fragment, as by the section
		rects["header"].width().xpect_eq(40);
		// the section's own basis sizes no box inside it
		rects["section"].height().xpect_eq(1);
	}

	/// Wide CJK chars each occupy 2 terminal columns.
	/// The border width must account for the display width, not the char count.
	#[beet_core::test]
	fn wide_chars_layout() {
		let out = render((
			LayoutStyle::flex_row().column_gap(Length::Rem(1.)),
			children![
				(rsx! { "中文" }, bordered()),
				(rsx! { "ＡＢＣ" }, bordered()),
			],
		));
		// "中文": 2 wide chars = 4 cols content → border top = ┌────┐
		out.as_str().xpect_contains("┌────┐");
		// "ＡＢＣ": 3 wide chars = 6 cols content → border top = ┌──────┐
		out.xpect_contains("┌──────┐");
	}
}
