//! Table layout for the charcell engine.
//!
//! A `<table>` ([`Display::Table`]) is laid out as a column-aligned grid: every
//! column takes the width of its widest cell (scaled down to fit the table), and
//! every row is as tall as its tallest cell. Rows and row groups carry no
//! display of their own — a *row* is found structurally as any node whose direct
//! children are [`Display::TableCell`], which covers both a `<tr>` and a markdown
//! `<thead>` that holds its header cells directly. The intermediate wrappers
//! (`<thead>`/`<tbody>`/`<tr>`) are recorded as table-managed so the main layout
//! loop leaves them alone rather than re-stacking the grid as plain blocks.
use super::*;
use crate::style::Display;
use beet_core::prelude::*;
use bevy::math::URect;
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
		rows.push(TableRow { node: entity, cells });
	}
}

/// The intrinsic width a cell entity wants, in cells (at least one).
fn cell_width(entity: Entity, query: &CharcellQuery) -> u32 {
	query
		.unresolved_node(entity)
		.map(|cell| cell.intrinsic_size().x.max(1))
		.unwrap_or(1)
}

/// Each column's width in cells: the widest cell in the column, scaled down
/// proportionally when the columns together overflow the table's `available`
/// content width.
fn column_widths(
	rows: &[TableRow],
	query: &CharcellQuery,
	available: u32,
) -> Vec<u32> {
	let cols = rows.iter().map(|row| row.cells.len()).max().unwrap_or(0);
	let mut widths = vec![0u32; cols];
	for row in rows {
		for (col, &cell) in row.cells.iter().enumerate() {
			widths[col] = widths[col].max(cell_width(cell, query));
		}
	}
	let total: u32 = widths.iter().sum();
	if total > available && available > 0 {
		for width in &mut widths {
			*width =
				((*width as u64 * available as u64) / total as u64).max(1) as u32;
		}
	}
	widths
}

/// A row's height: the tallest of its cells, each resolved at its column width.
fn row_height(
	row: &TableRow,
	query: &CharcellQuery,
	widths: &[u32],
	viewport: UVec2,
) -> u32 {
	row.cells
		.iter()
		.enumerate()
		.map(|(col, &cell)| {
			let width = widths.get(col).copied().unwrap_or(0);
			match query.unresolved_node(cell) {
				Ok(node) => resolve_height(&node, query, width, viewport).max(1),
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
	let widths = column_widths(&rows, query, available.x);
	let height = rows
		.iter()
		.map(|row| row_height(row, query, &widths, viewport))
		.sum();
	UVec2::new(widths.iter().sum::<u32>().min(available.x), height)
}

/// A table's content height at a constrained width (block-flow resolution path).
pub(super) fn resolve_table_height(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	content_width: u32,
	viewport: UVec2,
) -> u32 {
	let mut rows = Vec::new();
	collect_rows(node.entity, query, &mut rows, &mut HashSet::default());
	let widths = column_widths(&rows, query, content_width);
	rows.iter().map(|row| row_height(row, query, &widths, viewport)).sum()
}

/// Assign rects across a table's grid: stack rows top-to-bottom and place each
/// row's cells at their column offsets, every cell as tall as its row. Records
/// the managed wrapper/row entities so the caller skips re-laying them out.
pub(super) fn table_layout_rects(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	container_rect: URect,
	viewport: UVec2,
	layout_rects: &mut HashMap<Entity, URect>,
	managed: &mut HashSet<Entity>,
) -> Result {
	let box_model = BoxModel::from_node(node, viewport);
	let content = box_model.content_rect(container_rect);
	let mut rows = Vec::new();
	collect_rows(node.entity, query, &mut rows, managed);
	let widths = column_widths(&rows, query, content.width());

	let mut row_y = content.min.y;
	for row in &rows {
		if row_y >= content.max.y {
			break;
		}
		let height = row_height(row, query, &widths, viewport);
		let row_bottom = (row_y + height).min(content.max.y);
		let mut col_x = content.min.x;
		for (col, &cell) in row.cells.iter().enumerate() {
			let width = widths.get(col).copied().unwrap_or(0);
			let cell_rect = URect::new(
				col_x,
				row_y,
				(col_x + width).min(content.max.x),
				row_bottom,
			);
			layout_rects.insert(cell, cell_rect);
			col_x += width;
		}
		// the row node spans its cells, so a row-level background or border paints
		// behind the whole row.
		layout_rects.insert(
			row.node,
			URect::new(content.min.x, row_y, content.max.x, row_bottom),
		);
		row_y += height;
	}
	Ok(())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::style::*;
	use beet_core::prelude::*;
	use bevy::math::UVec2;

	fn cell(text: &'static str) -> impl Bundle {
		(
			LayoutStyle {
				display: Display::TableCell,
				..default()
			},
			rsx_direct! { {text} },
		)
	}

	/// Cells lay out as a grid, each row on its own line with its columns side by
	/// side — guarding the original bug where a `<table>` rendered as a vertical
	/// list (every cell stacked on its own row). The wider first-column cell also
	/// pushes the second column right, so columns line up across rows.
	#[beet_core::test]
	fn rows_lay_cells_side_by_side() {
		let table = (
			LayoutStyle {
				display: Display::Table,
				..default()
			},
			children![
				// a `<tr>` is a plain block whose cells mark it as a row
				(LayoutStyle::default(), children![
					cell("Name"),
					cell("Age")
				]),
				(LayoutStyle::default(), children![
					cell("Alice"),
					cell("30")
				]),
			],
		);
		let out = Buffer::render_oneshot_plain_sized(UVec2::new(20, 6), table)
			.trim_lines();
		// each row keeps its two cells on one line (not stacked into a list)
		let header = out.lines().find(|line| line.contains("Name")).unwrap();
		let body = out.lines().find(|line| line.contains("Alice")).unwrap();
		header.xpect_contains("Name").xpect_contains("Age");
		body.xpect_contains("Alice").xpect_contains("30");
		// the second column sits past the widest first-column cell ("Alice" = 5)
		(body.find("30").unwrap() >= 5).xpect_true();
	}

	/// The `<table>` rule carries `width: 100%`. With explicit-width resolution in
	/// the layout pass, that percent must still make the table span its container
	/// (a row-spanning background fills the full content width), guarding against a
	/// regression where percent resolution shrank the table off the old block-flow
	/// full-bleed path.
	#[beet_core::test]
	fn full_width_table_fills_container() {
		let bg = Color::srgb(0.2, 0.4, 0.8);
		let table = (
			LayoutStyle::default(),
			children![(
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
			)],
		);
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
