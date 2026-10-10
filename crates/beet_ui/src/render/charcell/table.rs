//! Table layout for the charcell engine.
//!
//! A `<table>` ([`Display::Table`]) is laid out as a column-aligned grid: every
//! column takes the width of its widest cell (narrowed to fit the table, see
//! [`column_widths`]), and every row is as tall as its tallest cell. Rows and
//! row groups carry no display of their own — a *row* is found structurally as
//! any node whose direct children are [`Display::TableCell`], which covers a
//! `<tr>` wherever it sits and a `<thead>` holding its header cells directly.
//! The intermediate wrappers (`<thead>`/`<tbody>`/`<tr>`) are recorded as
//! table-managed so the main layout loop leaves them alone rather than
//! re-stacking the grid as plain blocks.
use super::*;
use crate::style::Display;
use beet_core::prelude::*;
use bevy::math::IRect;
use bevy::math::UVec2;

/// One table row: the entity that owns the row plus its cell entities in column
/// order. Entities (not borrowed node views) are stored so a row outlives the
/// per-node query borrow used to collect it.
struct TableRow {
	node: Entity,
	cells: Vec<Entity>,
}

/// Collect a table's rows in document order, recording every structural wrapper
/// (and the row nodes themselves) into `managed`.
fn collect_rows(
	entity: Entity,
	query: &CharcellQuery,
	rows: &mut Vec<TableRow>,
	managed: &mut HashSet<Entity>,
) {
	let Ok(node) = query.unresolved_node(entity) else {
		return;
	};
	let is_cell = |child: &CharcellNodeData| {
		child.layout_style().display == Display::TableCell
	};
	let cells: Vec<Entity> = node
		.child_nodes(query)
		.filter(is_cell)
		.map(|cell| cell.entity)
		.collect();
	if cells.is_empty() {
		// a wrapper (table/thead/tbody/tfoot): managed, recurse for its rows.
		let children: Vec<Entity> =
			node.child_nodes(query).map(|child| child.entity).collect();
		for child in children {
			managed.insert(child);
			collect_rows(child, query, rows, managed);
		}
	} else {
		// a row: its cells are laid out by the grid; the row node is managed so the
		// main loop doesn't re-flow it.
		managed.insert(entity);
		rows.push(TableRow {
			node: entity,
			cells,
		});
	}
}

/// Each column's width in cells, the CSS auto table layout: the widest cell's
/// max-content width when the columns fit the table's `available` content
/// width, else each column floored at its longest word with the remaining space
/// shared in proportion to how much wider each wants to be. Only a table too
/// narrow for even its longest words scales the floors down, breaking words.
fn column_widths(
	rows: &[TableRow],
	query: &CharcellQuery,
	available: u32,
	viewport: UVec2,
) -> Vec<u32> {
	let cols = rows.iter().map(|row| row.cells.len()).max().unwrap_or(0);
	// each column's (min-content, max-content) width, at least one cell
	let mut min = vec![1u32; cols];
	let mut max = vec![1u32; cols];
	for row in rows {
		for (col, &cell) in row.cells.iter().enumerate() {
			let Ok(node) = query.unresolved_node(cell) else {
				continue;
			};
			let cell_max = node.intrinsic_size().x.max(1);
			max[col] = max[col].max(cell_max);
			min[col] = min[col]
				.max(min_content_width(&node, query, viewport).min(cell_max));
		}
	}
	let (min_total, max_total) =
		(min.iter().sum::<u32>(), max.iter().sum::<u32>());
	if available == 0 || max_total <= available {
		max
	} else if min_total >= available {
		min.iter()
			.map(|&width| {
				((width as u64 * available as u64) / min_total as u64).max(1)
					as u32
			})
			.collect()
	} else {
		let (spare, wanted) = (
			(available - min_total) as u64,
			(max_total - min_total) as u64,
		);
		min.iter()
			.zip(&max)
			.map(|(&lo, &hi)| lo + ((hi - lo) as u64 * spare / wanted) as u32)
			.collect()
	}
}

/// The narrowest `node` lays out without breaking a word, in cells: its longest
/// word at its glyph scale, or its widest child's, plus its own padding and
/// border (the CSS min-content width).
fn min_content_width(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	viewport: UVec2,
) -> u32 {
	let text = node
		.value()
		.map(|value| value.to_string())
		.or_else(|| node.marker().map(str::to_string));
	let content = match text {
		Some(text) => text
			.split(collapsible)
			.map(|word| measure_scaled(node.visual_style(), word, u32::MAX).x)
			.max()
			.unwrap_or(0),
		None => node
			.child_nodes(query)
			.map(|child| min_content_width(&child, query, viewport))
			.max()
			.unwrap_or(0),
	};
	content + BoxModel::from_node(node, viewport).overhead().x
}

/// A row's height: the tallest of its cells, each resolved at its column width.
fn row_height(
	row: &TableRow,
	query: &CharcellQuery,
	widths: &[u32],
	viewport: UVec2,
	port_rows: u32,
) -> u32 {
	row.cells
		.iter()
		.enumerate()
		.map(|(col, &cell)| {
			let width = widths.get(col).copied().unwrap_or(0);
			match query.unresolved_node(cell) {
				Ok(node) => {
					resolve_height(&node, query, width, viewport, port_rows)
						.max(1)
				}
				Err(_) => 1,
			}
		})
		.max()
		.unwrap_or(1)
}

/// The intrinsic content size of a table: the summed column widths and row
/// heights, clamped to the available width.
pub(super) fn measure_table(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	available: UVec2,
	viewport: UVec2,
) -> UVec2 {
	let mut rows = Vec::new();
	collect_rows(node.entity, query, &mut rows, &mut HashSet::default());
	let widths = column_widths(&rows, query, available.x, viewport);
	// the measure pass bounds a raster by the viewport, not a scroll port it
	// cannot know yet (see `measure_node`).
	let height = rows
		.iter()
		.map(|row| row_height(row, query, &widths, viewport, available.y))
		.sum();
	UVec2::new(widths.iter().sum::<u32>().min(available.x), height)
}

/// A table's content height at a constrained width (block-flow resolution path).
pub(super) fn resolve_table_height(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	content_width: u32,
	viewport: UVec2,
	port_rows: u32,
) -> u32 {
	let mut rows = Vec::new();
	collect_rows(node.entity, query, &mut rows, &mut HashSet::default());
	let widths = column_widths(&rows, query, content_width, viewport);
	rows.iter()
		.map(|row| row_height(row, query, &widths, viewport, port_rows))
		.sum()
}

/// Assign rects across a table's grid: stack rows top-to-bottom and place each
/// row's cells at their column offsets, every cell as tall as its row. Records
/// the managed wrapper/row entities so the caller skips re-laying them out.
/// `port_rows` is the scroll port height bounding the cells laid out here (see
/// [`resolve_height`]).
pub(super) fn table_layout_rects(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	container_rect: IRect,
	viewport: UVec2,
	layout_rects: &mut HashMap<Entity, IRect>,
	managed: &mut HashSet<Entity>,
	port_rows: u32,
) -> Result {
	let box_model = BoxModel::from_node(node, viewport);
	let content =
		scrollport_of(node, query, box_model.content_rect(container_rect));
	let mut rows = Vec::new();
	collect_rows(node.entity, query, &mut rows, managed);
	let widths =
		column_widths(&rows, query, content.width().max(0) as u32, viewport);

	let mut row_y = content.min.y;
	for row in &rows {
		if row_y >= content.max.y {
			break;
		}
		let height = row_height(row, query, &widths, viewport, port_rows);
		let row_bottom = (row_y + height as i32).min(content.max.y);
		let mut col_x = content.min.x;
		for (col, &cell) in row.cells.iter().enumerate() {
			let width = widths.get(col).copied().unwrap_or(0);
			let cell_rect = IRect::new(
				col_x,
				row_y,
				(col_x + width as i32).min(content.max.x),
				row_bottom,
			);
			layout_rects.insert(cell, cell_rect);
			col_x += width as i32;
		}
		// the row node spans its cells, so a row-level background or border paints
		// behind the whole row.
		layout_rects.insert(
			row.node,
			IRect::new(content.min.x, row_y, content.max.x, row_bottom),
		);
		row_y += height as i32;
	}
	Ok(())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::style::Display;
	use crate::style::*;
	use beet_core::prelude::*;
	use bevy::math::UVec2;

	fn cell(text: &'static str) -> impl Bundle {
		(
			LayoutStyle {
				display: Display::TableCell,
				..default()
			},
			rsx! { {text} },
		)
	}

	/// Render `table` through the real pipeline and Material rule set.
	fn render(width: u32, table: Snippet) -> String {
		crate::widgets::test_ext::render_charcell(width, (), table)
	}

	/// Cells lay out as a grid, each row on its own line with its columns side by
	/// side — guarding the original bug where a `<table>` rendered as a vertical
	/// list (every cell stacked on its own row). The wider first-column cell also
	/// pushes the second column right, so columns line up across rows.
	#[beet_core::test]
	fn rows_lay_cells_side_by_side() {
		let out = render(20, rsx! {
			<table>
				<tr><td>"Name"</td><td>"Age"</td></tr>
				<tr><td>"Alice"</td><td>"30"</td></tr>
			</table>
		});
		let header = out.lines().find(|line| line.contains("Name")).unwrap();
		let body = out.lines().find(|line| line.contains("Alice")).unwrap();
		// each row keeps its two cells on one line, the rows on separate lines
		header.xpect_contains("Age").xnot().xpect_contains("Alice");
		body.xpect_contains("30");
		// the second column starts past the widest first-column cell, the same
		// offset in every row
		body.find("30").xpect_eq(header.find("Age"));
		(body.find("30").unwrap() > body.find("Alice").unwrap() + 5)
			.xpect_true();
	}

	/// A table narrower than its content keeps every word whole while the
	/// space allows: a short-worded column holds its longest word and the prose
	/// column wraps, rather than every column scaling down and splitting
	/// `beet_core` mid-word.
	#[beet_core::test]
	fn narrow_table_keeps_words_whole() {
		render(40, rsx! {
			<Table>
				<tr>
					<td>"beet_core"</td>
					<td>"Cross-platform types, extension traits and the test runner"</td>
				</tr>
			</Table>
		})
		.xpect_contains("beet_core")
		.xpect_contains("Cross-platform");
	}

	/// The `<table>` rule carries `width: 100%`. With explicit-width resolution in
	/// the layout pass, that percent must still make the table span its container
	/// (a row-spanning background fills the full content width), guarding against a
	/// regression where percent resolution shrank the table off the old block-flow
	/// full-bleed path.
	#[beet_core::test]
	fn full_width_table_fills_container() {
		let bg = Color::srgb(0.2, 0.4, 0.8);
		let table = (LayoutStyle::default(), children![(
			LayoutStyle {
				display: Display::Table,
				..default()
			},
			BoxStyle {
				width: Some(Length::Percent(100.)),
				..default()
			},
			// a row carries the background so its rect (spanning the table's
			// content width) is what we measure
			children![(
				LayoutStyle::default(),
				VisualStyle {
					background: Some(bg),
					..default()
				},
				children![cell("A"), cell("B")],
			)],
		)]);
		let buffer = Buffer::new(UVec2::new(20, 4)).populate(table);
		// the row background spans all 20 columns of the container's content width
		buffer
			.iter_cells()
			.filter(|(pos, cell)| {
				pos.y == 0 && cell.style.background == Some(bg)
			})
			.count()
			.xpect_eq(20);
	}
}
