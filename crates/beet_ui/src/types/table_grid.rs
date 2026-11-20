//! The grid of any table, as HTML's table model forms it: a cell's place is
//! its position in that grid, never an attribute, so a markdown table, an
//! HTML page, a Word file's table and a workbook's sheet are all addressed
//! the same way, `B3` being the cell in the second column of the third row.
use crate::prelude::*;
use beet_core::prelude::*;
use core::num::NonZeroU32;

/// A slot of a table's grid, its column and row each counted from 1, written
/// in the A1 form a spreadsheet writes, ie `B3`, `AA10` or `XFD1048576`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TablePosition {
	/// The column, from 1, as HTML's table model places it: a cell spanning
	/// two columns before it moves it right by two, and a `rowspan` from a
	/// row above moves it past the slot that holds.
	pub column: NonZeroU32,
	/// The row, from 1.
	pub row: NonZeroU32,
}

/// The grids of a world's tables, the one way a table's rows and cells are
/// found by position, read and locked, whatever the table was read from.
///
/// A table's rows are those of its `thead`s, `tbody`s and its own `tr`s in
/// tree order, then its `tfoot`s' last, as HTML's forming a table orders
/// them; a `caption`, a `colgroup` and a nested table are no part of it, and
/// an entity carrying no element, ie a fragment, is walked through.
#[derive(SystemParam)]
pub struct TableGrid<'w, 's> {
	elements: Query<'w, 's, &'static Element>,
	views: ElementQuery<'w, 's>,
	children: Query<'w, 's, &'static Children>,
	parents: Query<'w, 's, &'static ChildOf>,
}

/// A table's cells as its grid places them.
struct GridLayout {
	/// The number of rows.
	rows: u32,
	/// Each cell, row major, with the slot it starts at and the columns and
	/// rows it spans, a `rowspan` past the last row cut to it.
	cells: Vec<(Entity, TablePosition, u32, u32)>,
}

impl TablePosition {
	/// The position at `column` and `row`, each counted from 1, `None` for a
	/// zero.
	pub fn new(column: u32, row: u32) -> Option<Self> {
		Self {
			column: NonZeroU32::new(column)?,
			row: NonZeroU32::new(row)?,
		}
		.xsome()
	}

	/// Parses the A1 form, one to three uppercase letters then a row with no
	/// leading zero.
	pub fn parse(a1: &str) -> Result<Self> {
		Self::parse_a1(a1)
			.map(|(column, row)| Self { column, row })
			.ok_or_else(|| {
				bevyhow!(
					"`{a1}` is not a cell position, expected its column's letters then its row, ie `B3`"
				)
			})
	}

	/// The letters naming column `column`, `1` being `A` and `27` `AA`.
	pub fn column_letters(column: NonZeroU32) -> String {
		let mut letters = Vec::new();
		let mut rest = column.get();
		while rest > 0 {
			letters.push(b'A' + ((rest - 1) % 26) as u8);
			rest = (rest - 1) / 26;
		}
		letters.reverse();
		String::from_utf8(letters).unwrap_or_default()
	}

	/// The column and row of an A1 reference, one to three uppercase letters
	/// then a row with no leading zero.
	pub fn parse_a1(a1: &str) -> Option<(NonZeroU32, NonZeroU32)> {
		let letters = a1
			.chars()
			.take_while(|char| char.is_ascii_uppercase())
			.count();
		let (column, row) = a1.split_at(letters);
		if column.is_empty() || column.len() > 3 {
			return None;
		}
		let column = column
			.bytes()
			.fold(0, |acc, byte| acc * 26 + (byte - b'A' + 1) as u32)
			.xmap(NonZeroU32::new)?;
		let row = (!row.starts_with('0')
			&& !row.is_empty()
			&& row.chars().all(|char| char.is_ascii_digit()))
		.then(|| row.parse().ok())
		.flatten()
		.and_then(NonZeroU32::new)?;
		Some((column, row))
	}
}

/// The A1 form, ie `B3`.
impl core::fmt::Display for TablePosition {
	fn fmt(
		&self,
		formatter: &mut core::fmt::Formatter<'_>,
	) -> core::fmt::Result {
		write!(
			formatter,
			"{}{}",
			Self::column_letters(self.column),
			self.row
		)
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

	/// Every cell of `table`, row major, with the position it starts at.
	pub fn cells(&self, table: Entity) -> Vec<(Entity, TablePosition)> {
		self.layout(table)
			.cells
			.into_iter()
			.map(|(cell, position, ..)| (cell, position))
			.collect()
	}

	/// The cell of `table` holding `position`: the one starting there, else
	/// the one whose `colspan` or `rowspan` covers it, and `None` for a slot
	/// no cell holds, ie past the end of a ragged row.
	pub fn cell(
		&self,
		table: Entity,
		position: TablePosition,
	) -> Option<Entity> {
		self.layout(table)
			.cells
			.into_iter()
			.find(|(_, start, columns, rows)| {
				let within = |start: NonZeroU32, span: u32, at: NonZeroU32| {
					(start.get()..start.get().saturating_add(span))
						.contains(&at.get())
				};
				within(start.column, *columns, position.column)
					&& within(start.row, *rows, position.row)
			})
			.map(|(cell, ..)| cell)
	}

	/// The table `cell` is in and the position it starts at, `None` for an
	/// entity that is no cell of a table.
	pub fn position(&self, cell: Entity) -> Option<(Entity, TablePosition)> {
		let table = self
			.parents
			.iter_ancestors(cell)
			.find(|ancestor| self.tag(*ancestor) == Some("table"))?;
		self.cells(table)
			.into_iter()
			.find(|(found, _)| *found == cell)
			.map(|(_, position)| (table, position))
	}

	/// The columns and rows of `table`'s grid: its row count, and the
	/// furthest column any cell reaches, spans included.
	pub fn size(&self, table: Entity) -> (u32, u32) {
		let layout = self.layout(table);
		let columns = layout
			.cells
			.iter()
			.map(|(_, start, columns, _)| start.column.get() + columns - 1)
			.max()
			.unwrap_or(0);
		(columns, layout.rows)
	}

	/// Whether a person may not type into `cell`: the first of it and its
	/// ancestors whose `contenteditable` says either decides, locked when it
	/// says `false`, so a `<tr contenteditable="false">` locks its cells and
	/// a `<td contenteditable="true">` inside it unlocks itself.
	pub fn is_locked(&self, cell: Entity) -> bool {
		core::iter::once(cell)
			.chain(self.parents.iter_ancestors(cell))
			.find_map(|entity| {
				self.views
					.get(entity)
					.ok()
					.and_then(|view| view.content_editable())
			})
			.is_some_and(|editable| !editable)
	}

	/// Places every cell of `table`: each row's cells left to right, each
	/// skipping the slots a `rowspan` from a row above still holds.
	fn layout(&self, table: Entity) -> GridLayout {
		let rows = self.rows(table);
		let row_count = rows.len() as u32;
		// the slots a `rowspan` from a row above holds, by row
		let mut held = HashSet::<(u32, u32)>::default();
		let mut cells = Vec::new();
		for (row_index, row) in rows.into_iter().enumerate() {
			let row_index = row_index as u32;
			let mut column = 1;
			for cell in self
				.element_children(row)
				.filter(|cell| self.tag(*cell).is_some_and(Self::is_cell_tag))
			{
				while held.contains(&(row_index, column)) {
					column += 1;
				}
				let (columns, rows) = self
					.views
					.get(cell)
					.map(|view| (view.colspan(), view.rowspan()))
					.unwrap_or((1, 1));
				// a span past the table's last row holds nothing
				let rows = rows.min(row_count - row_index);
				for below in 1..rows {
					for offset in 0..columns {
						held.insert((row_index + below, column + offset));
					}
				}
				let position = TablePosition::new(column, row_index + 1)
					.unwrap_or(TablePosition {
						column: NonZeroU32::MIN,
						row: NonZeroU32::MIN,
					});
				cells.push((cell, position, columns, rows));
				column += columns;
			}
		}
		GridLayout {
			rows: row_count,
			cells,
		}
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
		let mut pending = self
			.children
			.get(parent)
			.map(|children| children.iter().rev().collect::<Vec<_>>())
			.unwrap_or_default();
		while let Some(child) = pending.pop() {
			match self.elements.contains(child) {
				true => out.push(child),
				false => pending.extend(
					self.children
						.get(child)
						.into_iter()
						.flat_map(|children| children.iter().rev()),
				),
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

	/// Every cell of the table with its position and text, row major.
	fn grid(world: &mut World, table: Entity) -> Vec<String> {
		world.with_state::<(TableGrid, DocumentText), _>(|(grid, text)| {
			grid.cells(table)
				.into_iter()
				.map(|(cell, position)| {
					format!("{position} {}", text.own_text(cell))
				})
				.collect()
		})
	}

	/// The text of the cell holding `position`.
	fn cell_text(world: &mut World, table: Entity, position: &str) -> String {
		world.with_state::<(TableGrid, DocumentText), _>(|(grid, text)| {
			grid.cell(table, TablePosition::parse(position).unwrap())
				.map(|cell| text.own_text(cell))
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
			"A1 Name".to_string(),
			"C1 Acme".into(),
			"A2 a".into(),
			"B2 b".into(),
			"C2 c".into(),
		]);
		cell_text(&mut world, table, "B1").xpect_eq("Name");
		world
			.with_state::<TableGrid, _>(|grid| grid.size(table))
			.xpect_eq((3, 2));
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
			"A1 tall".to_string(),
			"B1 b".into(),
			"C1 c".into(),
			"B2 d".into(),
			"C2 e".into(),
			"A3 f".into(),
		]);
		cell_text(&mut world, table, "A2").xpect_eq("tall");
		cell_text(&mut world, table, "C3").xpect_eq("e");
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
			"A1 head".to_string(),
			"A2 body".into(),
			"A3 bare".into(),
			"A4 total".into(),
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
				(
					grid.cell(table, TablePosition::parse("C2").unwrap()),
					grid.size(table),
				)
			})
			.xpect_eq((None, (3, 2)));
	}

	/// A nested table is no part of its parent's grid, and a cell finds its
	/// own table and position.
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
			.xpect_eq(vec!["A1 outer".to_string(), "B1 ".into()]);
		let inner = world
			.query::<(Entity, &Value)>()
			.iter(&world)
			.find(|(_, value)| value.as_str().ok() == Some("inner"))
			.map(|(entity, _)| entity)
			.unwrap();
		let inner_cell = world.entity(inner).get::<ChildOf>().unwrap().parent();
		world
			.with_state::<TableGrid, _>(|grid| {
				grid.position(inner_cell).map(|(found, position)| {
					(found != table, position.to_string())
				})
			})
			.xpect_eq(Some((true, "B1".to_string())));
	}

	/// A table parsed from markdown and one parsed from HTML read the same
	/// grid.
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
			DocParsers::parse(ParseContext::new(
				&mut world.entity_mut(root),
				&MediaBytes::new(media_type, source.as_bytes().to_vec()),
			))
			.unwrap();
			let table = world
				.query::<(Entity, &Element)>()
				.iter(&world)
				.find(|(_, element)| element.tag() == "table")
				.map(|(entity, _)| entity)
				.unwrap();
			grid(&mut world, table).xpect_eq(vec![
				"A1 a".to_string(),
				"B1 b".into(),
				"A2 c".into(),
				"B2 d".into(),
			]);
		}
	}

	#[beet_core::test]
	fn a1_both_ways() {
		for (a1, column, row) in [
			("B3", 2, 3),
			("AA10", 27, 10),
			("XFD1048576", 16384, 1048576),
		] {
			let position = TablePosition::parse(a1).unwrap();
			(position.column.get(), position.row.get()).xpect_eq((column, row));
			position.to_string().xpect_eq(a1);
		}
		for bad in ["", "3", "b3", "A0", "A01", "ABCD1", "A", "A1B"] {
			TablePosition::parse(bad)
				.unwrap_err()
				.to_string()
				.xpect_contains("is not a cell position");
		}
	}
}
