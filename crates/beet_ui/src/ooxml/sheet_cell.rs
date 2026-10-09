//! A workbook's cells: their address, what the sheet says about each, and
//! the [`SheetCells`] that read them.
use crate::prelude::*;
use beet_core::prelude::*;

/// A workbook cell, `<sheet>!<column><row>`, ie `Start Here!D3`: the sheet by
/// its tab name, the column in letters and the row from 1.
#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, Component,
)]
#[reflect(Component)]
pub struct SheetCellAddress {
	/// The sheet's tab name.
	pub sheet: SmolStr,
	/// The column, `A` being 1.
	pub column: u32,
	/// The row, from 1.
	pub row: u32,
}

impl SheetCellAddress {
	/// Parses `<sheet>!<A1>`; a quoted sheet name, `'Start Here'!D3`, loses
	/// its quotes.
	pub fn parse(text: &str) -> Result<Self> {
		let invalid = || {
			bevyhow!("`{text}` is not a workbook cell, expected `<sheet>!<A1>`")
		};
		let (sheet, cell) = text.rsplit_once('!').ok_or_else(invalid)?;
		let sheet = sheet
			.strip_prefix('\'')
			.and_then(|sheet| sheet.strip_suffix('\''))
			.unwrap_or(sheet);
		let (column, row) = Self::parse_a1(cell).ok_or_else(invalid)?;
		match sheet.is_empty() {
			true => Err(invalid()),
			false => Self {
				sheet: sheet.into(),
				column,
				row,
			}
			.xok(),
		}
	}

	/// A cell of `sheet` from its `A1` reference.
	pub fn from_a1(sheet: impl Into<SmolStr>, a1: &str) -> Result<Self> {
		let (column, row) = Self::parse_a1(a1)
			.ok_or_else(|| bevyhow!("`{a1}` is not an A1 cell reference"))?;
		Self {
			sheet: sheet.into(),
			column,
			row,
		}
		.xok()
	}

	/// The cell's `A1` reference, ie `D3`.
	pub fn a1(&self) -> String {
		format!("{}{}", Self::column_letters(self.column), self.row)
	}

	/// The letters naming column `column`, `1` being `A` and `27` `AA`.
	pub fn column_letters(column: u32) -> String {
		let mut letters = Vec::new();
		let mut rest = column;
		while rest > 0 {
			letters.push(b'A' + ((rest - 1) % 26) as u8);
			rest = (rest - 1) / 26;
		}
		letters.reverse();
		String::from_utf8(letters).unwrap_or_default()
	}

	/// The column and row of an `A1` reference, uppercase letters then a row
	/// with no leading zero.
	pub fn parse_a1(a1: &str) -> Option<(u32, u32)> {
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
			.fold(0, |acc, byte| acc * 26 + (byte - b'A' + 1) as u32);
		Some((column, TableCellAddress::parse_count(row)?))
	}
}

impl core::fmt::Display for SheetCellAddress {
	fn fmt(
		&self,
		formatter: &mut core::fmt::Formatter<'_>,
	) -> core::fmt::Result {
		write!(formatter, "{}!{}", self.sheet, self.a1())
	}
}

/// A cell a person may not type into, ie a protected workbook's label: an
/// edit setting its text is refused.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect, Component)]
#[reflect(Component, Default)]
pub struct CellLocked;

/// The formula a workbook cell computes, as written without its `=`: its
/// shown text is the value last computed, and a writer refuses to replace it.
#[derive(Debug, Clone, PartialEq, Eq, Deref, Reflect, Component)]
#[reflect(Component)]
pub struct CellFormula(pub SmolStr);

/// A merged range's covered cell, ie `E3` of `D3:F3`: it shows, and an edit
/// writes, the range's first cell, the entity held here. It has an address
/// but no `<td>` of its own, the first cell spanning it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deref, Reflect, Component)]
#[reflect(Component)]
pub struct CoveredBy(pub Entity);

/// The cell format a gap's cell is written with when an edit sets a value
/// there: its row's when the row carries one, else its column's. A cell the
/// sheet already holds names its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deref, Reflect, Component)]
#[reflect(Component)]
pub struct SheetCellStyle(pub u32);

/// The addressed cells of a workbook, and the text a reader reads in each.
#[derive(SystemParam)]
pub struct SheetCells<'w, 's> {
	sheets: Query<
		'w,
		's,
		(
			&'static SheetCellAddress,
			Has<CellLocked>,
			Option<&'static CellFormula>,
			Option<&'static CoveredBy>,
		),
	>,
	children: Query<'w, 's, &'static Children>,
	tables: TableCells<'w, 's>,
}

impl SheetCells<'_, '_> {
	/// Every workbook cell under `root`, sheet by sheet in tab order and row
	/// by row, with its address.
	pub fn sheet_cells(&self, root: Entity) -> Vec<(SheetCellAddress, Entity)> {
		self.children
			.iter_descendants_depth_first(root)
			.filter_map(|entity| {
				self.sheets
					.get(entity)
					.ok()
					.map(|(address, ..)| (address.clone(), entity))
			})
			.collect()
	}

	/// The workbook cell at `address` under `root`, a covered cell answering
	/// its range's first.
	pub fn sheet_cell(
		&self,
		root: Entity,
		address: &SheetCellAddress,
	) -> Option<Entity> {
		self.children
			.iter_descendants_depth_first(root)
			.find(|entity| {
				self.sheets
					.get(*entity)
					.is_ok_and(|(found, ..)| found == address)
			})
			.map(|entity| self.shown(entity))
	}

	/// Whether the workbook cell `cell` is locked.
	pub fn is_locked(&self, cell: Entity) -> bool {
		self.sheets.get(cell).is_ok_and(|(_, locked, ..)| locked)
	}

	/// The listing of every unlocked workbook cell under `root`, a covered
	/// cell showing its range's first.
	pub fn listing(&self, root: Entity) -> Vec<CellText> {
		self.sheet_cells(root)
			.into_iter()
			.filter(|(_, cell)| !self.is_locked(*cell))
			.map(|(address, cell)| CellText {
				address: address.to_string().into(),
				text: self.cell_text(self.shown(cell)).into(),
			})
			.collect()
	}

	/// What a reader reads in `cell`, a formula as `=` and its expression.
	pub fn cell_text(&self, cell: Entity) -> String {
		match self.sheets.get(cell) {
			Ok((_, _, Some(formula), _)) => format!("={}", formula.0),
			_ => self.tables.cell_text(cell),
		}
	}

	/// The cell `cell` shows: its range's first when it is covered.
	fn shown(&self, cell: Entity) -> Entity {
		match self.sheets.get(cell) {
			Ok((_, _, _, Some(covered))) => covered.0,
			_ => cell,
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn parses_sheet_cells() {
		let cell = SheetCellAddress::parse("Start Here!D3").unwrap();
		cell.sheet.as_str().xpect_eq("Start Here");
		(cell.column, cell.row).xpect_eq((4, 3));
		cell.to_string().xpect_eq("Start Here!D3");
		SheetCellAddress::parse("'Table 1'!AA10")
			.unwrap()
			.to_string()
			.xpect_eq("Table 1!AA10");
		SheetCellAddress::column_letters(28).xpect_eq("AB");
		for bad in ["Sheet!3", "!A1", "Sheet!A01", "A1", "Sheet!a1"] {
			SheetCellAddress::parse(bad).xpect_err();
		}
	}
}
