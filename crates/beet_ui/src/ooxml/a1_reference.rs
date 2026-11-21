//! [`A1Reference`], the way a workbook names a slot of a sheet's grid.
use beet_core::prelude::*;

/// A cell's A1 reference as a workbook writes it, its column's letters then
/// its row counted from 1, ie `B3` for the grid slot `UVec2::new(1, 2)`.
///
/// [`TableGrid`](crate::prelude::TableGrid) counts slots from 0; this is
/// where a workbook's counting converts, as a date's text converts at its
/// parse and display, so a reader parses here and a writer displays here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct A1Reference(pub UVec2);

impl A1Reference {
	/// The furthest slot a workbook holds, `XFD1048576`.
	pub const MAX: UVec2 = UVec2::new(16_383, 1_048_575);

	/// Parses one to three uppercase letters then a row with no leading
	/// zero, within [`Self::MAX`].
	pub fn parse(reference: &str) -> Result<Self> {
		let letters = reference
			.bytes()
			.take_while(|byte| byte.is_ascii_uppercase())
			.count();
		let (column, row) = reference.split_at(letters);
		let column = (1..=3).contains(&column.len()).then(|| {
			column
				.bytes()
				.fold(0, |acc, byte| acc * 26 + (byte - b'A' + 1) as u32)
				- 1
		});
		let row = (!row.starts_with('0')
			&& !row.is_empty()
			&& row.bytes().all(|byte| byte.is_ascii_digit()))
		.then(|| row.parse::<u32>().ok())
		.flatten()
		.map(|row| row - 1);
		match (column, row) {
			(Some(column), Some(row))
				if column <= Self::MAX.x && row <= Self::MAX.y =>
			{
				Self(UVec2::new(column, row)).xok()
			}
			_ => bevybail!(
				"`{reference}` is not an A1 reference, expected its column's letters then its row counted from 1, from `A1` to `XFD1048576`"
			),
		}
	}

	/// The letters naming the column counted from 0, `0` being `A` and `26`
	/// `AA`.
	pub fn column_letters(column: u32) -> String {
		let mut letters = Vec::new();
		let mut rest = column as u64 + 1;
		while rest > 0 {
			letters.push(b'A' + ((rest - 1) % 26) as u8);
			rest = (rest - 1) / 26;
		}
		letters.reverse();
		String::from_utf8(letters).unwrap_or_default()
	}
}

/// The reference, ie `B3`.
impl core::fmt::Display for A1Reference {
	fn fmt(
		&self,
		formatter: &mut core::fmt::Formatter<'_>,
	) -> core::fmt::Result {
		write!(
			formatter,
			"{}{}",
			Self::column_letters(self.0.x),
			self.0.y as u64 + 1
		)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn converts_both_ways() {
		for (reference, column, row) in [
			("A1", 0, 0),
			("B3", 1, 2),
			("Z1", 25, 0),
			("AA10", 26, 9),
			("XFD1048576", 16_383, 1_048_575),
		] {
			let parsed = A1Reference::parse(reference).unwrap();
			parsed.0.xpect_eq(UVec2::new(column, row));
			parsed.to_string().xpect_eq(reference);
		}
		for bad in [
			"", "3", "b3", "A0", "A01", "ABCD1", "A", "A1B", "XFE1", "A1048577",
		] {
			A1Reference::parse(bad)
				.unwrap_err()
				.to_string()
				.xpect_contains("is not an A1 reference");
		}
	}
}
