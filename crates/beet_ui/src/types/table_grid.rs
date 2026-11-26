//! The grid of any table, as HTML's table model forms it: a cell's place is
//! its slot in that grid, never an attribute, so a markdown table, an HTML
//! page, a Word file's table and a workbook's sheet are all addressed the
//! same way.
//!
//! A slot is a [`UVec2`] counted from 0 as the charcell grid counts, `x` its
//! column and `y` its row, so `UVec2::new(1, 2)` is the second column of the
//! third row. A format counting otherwise converts at its own edge, ie the
//! A1 references of a workbook, where that slot is `B3`.
use crate::prelude::*;
use beet_core::prelude::*;

/// The grids of a world's tables, the one way a table's rows and cells are
/// found by slot, read and locked, whatever the table was read from.
///
/// A table's rows are those of its `thead`s, `tbody`s and its own `tr`s in
/// tree order, then its `tfoot`s' last, as HTML's forming a table orders
/// them; a `caption`, a `colgroup` and a nested table are no part of it, and
/// an entity carrying no element, ie a fragment, is walked through.
#[derive(SystemParam)]
pub struct TableGrid<'w, 's> {
	elements: Query<'w, 's, &'static Element>,
	views: ElementQuery<'w, 's>,
	tree: RenderTreeQuery<'w, 's>,
}

/// A cell as its table's grid places it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridCell {
	/// The `td` or `th`.
	pub entity: Entity,
	/// The slot it starts at, its top left.
	pub start: UVec2,
	/// The columns and rows it spans, each at least 1: its `colspan`, and its
	/// `rowspan` cut to the table's last row.
	pub span: UVec2,
}

impl GridCell {
	/// The slot it ends at, its bottom right.
	pub fn last(&self) -> UVec2 { self.start + self.span - UVec2::ONE }

	/// Whether it holds `slot`, by starting there or spanning over it.
	pub fn contains(&self, slot: UVec2) -> bool {
		slot.cmpge(self.start).all() && slot.cmplt(self.start + self.span).all()
	}
}

impl TableGrid<'_, '_> {
	/// Whether `tag` is a table cell's, `td` or `th`.
	pub fn is_cell_tag(tag: &str) -> bool { matches!(tag, "td" | "th") }

	/// Whether `tag` is a table's own structure: the table, a row or a cell.
	pub fn is_table_tag(tag: &str) -> bool {
		matches!(tag, "table" | "tr") || Self::is_cell_tag(tag)
	}

	/// The rows of `table` in the order its grid counts them.
	pub fn rows(&self, table: Entity) -> Vec<Entity> {
		let (mut rows, mut footers) = (Vec::new(), Vec::new());
		for child in self.element_children(table) {
			match self.tag(child) {
				Some("tr") => rows.push(child),
				Some("thead" | "tbody") => rows.extend(self.row_group(child)),
				Some("tfoot") => footers.extend(self.row_group(child)),
				_ => {}
			}
		}
		rows.extend(footers);
		rows
	}

	/// Every cell of `table`, row major.
	pub fn cells(&self, table: Entity) -> Vec<GridCell> {
		let rows = self.rows(table);
		let row_count = rows.len() as u32;
		// the slots a `rowspan` from a row above holds
		let mut held = HashSet::<UVec2>::default();
		let mut cells = Vec::new();
		for (row, tr) in rows.into_iter().enumerate() {
			let mut start = UVec2::new(0, row as u32);
			for cell in self
				.element_children(tr)
				.filter(|cell| self.tag(*cell).is_some_and(Self::is_cell_tag))
			{
				while held.contains(&start) {
					start.x += 1;
				}
				let (columns, rows) = self
					.views
					.get(cell)
					.map(|view| (view.colspan(), view.rowspan()))
					.unwrap_or((1, 1));
				// a span past the table's last row holds nothing
				let span = UVec2::new(columns, rows.min(row_count - start.y));
				let placed = GridCell {
					entity: cell,
					start,
					span,
				};
				for below in 1..span.y {
					for offset in 0..span.x {
						held.insert(start + UVec2::new(offset, below));
					}
				}
				cells.push(placed);
				start.x += span.x;
			}
		}
		cells
	}

	/// The cell of `table` holding `slot`: the one starting there, else the
	/// one whose `colspan` or `rowspan` covers it, and `None` for a slot no
	/// cell holds, ie past the end of a ragged row.
	pub fn cell(&self, table: Entity, slot: UVec2) -> Option<GridCell> {
		self.cells(table)
			.into_iter()
			.find(|cell| cell.contains(slot))
	}

	/// The table `cell` is in and its place in that table's grid, `None` for
	/// an entity that is no cell of a table.
	pub fn locate(&self, cell: Entity) -> Option<(Entity, GridCell)> {
		let table = self
			.tree
			.iter_ancestors_inclusive(cell)
			.skip(1)
			.find(|ancestor| self.tag(*ancestor) == Some("table"))?;
		self.cells(table)
			.into_iter()
			.find(|found| found.entity == cell)
			.map(|found| (table, found))
	}

	/// The columns and rows of `table`'s grid: the furthest column any cell
	/// reaches, spans included, and its row count.
	pub fn size(&self, table: Entity) -> UVec2 {
		let columns = self
			.cells(table)
			.iter()
			.map(|cell| cell.start.x + cell.span.x)
			.max()
			.unwrap_or(0);
		UVec2::new(columns, self.rows(table).len() as u32)
	}

	/// Whether a person may not type into `cell`: the first of it and its
	/// ancestors whose `contenteditable` says either decides, locked when it
	/// says `false`, so a `<tr contenteditable="false">` locks its cells and
	/// a `<td contenteditable="true">` inside it unlocks itself.
	pub fn is_locked(&self, cell: Entity) -> bool {
		self.tree
			.iter_ancestors_inclusive(cell)
			.find_map(|entity| {
				self.views
					.get(entity)
					.ok()
					.and_then(|view| view.content_editable())
			})
			.is_some_and(|editable| !editable)
	}

	/// The `tr`s of a row group.
	fn row_group(&self, group: Entity) -> Vec<Entity> {
		self.element_children(group)
			.filter(|child| self.tag(*child) == Some("tr"))
			.collect()
	}

	/// The element children of `parent` in order, walking through every
	/// child carrying no element, ie a fragment.
	fn element_children(
		&self,
		parent: Entity,
	) -> impl Iterator<Item = Entity> + '_ {
		let mut out = Vec::new();
		let mut pending = self.tree.children(parent).rev().collect::<Vec<_>>();
		while let Some(child) = pending.pop() {
			match self.elements.contains(child) {
				true => out.push(child),
				false => pending.extend(self.tree.children(child).rev()),
			}
		}
		out.into_iter()
	}

	fn tag(&self, entity: Entity) -> Option<&str> {
		self.elements.get(entity).ok().map(Element::tag)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	#[cfg(feature = "scene")]
	use beet_scene::prelude::*;

	/// The world holding `scene`, and its first table.
	fn table(scene: impl Bundle) -> (World, Entity) {
		let mut world = World::new();
		world.spawn(scene);
		let table = world
			.query::<(Entity, &Element)>()
			.iter(&world)
			.find(|(_, element)| element.tag() == "table")
			.map(|(entity, _)| entity)
			.unwrap();
		(world, table)
	}

	/// Every cell of the table as its start's column and row then its text,
	/// row major.
	fn grid(world: &mut World, table: Entity) -> Vec<String> {
		world.with_state::<(TableGrid, DocumentText), _>(|(grid, text)| {
			grid.cells(table)
				.into_iter()
				.map(|cell| {
					format!(
						"{},{} {}",
						cell.start.x,
						cell.start.y,
						text.own_text(cell.entity)
					)
				})
				.collect()
		})
	}

	/// The text of the cell holding the slot at `column` and `row`.
	fn cell_text(
		world: &mut World,
		table: Entity,
		column: u32,
		row: u32,
	) -> String {
		world.with_state::<(TableGrid, DocumentText), _>(|(grid, text)| {
			grid.cell(table, UVec2::new(column, row))
				.map(|cell| text.own_text(cell.entity))
				.unwrap_or_default()
		})
	}

	#[beet_core::test]
	fn colspan_moves_the_cells_after_it() {
		let (mut world, table) = table(rsx! {
			<table>
				<tr><td colspan="2">"Name"</td><td>"Acme"</td></tr>
				<tr><td>"a"</td><td>"b"</td><td>"c"</td></tr>
			</table>
		});
		grid(&mut world, table).xpect_eq(vec![
			"0,0 Name".to_string(),
			"2,0 Acme".into(),
			"0,1 a".into(),
			"1,1 b".into(),
			"2,1 c".into(),
		]);
		cell_text(&mut world, table, 1, 0).xpect_eq("Name");
		world
			.with_state::<TableGrid, _>(|grid| grid.size(table))
			.xpect_eq(UVec2::new(3, 2));
	}

	/// A `rowspan` holds its column in the rows below, pushing their cells
	/// right; an absent, zero or unparseable span is 1, and a span past the
	/// last row holds nothing.
	#[beet_core::test]
	fn rowspan_pushes_a_later_row_right() {
		let (mut world, table) = table(rsx! {
			<table>
				<tr><td rowspan="2">"tall"</td><td colspan="0">"b"</td><td colspan="x">"c"</td></tr>
				<tr><td>"d"</td><td rowspan="9">"e"</td></tr>
				<tr><td>"f"</td></tr>
			</table>
		});
		grid(&mut world, table).xpect_eq(vec![
			"0,0 tall".to_string(),
			"1,0 b".into(),
			"2,0 c".into(),
			"1,1 d".into(),
			"2,1 e".into(),
			"0,2 f".into(),
		]);
		cell_text(&mut world, table, 0, 1).xpect_eq("tall");
		cell_text(&mut world, table, 2, 2).xpect_eq("e");
		world
			.with_state::<TableGrid, _>(|grid| {
				grid.cell(table, UVec2::new(2, 1)).map(|cell| cell.span)
			})
			.xpect_eq(Some(UVec2::new(1, 2)));
	}

	/// Rows count as HTML forms a table: `thead` and `tbody` in tree order,
	/// `tfoot` last wherever it is written, and a `caption` no row.
	#[beet_core::test]
	fn orders_thead_tbody_and_tfoot() {
		let (mut world, table) = table(rsx! {
			<table>
				<caption>"Sales"</caption>
				<tfoot><tr><td>"total"</td></tr></tfoot>
				<thead><tr><th>"head"</th></tr></thead>
				<tbody><tr><td>"body"</td></tr></tbody>
				<tr><td>"bare"</td></tr>
			</table>
		});
		grid(&mut world, table).xpect_eq(vec![
			"0,0 head".to_string(),
			"0,1 body".into(),
			"0,2 bare".into(),
			"0,3 total".into(),
		]);
	}

	/// A ragged row's missing slot holds no cell, and the grid is as wide as
	/// its widest row.
	#[beet_core::test]
	fn ragged_rows_hold_nothing() {
		let (mut world, table) = table(rsx! {
			<table>
				<tr><td>"a"</td><td>"b"</td><td>"c"</td></tr>
				<tr><td>"d"</td></tr>
			</table>
		});
		world
			.with_state::<TableGrid, _>(|grid| {
				(grid.cell(table, UVec2::new(2, 1)), grid.size(table))
			})
			.xpect_eq((None, UVec2::new(3, 2)));
	}

	/// A nested table is no part of its parent's grid, and a cell finds its
	/// own table and slot.
	#[beet_core::test]
	fn nested_tables_keep_their_own_grid() {
		let (mut world, table) = table(rsx! {
			<table>
				<tr>
					<td>"outer"</td>
					<td><table><tr><td>"x"</td><td>"inner"</td></tr></table></td>
				</tr>
			</table>
		});
		grid(&mut world, table)
			.xpect_eq(vec!["0,0 outer".to_string(), "1,0 ".into()]);
		let inner = world
			.query::<(Entity, &Value)>()
			.iter(&world)
			.find(|(_, value)| value.as_str().ok() == Some("inner"))
			.map(|(entity, _)| entity)
			.unwrap();
		let inner_cell = world.entity(inner).get::<ChildOf>().unwrap().parent();
		world
			.with_state::<TableGrid, _>(|grid| {
				grid.locate(inner_cell)
					.map(|(found, cell)| (found != table, cell.start))
			})
			.xpect_eq(Some((true, UVec2::new(1, 0))));
	}

	/// A table parsed from markdown and one parsed from HTML read the same
	/// grid.
	#[cfg(feature = "scene")]
	#[beet_core::test]
	fn parsed_tables_read_their_grid() {
		for (media_type, source) in [
			#[cfg(feature = "markdown_parser")]
			(MediaType::Markdown, "| a | b |\n|---|---|\n| c | d |\n"),
			(
				MediaType::Html,
				"<table><thead><tr><th>a</th><th>b</th></tr></thead><tbody><tr><td>c</td><td>d</td></tr></tbody></table>",
			),
		] {
			let mut world =
				(TemplatePlugin, DocumentPlugin, ParsePlugin).into_world();
			let root = world.spawn_empty().id();
			ParseContext::new(
				&mut world.entity_mut(root),
				&MediaBytes::new(media_type, source.as_bytes().to_vec()),
			)
			.parse()
			.unwrap();
			let table = world
				.query::<(Entity, &Element)>()
				.iter(&world)
				.find(|(_, element)| element.tag() == "table")
				.map(|(entity, _)| entity)
				.unwrap();
			grid(&mut world, table).xpect_eq(vec![
				"0,0 a".to_string(),
				"1,0 b".into(),
				"0,1 c".into(),
				"1,1 d".into(),
			]);
		}
	}
}
