use beet_core::prelude::*;

/// A Word table cell, `t<table>r<row>c<cell>`, each counted from 1: tables in
/// document order as they open, nested ones included, rows and cells as the
/// direct children of their table and row. `t3r2c1` is the first cell of the
/// second row of the third table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TableCellAddress {
	/// The table, from 1.
	pub table: u32,
	/// The row within it, from 1.
	pub row: u32,
	/// The cell within the row, from 1.
	pub cell: u32,
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
			table: count(table)?,
			row: count(row)?,
			cell: count(cell)?,
		}
		.xsome()
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

/// A workbook cell, `<sheet>!<column><row>`, ie `Start Here!D3`: the sheet by
/// its tab name, the column in letters and the row from 1.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
		Some((column, count(row)?))
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

/// A count from 1 in plain digits, no leading zero.
fn count(text: &str) -> Option<u32> {
	(!text.is_empty()
		&& !text.starts_with('0')
		&& text.chars().all(|char| char.is_ascii_digit()))
	.then(|| text.parse().ok())
	.flatten()
}

#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	fn parses_table_cells() {
		let cell = TableCellAddress::parse("t3r2c1").unwrap();
		(cell.table, cell.row, cell.cell).xpect_eq((3, 2, 1));
		cell.to_string().xpect_eq("t3r2c1");
		for bad in ["t0r1c1", "t1r1", "t01r1c1", "x1r1c1"] {
			TableCellAddress::parse(bad).xpect_err();
		}
	}

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
