//! The cells of a document's tables, addressed: every cell of every table a
//! [`CellAddress`], so an edit or a cells listing names a cell the same way
//! in a markdown table, an HTML page, a Word form and a workbook.
//!
//! A format that names its grids supplies the addresses itself, ie a
//! workbook's `Budget!D3`; every other table is numbered in one place:
//! [`DocParsers`] leaves [`NumberTables`] on every root it parses into, and
//! the [`TableNumbering`] step of [`PostParseTree`] numbers that document's
//! unaddressed tables once, `t3r2c1`, whatever its format. Either way a
//! column is the visual column HTML's table model places a cell in, so a
//! spanning cell answers for every slot it covers through [`Cells`].
use crate::prelude::*;
use beet_core::prelude::*;
use core::num::NonZeroU32;

/// On a parsed document's root until [`PostParseTree`] numbers its tables:
/// every table outside a `header`, `footer` or `aside` none of whose cells
/// carries a [`CellAddress`], in document order, which is Word's main-part
/// rule, each of its cells given one. A table arriving addressed, ie a
/// workbook's sheet, is left alone. The numbering removes it, so a document
/// numbers its tables once and an edit adding a table renumbers nothing.
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

/// A table cell: its grid, its visual column and its row, each counted from
/// 1. Spelled `t<table>r<row>c<column>` for a numbered table, ie `t3r2c1`,
/// and `<sheet>!<A1>` for a named one, ie `Budget!D3`. The column is the one
/// HTML's table model places the cell in, never its place among the cells
/// present: a cell spanning two columns before it moves it right by two, and
/// a `rowspan` from a row above moves it past the slot it holds.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect, Component)]
#[reflect(Component)]
pub struct CellAddress {
	/// The table: its number in the document, or a sheet's tab name.
	pub grid: CellGrid,
	/// The visual column, from 1.
	pub column: NonZeroU32,
	/// The row, from 1.
	pub row: NonZeroU32,
}

/// The table a [`CellAddress`] is in.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect)]
pub enum CellGrid {
	/// A table by its number in the document, as [`NumberTables`] counts
	/// them, a captioned Word or HTML table included.
	Ordinal(NonZeroU32),
	/// A grid a format names itself, ie a workbook's sheet by its tab name.
	Named(SmolStr),
}

/// The formula a cell computes, as a spreadsheet writes it, without its `=`:
/// its shown text is the value last computed. An edit replaces it as typing
/// into the cell would, with a value, or with another formula led by `=`.
#[derive(Debug, Clone, PartialEq, Eq, Deref, Reflect, Component)]
#[reflect(Component)]
pub struct CellFormula(pub SmolStr);

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

/// The addressed cells of a document, the one way any table's cells are
/// found, read and locked.
#[derive(SystemParam)]
pub struct Cells<'w, 's> {
	addresses: Query<'w, 's, &'static CellAddress>,
	formulas: Query<'w, 's, &'static CellFormula>,
	elements: Query<'w, 's, &'static Element>,
	views: ElementQuery<'w, 's>,
	children: Query<'w, 's, &'static Children>,
	parents: Query<'w, 's, &'static ChildOf>,
	text: DocumentText<'w, 's>,
}

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
			let numbered =
				world.with_state::<Cells, _>(|cells| cells.numbering(root));
			for (entity, address) in numbered.cells {
				world.entity_mut(entity).insert(address);
			}
			if number.caption {
				for (table, number) in numbered.tables {
					let caption = world
						.spawn((Element::new("caption"), children![
							Value::Str(format!("t{number}").into())
						]))
						.id();
					world.entity_mut(table).insert_child(0, caption);
				}
			}
			world.entity_mut(root).remove::<NumberTables>();
		}
	}
}

/// What [`Cells::numbering`] found under one root.
struct Numbering {
	/// Each numbered table with its number, in document order.
	tables: Vec<(Entity, NonZeroU32)>,
	/// Each cell of a numbered table with its address.
	cells: Vec<(Entity, CellAddress)>,
}

impl CellAddress {
	/// Parses either spelling: `t<n>r<n>c<n>`, every number from 1, or
	/// `<sheet>!<A1>`, a quoted sheet name, `'Start Here'!D3`, losing its
	/// quotes.
	pub fn parse(text: &str) -> Result<Self> {
		Self::parse_ordinal(text)
			.or_else(|| Self::parse_named(text))
			.ok_or_else(|| {
				bevyhow!(
					"`{text}` is not a cell, expected `t<n>r<n>c<n>` or `<sheet>!<A1>`"
				)
			})
	}

	fn parse_ordinal(text: &str) -> Option<Self> {
		let rest = text.strip_prefix('t')?;
		let (table, rest) = rest.split_once('r')?;
		let (row, column) = rest.split_once('c')?;
		Self {
			grid: CellGrid::Ordinal(Self::parse_count(table)?),
			column: Self::parse_count(column)?,
			row: Self::parse_count(row)?,
		}
		.xsome()
	}

	fn parse_named(text: &str) -> Option<Self> {
		let (sheet, cell) = text.rsplit_once('!')?;
		let sheet = sheet
			.strip_prefix('\'')
			.and_then(|sheet| sheet.strip_suffix('\''))
			.unwrap_or(sheet);
		let (column, row) = Self::parse_a1(cell)?;
		(!sheet.is_empty()).then(|| Self {
			grid: CellGrid::Named(sheet.into()),
			column,
			row,
		})
	}

	/// The cell's `A1` reference, ie `D3`.
	pub fn a1(&self) -> String {
		format!("{}{}", Self::column_letters(self.column), self.row)
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

	/// The column and row of an `A1` reference, one to three uppercase
	/// letters then a row with no leading zero.
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
		Some((column, Self::parse_count(row)?))
	}

	/// A count from 1 in plain digits, no leading zero.
	fn parse_count(text: &str) -> Option<NonZeroU32> {
		(!text.starts_with('0')
			&& text.chars().all(|char| char.is_ascii_digit()))
		.then(|| text.parse().ok())
		.flatten()
		.and_then(NonZeroU32::new)
	}

	/// The count of the item at `index` from 0, ie a table's place among the
	/// tables before it.
	fn count(index: usize) -> NonZeroU32 {
		NonZeroU32::MIN.saturating_add(index as u32)
	}
}

/// `t3r2c1` for an [`Ordinal`](CellGrid::Ordinal) grid, `Budget!D3` for a
/// [`Named`](CellGrid::Named) one.
impl core::fmt::Display for CellAddress {
	fn fmt(
		&self,
		formatter: &mut core::fmt::Formatter<'_>,
	) -> core::fmt::Result {
		match &self.grid {
			CellGrid::Ordinal(table) => {
				write!(formatter, "t{table}r{}c{}", self.row, self.column)
			}
			CellGrid::Named(sheet) => {
				write!(formatter, "{sheet}!{}", self.a1())
			}
		}
	}
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
	/// Every addressed cell under `root` a person may type into, in document
	/// order: the map an edit addresses a document by.
	pub fn listing(world: &mut World, root: Entity) -> Vec<CellText> {
		world.with_state::<Cells, _>(|cells| cells.listing(root))
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

impl Cells<'_, '_> {
	/// Whether `tag` is a table cell's, `td` or `th`.
	pub fn is_cell_tag(tag: &str) -> bool { matches!(tag, "td" | "th") }

	/// Whether `tag` is a table's own structure: the table, a row or a cell.
	pub fn is_table_tag(tag: &str) -> bool {
		matches!(tag, "table" | "tr") || Self::is_cell_tag(tag)
	}

	/// Every addressed cell under `root` in document order, with its address.
	pub fn cells(&self, root: Entity) -> Vec<(CellAddress, Entity)> {
		self.children
			.iter_descendants_depth_first(root)
			.filter_map(|entity| {
				self.addresses
					.get(entity)
					.ok()
					.map(|address| (address.clone(), entity))
			})
			.collect()
	}

	/// The cell at `address` under `root`: the cell carrying it, else the
	/// cell whose `colspan` and `rowspan` cover that slot, so naming any slot
	/// of a merged cell answers the merged cell.
	pub fn cell(&self, root: Entity, address: &CellAddress) -> Option<Entity> {
		let cells = self.cells(root);
		cells
			.iter()
			.find(|(found, _)| found == address)
			.or_else(|| {
				cells
					.iter()
					.find(|(found, cell)| self.covers(found, *cell, address))
			})
			.map(|(_, cell)| *cell)
	}

	/// The listing of every addressed cell under `root` a person may type
	/// into, in document order.
	pub fn listing(&self, root: Entity) -> Vec<CellText> {
		self.cells(root)
			.into_iter()
			.filter(|(_, cell)| !self.is_locked(*cell))
			.map(|(address, cell)| CellText {
				address: address.to_string().into(),
				text: self.cell_text(cell).into(),
			})
			.collect()
	}

	/// What a reader reads in `cell`: a formula as `=` and its expression,
	/// else its blocks' text, each trimmed, the empty dropped and the rest
	/// joined by ` / `, and its own text when it holds no block.
	pub fn cell_text(&self, cell: Entity) -> String {
		if let Ok(formula) = self.formulas.get(cell) {
			return format!("={}", formula.0);
		}
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

	/// Whether `cell`, at `found`, spans the slot `address` names.
	fn covers(
		&self,
		found: &CellAddress,
		cell: Entity,
		address: &CellAddress,
	) -> bool {
		let (columns, rows) = self.span(cell);
		let within = |start: NonZeroU32, span: u32, at: NonZeroU32| {
			(start.get()..start.get().saturating_add(span)).contains(&at.get())
		};
		found.grid == address.grid
			&& within(found.column, columns, address.column)
			&& within(found.row, rows, address.row)
	}

	/// The columns and rows `cell` spans, as its `colspan` and `rowspan` say.
	fn span(&self, cell: Entity) -> (u32, u32) {
		self.views
			.get(cell)
			.map(|view| (view.colspan(), view.rowspan()))
			.unwrap_or((1, 1))
	}

	/// The number of every unaddressed table under `root` outside a
	/// `header`, `footer` or `aside`, in document order as the tables open,
	/// and the address of each of its cells: rows as their table's own,
	/// columns as HTML's table model places each cell, a nested table's left
	/// to it.
	fn numbering(&self, root: Entity) -> Numbering {
		let mut tables = Vec::new();
		self.tables_under(root, &mut tables);
		let mut numbering = Numbering {
			tables: Vec::new(),
			cells: Vec::new(),
		};
		for (index, table) in tables.into_iter().enumerate() {
			let number = CellAddress::count(index);
			numbering.tables.push((table, number));
			let rows = self.within(table, Self::is_row_tag);
			// the slots a `rowspan` from a row above still holds, by row
			let mut held = HashSet::<(usize, u32)>::default();
			for (row_index, row) in rows.iter().enumerate() {
				let mut column = 1;
				for cell in self.within(*row, Self::is_cell_tag) {
					while held.contains(&(row_index, column)) {
						column += 1;
					}
					let (columns, spanned) = self.span(cell);
					// a span past the table's last row holds nothing
					let below = (spanned as usize)
						.min(rows.len() - row_index)
						.saturating_sub(1);
					for row_offset in 1..=below {
						for offset in 0..columns {
							held.insert((
								row_index + row_offset,
								column + offset,
							));
						}
					}
					numbering.cells.push((cell, CellAddress {
						grid: CellGrid::Ordinal(number),
						column: NonZeroU32::MIN.saturating_add(column - 1),
						row: CellAddress::count(row_index),
					}));
					column += columns;
				}
			}
		}
		numbering
	}

	/// Every unaddressed table under `parent` in document order, each before
	/// the tables nested in it, none under a `header`, `footer` or `aside`.
	fn tables_under(&self, parent: Entity, out: &mut Vec<Entity>) {
		for child in self.children.get(parent).into_iter().flatten() {
			match self.tag(*child) {
				Some("header" | "footer" | "aside") => continue,
				Some("table") if !self.is_addressed(*child) => out.push(*child),
				_ => {}
			}
			self.tables_under(*child, out);
		}
	}

	/// Whether any cell of `table`, a nested table's aside, carries a
	/// [`CellAddress`], ie the format addressed it.
	fn is_addressed(&self, table: Entity) -> bool {
		self.within(table, Self::is_row_tag).into_iter().any(|row| {
			self.within(row, Self::is_cell_tag)
				.into_iter()
				.any(|cell| self.addresses.contains(cell))
		})
	}

	fn is_row_tag(tag: &str) -> bool { tag == "tr" }

	/// The elements whose tag `is` under `parent`, in order, through every
	/// entity between that is no table, row or cell, ie a `<tbody>` or a
	/// node a reader never sees.
	fn within(&self, parent: Entity, is: fn(&str) -> bool) -> Vec<Entity> {
		let mut out = Vec::new();
		for child in self.children.get(parent).into_iter().flatten() {
			match self.tag(*child) {
				Some(tag) if is(tag) => out.push(*child),
				Some(tag) if Self::is_table_tag(tag) => {}
				_ => out.extend(self.within(*child, is)),
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
	use core::num::NonZeroU32;

	/// The addresses a listing of `scene`'s numbered tables prints, with
	/// each cell's text.
	fn listing(scene: impl Bundle) -> Vec<String> {
		let mut world = ParsePlugin.into_world();
		let root = world.spawn((NumberTables::default(), scene)).id();
		world.run_schedule(PostParseTree);
		world.entity(root).contains::<NumberTables>().xpect_false();
		CellText::listing(&mut world, root)
			.iter()
			.map(ToString::to_string)
			.collect()
	}

	#[beet_core::test]
	fn parses_both_spellings() {
		let cell = CellAddress::parse("t3r2c1").unwrap();
		cell.grid
			.clone()
			.xpect_eq(CellGrid::Ordinal(NonZeroU32::new(3).unwrap()));
		(cell.row.get(), cell.column.get()).xpect_eq((2, 1));
		cell.to_string().xpect_eq("t3r2c1");
		let cell = CellAddress::parse("Start Here!D3").unwrap();
		cell.grid
			.clone()
			.xpect_eq(CellGrid::Named("Start Here".into()));
		(cell.column.get(), cell.row.get()).xpect_eq((4, 3));
		cell.to_string().xpect_eq("Start Here!D3");
		CellAddress::parse("'Table 1'!AA10")
			.unwrap()
			.to_string()
			.xpect_eq("Table 1!AA10");
		// a sheet may be named like a table cell
		CellAddress::parse("t1r1c1!A1")
			.unwrap()
			.grid
			.xpect_eq(CellGrid::Named("t1r1c1".into()));
		CellAddress::column_letters(NonZeroU32::new(28).unwrap())
			.xpect_eq("AB");
		for bad in [
			"t0r1c1",
			"t1r1",
			"t01r1c1",
			"x1r1c1",
			"Sheet!3",
			"!A1",
			"Sheet!A01",
			"A1",
			"Sheet!a1",
			"Sheet!ABCD1",
		] {
			CellAddress::parse(bad)
				.unwrap_err()
				.to_string()
				.xpect_contains("expected `t<n>r<n>c<n>` or `<sheet>!<A1>`");
		}
	}

	/// Tables number as they open, a nested one after its host, a table
	/// under a `header`, `footer` or `aside` is no table to number, and a
	/// cell reads as its paragraphs joined.
	#[beet_core::test]
	fn numbers_and_reads_cells() {
		listing(rsx! {
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
		})
		.xpect_eq(vec![
			"| t1r1c1 | Name / Here |".to_string(),
			"| t1r1c2 | (empty) |".into(),
			"| t2r1c1 | inner |".into(),
			"| t3r1c1 | a \\| b |".into(),
			"| t3r1c2 | (empty) |".into(),
		]);
	}

	/// A column is the visual column a cell sits in: a `colspan` moves the
	/// cells after it, and naming any slot it covers answers the spanning
	/// cell.
	#[beet_core::test]
	fn numbers_visual_columns() {
		let mut world = ParsePlugin.into_world();
		let root = world
			.spawn((NumberTables::default(), rsx! {
				<div><table>
					<tr><td colspan="2">"Name"</td><td>"Acme"</td></tr>
					<tr><td>"a"</td><td>"b"</td><td>"c"</td></tr>
				</table></div>
			}))
			.id();
		world.run_schedule(PostParseTree);
		CellText::listing(&mut world, root)
			.iter()
			.map(ToString::to_string)
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				"| t1r1c1 | Name |".to_string(),
				"| t1r1c3 | Acme |".into(),
				"| t1r2c1 | a |".into(),
				"| t1r2c2 | b |".into(),
				"| t1r2c3 | c |".into(),
			]);
		world
			.with_state::<Cells, _>(|cells| {
				let cell = cells
					.cell(root, &CellAddress::parse("t1r1c2").unwrap())
					.unwrap();
				cells.cell_text(cell)
			})
			.xpect_eq("Name");
	}

	/// A `rowspan` holds its column in the rows below, shifting their cells
	/// past it, and an absent, zero or unparseable span is 1.
	#[beet_core::test]
	fn rowspans_shift_the_rows_below() {
		listing(rsx! {
			<div><table>
				<tr><td rowspan="2">"tall"</td><td colspan="0">"b"</td><td colspan="x">"c"</td></tr>
				<tr><td>"d"</td><td>"e"</td></tr>
				<tr><td>"f"</td></tr>
			</table></div>
		})
		.xpect_eq(vec![
			"| t1r1c1 | tall |".to_string(),
			"| t1r1c2 | b |".into(),
			"| t1r1c3 | c |".into(),
			"| t1r2c2 | d |".into(),
			"| t1r2c3 | e |".into(),
			"| t1r3c1 | f |".into(),
		]);
	}
}
