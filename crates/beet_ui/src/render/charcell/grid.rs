//! Grid layout (`display: grid`): children flow row-major into equal-width
//! column tracks.
//!
//! The track configuration is [`GridTracks`]: a column count ("width",
//! defaulting to the conventional 12) and a row track height ("height",
//! defaulting to square — half the column width, terminal cells being about
//! twice as tall as wide). Gaps come from the shared CSS `gap` values.

use super::*;
use beet_core::prelude::*;
use bevy::math::IRect;
use bevy::math::UVec2;

/// Per-column geometry for a grid's content box: equal tracks with the
/// remainder spread across the leading columns, gaps between.
struct GridGeometry {
	columns: u32,
	column_gap: u32,
	track: u32,
	remainder: u32,
	row_height: u32,
}

impl GridGeometry {
	fn new(node: &CharcellNodeData, content_width: u32, viewport: UVec2) -> Self {
		let grid = &node.layout_style().grid;
		let columns = grid.columns.0.max(1);
		let column_gap = node.flexbox().column_gap_cells(viewport);
		let track_total =
			content_width.saturating_sub(column_gap * (columns - 1));
		let track = track_total / columns;
		Self {
			columns,
			column_gap,
			track,
			remainder: track_total % columns,
			row_height: grid.rows.cells(track, viewport),
		}
	}

	/// The x offset of column `col` within the content box.
	fn x_offset(&self, col: u32) -> u32 {
		self.track * col + col.min(self.remainder) + self.column_gap * col
	}

	/// The width of column `col` (leading columns absorb the remainder).
	fn width(&self, col: u32) -> u32 {
		self.track + (col < self.remainder) as u32
	}
}

/// Measure a grid container: full available width, rows sized by the track
/// height and the row-major child count.
pub(super) fn measure_grid(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	available: UVec2,
	viewport: UVec2,
) -> UVec2 {
	let geometry = GridGeometry::new(node, available.x, viewport);
	let count = node.flow_child_nodes(query).count() as u32;
	let rows = count.div_ceil(geometry.columns);
	let row_gap = node.flexbox().row_gap_cells(viewport);
	let height = rows * geometry.row_height + row_gap * rows.saturating_sub(1);
	UVec2::new(available.x, height)
}

/// Grid flow: assign each in-flow child its row-major track cell.
pub(super) fn grid_layout_rects(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	container_rect: IRect,
	viewport: UVec2,
	layout_rects: &mut HashMap<Entity, IRect>,
) -> Result {
	let box_model = BoxModel::from_node(node, viewport);
	let content_rect =
		scrollport_of(node, query, box_model.content_rect(container_rect));
	let geometry =
		GridGeometry::new(node, content_rect.width().max(0) as u32, viewport);
	let row_gap = node.flexbox().row_gap_cells(viewport);
	// a scroll container lays out its full overflow region; otherwise rows
	// past the content box are dropped like block flow
	let scrolls = node.is_scroll_container();

	let mut col = 0u32;
	let mut row_y = content_rect.min.y;
	for child in node.flow_child_nodes(query) {
		if !scrolls && row_y >= content_rect.max.y {
			break;
		}
		let x = content_rect.min.x + geometry.x_offset(col) as i32;
		let child_rect = IRect::new(
			x,
			row_y,
			x + geometry.width(col) as i32,
			row_y + geometry.row_height as i32,
		);
		layout_rects.insert(child.entity, child_rect);
		col += 1;
		if col == geometry.columns {
			col = 0;
			row_y += (geometry.row_height + row_gap) as i32;
		}
	}
	Ok(())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::style::*;
	use beet_core::prelude::*;
	use bevy::math::IRect;
	use bevy::math::UVec2;

	/// A `size`-cell world whose root is a `.grid` container with `count`
	/// labelled span children, laid out under `rules`. Returns the world and
	/// the spans in document order.
	fn grid_world(
		size: UVec2,
		rules: Vec<Rule>,
		count: usize,
	) -> (World, Vec<Entity>) {
		let mut world = CharcellPlugin::world();
		world.get_resource_or_init::<RuleSet>().extend_rules(rules);
		let grid = world
			.spawn((
				Buffer::new(size).into_double_buffer(),
				Element::new("div"),
				Classes::new([ClassName::string("grid")]),
			))
			.id();
		let spans = (0..count)
			.map(|i| {
				world
					.spawn((
						Element::new("span").with_inner_text(&i.to_string()),
						ChildOf(grid),
					))
					.id()
			})
			.collect();
		world.run_schedule(PostParseTree);
		(world, spans)
	}

	fn rect(world: &World, entity: Entity) -> IRect {
		world.get::<LayoutRect>(entity).unwrap().0
	}

	fn grid_rule() -> Rule {
		Rule::class("grid").with_value(common_props::DisplayProp, Display::Grid)
	}

	/// Children flow row-major into the default 12 columns: a 24-wide grid
	/// gives 2-cell tracks, the 13th child wrapping to the second row.
	#[beet_core::test]
	fn default_twelve_columns_wrap_row_major() {
		let (world, spans) =
			grid_world(UVec2::new(24, 10), vec![grid_rule()], 13);
		// 24 cols / 12 tracks = 2 cells each; square rows = 2/2 = 1 tall
		rect(&world, spans[0]).xpect_eq(IRect::new(0, 0, 2, 1));
		rect(&world, spans[1]).xpect_eq(IRect::new(2, 0, 4, 1));
		rect(&world, spans[11]).xpect_eq(IRect::new(22, 0, 24, 1));
		// the 13th wraps to the second row's first track
		rect(&world, spans[12]).xpect_eq(IRect::new(0, 1, 2, 2));
	}

	/// Cells authored as a collected `Vec` (eg `{(0..4).map(cell).collect()}`) sit
	/// under a tag-less grouping wrapper: an entity with [`Children`] but no
	/// [`Element`], the shape `Vec::into_snippet` lowers a collected child position
	/// to. The wrapper must be spliced out so the cells flow as direct grid tracks;
	/// otherwise the grid lays out the single wrapper as one cell and the cells
	/// collapse into a zero-width nested grid (the `bsx_site` grid demo
	/// regression). This mirrors the HTML walker, which emits no tag for such a
	/// node yet still renders its children.
	#[beet_core::test]
	fn fragment_wrapped_cells_flow_as_tracks() {
		let mut world = CharcellPlugin::world();
		world.get_resource_or_init::<RuleSet>().extend_rules(vec![grid_rule()]);
		// a `.grid` whose four cells are nested under one tag-less wrapper (children,
		// no `Element`), exactly the shape `Vec::into_snippet` lowers to.
		let grid = world
			.spawn((
				Buffer::new(UVec2::new(24, 6)).into_double_buffer(),
				Element::new("div"),
				Classes::new([ClassName::string("grid")]),
			))
			.id();
		let fragment = world.spawn(ChildOf(grid)).id();
		let spans = (0..4)
			.map(|i| {
				world
					.spawn((
						Element::new("span").with_inner_text(&i.to_string()),
						ChildOf(fragment),
					))
					.id()
			})
			.collect::<Vec<_>>();
		world.run_schedule(PostParseTree);

		// the cells lay out into the default 12 tracks (24 / 12 = 2-cell tracks),
		// side by side on the first row, not collapsed under the wrapper.
		rect(&world, spans[0]).xpect_eq(IRect::new(0, 0, 2, 1));
		rect(&world, spans[1]).xpect_eq(IRect::new(2, 0, 4, 1));
		rect(&world, spans[3]).xpect_eq(IRect::new(6, 0, 8, 1));
	}

	/// The same regression end to end through the real `rsx!` lowering: a collected
	/// `Vec` child position (`{(..).map(..).collect()}`) is what the `bsx_site`
	/// grid demo authors, and `Vec::into_snippet` lowers it to the tag-less wrapper.
	/// Each cell must paint at its own track, not collapse into one wrapper cell.
	#[beet_core::test]
	fn collected_vec_grid_cells_paint_as_tracks() {
		let mut world = CharcellPlugin::world();
		world.get_resource_or_init::<RuleSet>().extend_rules(vec![grid_rule()]);
		// 12 cols over 12 cells = 1-cell tracks, so each digit lands on its own column
		world.spawn((
			Buffer::new(UVec2::new(12, 4)).into_double_buffer(),
			rsx! {
				<div class="grid">
					{(0..12).map(|i| rsx! { <span>{i % 10}</span> }).collect::<Vec<_>>()}
				</div>
			},
		));
		world.run_schedule(PostParseTree);
		let buffer = world
			.query::<&DoubleBuffer>()
			.iter(&world)
			.next()
			.unwrap()
			.current_buffer();
		let glyph_at = |x: u32| {
			buffer
				.get(UVec2::new(x, 0))
				.map(|cell| cell.symbol_str().to_string())
				.unwrap_or_default()
		};
		// each cell occupies one track, so columns 0..10 read the cell digits in
		// order (collapsed-into-one-wrapper would paint only the first at column 0)
		glyph_at(0).xpect_eq("0");
		glyph_at(1).xpect_eq("1");
		glyph_at(9).xpect_eq("9");
	}

	/// An adjustable column count: 4 tracks split a 20-wide grid into 5-cell
	/// columns, and square rows stand half the track width tall.
	#[beet_core::test]
	fn adjustable_columns_and_square_rows() {
		let (world, spans) = grid_world(
			UVec2::new(20, 12),
			vec![grid_rule().with_value(
				common_props::GridTemplateColumnsProp,
				GridColumns(4),
			)],
			5,
		);
		// 20 / 4 = 5-wide tracks, square rows: 5 / 2 = 2 tall
		rect(&world, spans[0]).xpect_eq(IRect::new(0, 0, 5, 2));
		rect(&world, spans[3]).xpect_eq(IRect::new(15, 0, 20, 2));
		// the fifth wraps below the square track
		rect(&world, spans[4]).xpect_eq(IRect::new(0, 2, 5, 4));
	}

	/// An adjustable row height: an explicit `grid-auto-rows` length overrides
	/// the square default.
	#[beet_core::test]
	fn adjustable_row_height() {
		let (world, spans) = grid_world(
			UVec2::new(20, 12),
			vec![
				grid_rule()
					.with_value(
						common_props::GridTemplateColumnsProp,
						GridColumns(2),
					)
					.with_value(
						common_props::GridAutoRowsProp,
						GridRows::Length(Length::Rem(3.)),
					),
			],
			3,
		);
		rect(&world, spans[0]).xpect_eq(IRect::new(0, 0, 10, 3));
		rect(&world, spans[2]).xpect_eq(IRect::new(0, 3, 10, 6));
	}

	/// Gaps separate tracks, deducted from the track space before division.
	#[beet_core::test]
	fn gaps_separate_tracks() {
		let (world, spans) = grid_world(
			UVec2::new(21, 14),
			vec![
				grid_rule()
					.with_value(
						common_props::GridTemplateColumnsProp,
						GridColumns(2),
					)
					.with_value(common_props::ColumnGapProp, Length::Rem(1.))
					.with_value(common_props::RowGapProp, Length::Rem(1.)),
			],
			3,
		);
		// 21 - 1 gap = 20 track cells: 2 tracks of 10, the gap between them
		rect(&world, spans[0]).xpect_eq(IRect::new(0, 0, 10, 5));
		rect(&world, spans[1]).xpect_eq(IRect::new(11, 0, 21, 5));
		// second row offset by the square row height (10/2) plus the row gap
		rect(&world, spans[2]).xpect_eq(IRect::new(0, 6, 10, 11));
	}

	/// The grid paints its children at their track cells (an all-inline child
	/// list must not collapse the container into an inline formatting context).
	#[beet_core::test]
	fn paints_children_in_track_cells() {
		let (mut world, _) =
			grid_world(UVec2::new(12, 4), vec![grid_rule().with_value(
				common_props::GridTemplateColumnsProp,
				GridColumns(3),
			)], 3);
		let buffer = world
			.query::<&DoubleBuffer>()
			.iter(&world)
			.next()
			.unwrap()
			.current_buffer();
		// 12 / 3 = 4-wide tracks: each label paints at its track's origin
		let glyph_at = |x: u32| {
			buffer
				.get(UVec2::new(x, 0))
				.map(|cell| cell.symbol_str().to_string())
				.unwrap_or_default()
		};
		glyph_at(0).xpect_eq("0");
		glyph_at(4).xpect_eq("1");
		glyph_at(8).xpect_eq("2");
	}
}
