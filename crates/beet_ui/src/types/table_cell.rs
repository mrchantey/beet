//! The cells of a document's tables, addressed: every cell of every table as
//! a [`TableCellAddress`], so an edit or a cells listing names a cell the same
//! way in a markdown table, an HTML page and a Word form. A workbook's cells
//! carry the `ooxml` module's `SheetCellAddress` instead, and
//! [`CellText::listing`] lists both.
//!
//! Cells are numbered in one place: [`MediaParser`] leaves [`NumberTables`]
//! on every root it parses into, and the [`TableNumbering`] step of
//! [`PostParseTree`] numbers that document's tables once, whatever its
//! format.
use crate::prelude::*;
use beet_core::prelude::*;
use core::num::NonZeroU32;

/// On a parsed document's root until [`PostParseTree`] numbers its tables:
/// every table outside a `header`, `footer` or `aside`, in document order,
/// which is Word's main-part rule, each of its cells given a
/// [`TableCellAddress`]. A workbook's sheets are tables already addressed by
/// sheet, and are left alone. The numbering removes it, so a document numbers
/// its tables once and an edit adding a table renumbers nothing.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Component)]
pub struct NumberTables {
	/// Whether each numbered table is captioned with its number, ie `t3`: a
	/// projection-only `<caption>` a reader sees above the table and a writer
	/// never writes, for a Word file, whose tables carry no number a person
	/// can see.
	pub caption: bool,
}

/// The [`PostParseTree`] step numbering the tables of every root carrying
/// [`NumberTables`], which a step extending a parsed tree, ie a deck's triage,
/// runs after so its own tables stay unnumbered.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct TableNumbering;

impl NumberTables {
	/// Numbers the tables of every root carrying [`NumberTables`], captioning
	/// them where it asks, and removes it.
	pub(crate) fn apply_all(world: &mut World) {
		let roots = world
			.query::<(Entity, &NumberTables)>()
			.iter(world)
			.map(|(entity, number)| (entity, *number))
			.collect::<Vec<_>>();
		for (root, number) in roots {
			let numbered = world
				.with_state::<TableCells, _>(|cells| cells.numbering(root));
			for (entity, address) in &numbered.cells {
				world.entity_mut(*entity).insert(*address);
			}
			if number.caption {
				for (table, address) in numbered.tables {
					let caption = world
						.spawn((Element::new("caption"), children![
							Value::Str(format!("t{address}").into())
						]))
						.id();
					world.entity_mut(table).insert_child(0, caption);
				}
			}
			world.entity_mut(root).remove::<NumberTables>();
		}
	}
}

/// What [`TableCells::numbering`] found under one root.
struct Numbering {
	/// Each numbered table with its number, in document order.
	tables: Vec<(Entity, NonZeroU32)>,
	/// Each unaddressed cell with its address.
	cells: Vec<(Entity, TableCellAddress)>,
}

/// A table cell, `t<table>r<row>c<cell>`, each counted from 1: tables in
/// document order as they open, nested ones included, rows as the rows of
/// their table and cells as the cells of their row. `t3r2c1` is the first
/// cell of the second row of the third table. A document's parser numbers
/// its cells once, so an edit adding a table renumbers nothing.
#[derive(
	Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, Component,
)]
#[reflect(Component)]
pub struct TableCellAddress {
	/// The table, from 1.
	pub table: NonZeroU32,
	/// The row within it, from 1.
	pub row: NonZeroU32,
	/// The cell within the row, from 1.
	pub cell: NonZeroU32,
}

impl TableCellAddress {
	/// Parses `t<n>r<n>c<n>`, every number from 1.
	pub fn parse(text: &str) -> Result<Self> {
		Self::try_parse(text).ok_or_else(|| {
			bevyhow!("`{text}` is not a table cell, expected `t<n>r<n>c<n>`")
		})
	}

	fn try_parse(text: &str) -> Option<Self> {
		let rest = text.strip_prefix('t')?;
		let (table, rest) = rest.split_once('r')?;
		let (row, cell) = rest.split_once('c')?;
		Self {
			table: Self::parse_count(table)?,
			row: Self::parse_count(row)?,
			cell: Self::parse_count(cell)?,
		}
		.xsome()
	}

	/// A count from 1 in plain digits, no leading zero.
	pub(crate) fn parse_count(text: &str) -> Option<NonZeroU32> {
		(!text.starts_with('0')
			&& text.chars().all(|char| char.is_ascii_digit()))
		.then(|| text.parse().ok())
		.flatten()
		.and_then(NonZeroU32::new)
	}

	/// The count of the item at `index` from 0, ie a table's place among the
	/// tables before it.
	pub(crate) fn count(index: usize) -> NonZeroU32 {
		NonZeroU32::MIN.saturating_add(index as u32)
	}
}

impl core::fmt::Display for TableCellAddress {
	fn fmt(
		&self,
		formatter: &mut core::fmt::Formatter<'_>,
	) -> core::fmt::Result {
		write!(formatter, "t{}r{}c{}", self.table, self.row, self.cell)
	}
}

/// One addressed cell as a cells listing prints it: its address and the
/// text a reader reads in it.
#[derive(Debug, Default, Clone, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CellText {
	/// The address, `t1r2c3` or `Sheet!A1`.
	pub address: SmolStr,
	/// The cell's paragraphs, trimmed, the empty dropped and the rest joined
	/// by ` / `; a formula as `=` and its expression.
	pub text: SmolStr,
}

/// The listing's row, `| address | text |`, an empty cell `(empty)` and a
/// pipe escaped so the row stays one row.
impl core::fmt::Display for CellText {
	fn fmt(
		&self,
		formatter: &mut core::fmt::Formatter<'_>,
	) -> core::fmt::Result {
		let text = match self.text.is_empty() {
			true => "(empty)".to_string(),
			false => self.text.replace('|', "\\|"),
		};
		write!(formatter, "| {} | {text} |", self.address)
	}
}

impl CellText {
	/// Every addressed cell under `root`, the map an edit addresses a
	/// document by: its table cells in document order, then a workbook's
	/// unlocked cells.
	pub fn listing(world: &mut World, root: Entity) -> Vec<CellText> {
		let cells =
			world.with_state::<TableCells, _>(|cells| cells.listing(root));
		#[cfg(feature = "ooxml")]
		let cells = cells
			.into_iter()
			.chain(
				world.with_state::<SheetCells, _>(|cells| cells.listing(root)),
			)
			.collect();
		cells
	}

	/// The cells as a table headed `Cell` and `Text`, an empty cell's text
	/// `(empty)`: the map an edit addresses a form by.
	pub fn table(cells: &[CellText]) -> impl Bundle + use<> {
		let rows = cells
			.iter()
			.map(|cell| {
				let text = match cell.text.is_empty() {
					true => SmolStr::new("(empty)"),
					false => cell.text.clone(),
				};
				(cell.address.clone(), text)
			})
			.collect::<Vec<_>>();
		let row = |cells: [(&'static str, SmolStr); 2]| {
			let [(tag, first), (_, second)] = cells;
			(Element::new("tr"), children![
				(Element::new(tag), children![Value::Str(first)]),
				(Element::new(tag), children![Value::Str(second)]),
			])
		};
		(
			Element::new("table"),
			OnSpawn::new(move |entity| {
				entity.with_children(|table| {
					table.spawn(row([
						("th", "Cell".into()),
						("th", "Text".into()),
					]));
					for (address, text) in rows {
						table.spawn(row([("td", address), ("td", text)]));
					}
				});
			}),
		)
	}
}

/// The addressed table cells of a document, and the text a reader reads in
/// each.
#[derive(SystemParam)]
pub struct TableCells<'w, 's> {
	tables: Query<'w, 's, &'static TableCellAddress>,
	elements: Query<'w, 's, &'static Element>,
	children: Query<'w, 's, &'static Children>,
	text: ReaderText<'w, 's>,
	/// The cells a workbook addresses by sheet, whose tables are no tables
	/// to number.
	#[cfg(feature = "ooxml")]
	sheet_cells: Query<'w, 's, (), With<SheetCellAddress>>,
}

impl TableCells<'_, '_> {
	/// Every table cell under `root` in document order, with its address.
	pub fn table_cells(&self, root: Entity) -> Vec<(TableCellAddress, Entity)> {
		self.children
			.iter_descendants_depth_first(root)
			.filter_map(|entity| {
				self.tables
					.get(entity)
					.ok()
					.map(|address| (*address, entity))
			})
			.collect()
	}

	/// The table cell at `address` under `root`.
	pub fn table_cell(
		&self,
		root: Entity,
		address: TableCellAddress,
	) -> Option<Entity> {
		self.children
			.iter_descendants_depth_first(root)
			.find(|entity| self.tables.get(*entity) == Ok(&address))
	}

	/// The listing of every table cell under `root`, in document order.
	pub fn listing(&self, root: Entity) -> Vec<CellText> {
		self.table_cells(root)
			.into_iter()
			.map(|(address, cell)| CellText {
				address: address.to_string().into(),
				text: self.cell_text(cell).into(),
			})
			.collect()
	}

	/// What a reader reads in `cell`: its blocks' text, each trimmed, the
	/// empty dropped and the rest joined by ` / `, and its own text when it
	/// holds no block.
	pub fn cell_text(&self, cell: Entity) -> String {
		let blocks = self.text.blocks(cell);
		let texts = match blocks.is_empty() {
			true => vec![self.text.own_text(cell)],
			false => blocks
				.into_iter()
				.map(|block| self.text.text(block))
				.collect(),
		};
		texts
			.iter()
			.map(|text| text.split_whitespace().collect::<Vec<_>>().join(" "))
			.filter(|text| !text.is_empty())
			.collect::<Vec<_>>()
			.join(" / ")
	}

	/// The number of every table under `root` outside a `header`, `footer`
	/// or `aside`, in document order as the tables open, a sheet aside, and
	/// the address of every cell not yet addressed: rows and cells as their
	/// table's and row's own, a nested table's left to it.
	fn numbering(&self, root: Entity) -> Numbering {
		let mut tables = Vec::new();
		self.tables_under(root, &mut tables);
		let mut numbering = Numbering {
			tables: Vec::new(),
			cells: Vec::new(),
		};
		for (index, table) in tables.into_iter().enumerate() {
			let number = TableCellAddress::count(index);
			numbering.tables.push((table, number));
			for (row_index, row) in
				self.within(table, &["tr"]).into_iter().enumerate()
			{
				for (cell_index, cell) in
					self.within(row, &["td", "th"]).into_iter().enumerate()
				{
					if self.tables.contains(cell) {
						continue;
					}
					numbering.cells.push((cell, TableCellAddress {
						table: number,
						row: TableCellAddress::count(row_index),
						cell: TableCellAddress::count(cell_index),
					}));
				}
			}
		}
		numbering
	}

	/// Every table under `parent` in document order, each before the tables
	/// nested in it, none under a `header`, `footer` or `aside`, and no
	/// workbook sheet.
	fn tables_under(&self, parent: Entity, out: &mut Vec<Entity>) {
		for child in self.children.get(parent).into_iter().flatten() {
			match self.tag(*child) {
				Some("header" | "footer" | "aside") => continue,
				Some("table") if !self.is_sheet(*child) => out.push(*child),
				_ => {}
			}
			self.tables_under(*child, out);
		}
	}

	/// Whether `table` is a workbook's sheet, its cells addressed by sheet.
	fn is_sheet(&self, table: Entity) -> bool {
		#[cfg(feature = "ooxml")]
		return self.within(table, &["tr"]).into_iter().any(|row| {
			self.within(row, &["td", "th"])
				.into_iter()
				.any(|cell| self.sheet_cells.contains(cell))
		});
		#[cfg(not(feature = "ooxml"))]
		{
			let _ = table;
			false
		}
	}

	/// The elements tagged `tags` under `parent`, in order, through every
	/// entity between that is no table, row or cell, ie a `<tbody>` or a
	/// node a reader never sees.
	fn within(&self, parent: Entity, tags: &[&str]) -> Vec<Entity> {
		let mut out = Vec::new();
		for child in self.children.get(parent).into_iter().flatten() {
			match self.tag(*child) {
				Some(tag) if tags.contains(&tag) => out.push(*child),
				Some("table" | "tr" | "td" | "th") => {}
				_ => out.extend(self.within(*child, tags)),
			}
		}
		out
	}

	fn tag(&self, entity: Entity) -> Option<&str> {
		self.elements.get(entity).ok().map(Element::tag)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn parses_table_cells() {
		let cell = TableCellAddress::parse("t3r2c1").unwrap();
		(cell.table.get(), cell.row.get(), cell.cell.get()).xpect_eq((3, 2, 1));
		cell.to_string().xpect_eq("t3r2c1");
		for bad in ["t0r1c1", "t1r1", "t01r1c1", "x1r1c1"] {
			TableCellAddress::parse(bad).xpect_err();
		}
	}

	/// Tables number as they open, a nested one after its host, a table
	/// under a `header`, `footer` or `aside` is no table to number, and a
	/// cell reads as its paragraphs joined.
	#[beet_core::test]
	fn numbers_and_reads_cells() {
		let mut world = ParsePlugin.into_world();
		let root = world
			.spawn((NumberTables::default(), rsx! {
				<div>
					<header><table><tr><td>"page"</td></tr></table></header>
					<table><tbody><tr>
						<td><p>"Name"</p><p>" "</p><p>"Here"</p></td>
						<td>
							<table><tr><td>"inner"</td></tr></table>
						</td>
					</tr></tbody></table>
					<aside><table><tr><td>"note"</td></tr></table></aside>
					<table><tr><th>"a | b"</th><td/></tr></table>
				</div>
			}))
			.id();
		world.run_schedule(PostParseTree);
		world.entity(root).contains::<NumberTables>().xpect_false();
		CellText::listing(&mut world, root)
			.iter()
			.map(ToString::to_string)
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				"| t1r1c1 | Name / Here |".to_string(),
				"| t1r1c2 | (empty) |".into(),
				"| t2r1c1 | inner |".into(),
				"| t3r1c1 | a \\| b |".into(),
				"| t3r1c2 | (empty) |".into(),
			]);
	}
}
