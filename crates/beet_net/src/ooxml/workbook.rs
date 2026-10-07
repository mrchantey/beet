use crate::prelude::*;
use beet_core::prelude::*;
use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
use ooxmlsdk::parts::worksheet_part::WorksheetPart;
use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as sml;
use ooxmlsdk::simple_type::BooleanValue;

/// A workbook, `.xlsx`, read and written through `ooxmlsdk`'s typed parts:
/// sheets by tab name, rows and cells, the styles that lock a cell, the merges
/// and the shared strings. A protected form takes input only in its unlocked
/// cells, and those are what [`unlocked_cells`](Self::unlocked_cells) lists
/// and [`set`](Self::set) writes.
///
/// A cell's lock is its style's `protection`, locked unless it says
/// otherwise; a cell absent from the sheet takes its row's style when the row
/// carries one, else its column's. A cell inside a merged range reads, and is
/// written as, the range's first cell.
pub struct Workbook {
	/// The file's parts.
	file: SpreadsheetDocument,
}

/// One cell of a [`Workbook`]: its address and its value as text, a formula
/// as `=` and its expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbookCell {
	/// Where the cell is.
	pub address: SheetCellAddress,
	/// What it holds.
	pub value: String,
}

/// The cells dump's row, `| Sheet!A1 | value |`, an empty cell `(empty)`.
impl core::fmt::Display for WorkbookCell {
	fn fmt(
		&self,
		formatter: &mut core::fmt::Formatter<'_>,
	) -> core::fmt::Result {
		write!(
			formatter,
			"| {} | {} |",
			self.address,
			super::dump_text(&self.value)
		)
	}
}

impl Workbook {
	/// Opens a workbook from its bytes.
	pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Result<Self> {
		Self {
			file: SpreadsheetDocument::new(std::io::Cursor::new(bytes.into()))?,
		}
		.xok()
	}

	/// A new workbook of `sheets`, each a tab name and the worksheet's inner
	/// SpreadsheetML (`<sheetData>` onwards), styled by `cell_formats`, the
	/// `<xf>` elements of the styles' `cellXfs`; ie for a test fixture.
	pub fn from_sheets(
		cell_formats: &str,
		sheets: &[(&str, &str)],
	) -> Result<Self> {
		const SML: &str =
			"http://schemas.openxmlformats.org/spreadsheetml/2006/main";
		const REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
		let mut file = SpreadsheetDocument::create(Default::default());
		let workbook = file.add_workbook_part()?;
		let mut listed = String::new();
		for (index, (name, inner)) in sheets.iter().enumerate() {
			let id = format!("rIdSheet{}", index + 1);
			let sheet: WorksheetPart = workbook.add_new_part(&mut file, &id)?;
			sheet.set_data(
				&mut file,
				format!(
					"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\r\n\
					 <worksheet xmlns=\"{SML}\" xmlns:r=\"{REL}\">{inner}</worksheet>"
				),
			)?;
			listed.push_str(&format!(
				"<sheet name=\"{name}\" sheetId=\"{}\" r:id=\"{id}\"/>",
				index + 1
			));
		}
		let styles: ooxmlsdk::parts::workbook_styles_part::WorkbookStylesPart =
			workbook.add_new_part(&mut file, "rIdStyles")?;
		styles.set_data(
			&mut file,
			format!(
				"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\r\n\
				 <styleSheet xmlns=\"{SML}\"><fonts count=\"1\"><font/></fonts>\
				 <fills count=\"1\"><fill/></fills><borders count=\"1\"><border/></borders>\
				 <cellXfs>{cell_formats}</cellXfs></styleSheet>"
			),
		)?;
		workbook.set_data(
			&mut file,
			format!(
				"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\r\n\
				 <workbook xmlns=\"{SML}\" xmlns:r=\"{REL}\"><sheets>{listed}</sheets></workbook>"
			),
		)?;
		Self::from_bytes(file.to_package_bytes()?)
	}

	/// Opens the workbook `blob` holds.
	pub async fn open(blob: &Blob) -> Result<Self> {
		Self::from_bytes(blob.get().await?.to_vec())
	}

	/// The file's bytes.
	pub fn to_bytes(&self) -> Result<Vec<u8>> {
		self.file.to_package_bytes()?.xok()
	}

	/// Writes the file to `blob`.
	pub async fn save(&self, blob: &Blob) -> Result {
		blob.insert(self.to_bytes()?).await
	}

	/// The file's parts, for what the reads here do not reach.
	pub fn file(&self) -> &SpreadsheetDocument { &self.file }

	/// Every unlocked cell, sheet by sheet in tab order and row by row, with
	/// its value.
	pub fn unlocked_cells(&self) -> Result<Vec<WorkbookCell>> {
		let styles = self.unlocked_styles()?;
		let strings = self.shared_strings()?;
		let mut out = Vec::new();
		for (name, part) in self.sheets()? {
			let sheet = part.root_element(&self.file)?;
			let layout = SheetLayout::new(sheet, &strings);
			for row in &sheet.sheet_data.row {
				let Some(row_index) = row.row_index else {
					continue;
				};
				let present = SheetLayout::row_cells(row);
				let last = present.keys().max().copied().unwrap_or(0);
				for column in 1..=last {
					let style = layout.style(
						row,
						present.get(&column).copied(),
						column,
					);
					if !styles.get(style as usize).copied().unwrap_or(false) {
						continue;
					}
					out.push(WorkbookCell {
						address: SheetCellAddress {
							sheet: name.clone(),
							column,
							row: row_index,
						},
						value: layout.value(column, row_index),
					});
				}
			}
		}
		out.xok()
	}

	/// The value of one cell, a merged cell answering its range's first.
	pub fn value(&self, address: &SheetCellAddress) -> Result<String> {
		let strings = self.shared_strings()?;
		let part = self.sheet(&address.sheet)?;
		let sheet = part.root_element(&self.file)?;
		SheetLayout::new(sheet, &strings)
			.value(address.column, address.row)
			.xok()
	}

	/// Writes `value` into an unlocked cell, a number when it parses as one
	/// and text otherwise, and answers the cell written: the range's first
	/// for a merged cell. A locked cell and a formula are refused. The
	/// workbook is marked to recalculate on open, since nothing here
	/// computes a formula.
	pub fn set(
		&mut self,
		address: &SheetCellAddress,
		value: &str,
	) -> Result<SheetCellAddress> {
		if value.trim().starts_with('=') {
			bevybail!("{address}: a formula is not a value");
		}
		let styles = self.unlocked_styles()?;
		let part = self.sheet(&address.sheet)?;
		let sheet = part.root_element_mut(&mut self.file)?;
		let (column, row_index) =
			SheetLayout::master(sheet, address.column, address.row);
		let target = SheetCellAddress {
			sheet: address.sheet.clone(),
			column,
			row: row_index,
		};
		let existing_row = sheet
			.sheet_data
			.row
			.iter()
			.find(|row| row.row_index == Some(row_index));
		let style = match existing_row
			.and_then(|row| SheetLayout::row_cells(row).get(&column).copied())
		{
			Some(cell) => cell.style_index.unwrap_or(0),
			None => SheetLayout::inherited_style(
				&sheet.columns,
				existing_row.unwrap_or(&sml::Row::default()),
				column,
			),
		};
		if !styles.get(style as usize).copied().unwrap_or(false) {
			bevybail!(
				"{target} is locked; only an unlocked cell takes a value"
			);
		}
		let row = SheetLayout::row_mut(sheet, row_index);
		let cell = SheetLayout::cell_mut(row, column, style);
		let trimmed = value.trim();
		cell.cell_formula = None;
		match is_number(trimmed) {
			true => {
				cell.data_type = None;
				cell.inline_string = None;
				cell.cell_value = Some(sml::CellValue(sml::XstringType {
					space: None,
					xml_content: Some(trimmed.to_string()),
				}));
			}
			false => {
				cell.data_type = Some(sml::CellValues::InlineString);
				cell.cell_value = None;
				cell.inline_string = Some(Box::new(sml::InlineString {
					text: Some(sml::Text(sml::XstringType {
						space: Some(
							ooxmlsdk::schemas::xml::SpaceProcessingModeValues::Preserve,
						),
						xml_content: Some(value.to_string()),
					})),
					..Default::default()
				}));
			}
		}
		let workbook = self
			.file
			.workbook_part()?
			.root_element_mut(&mut self.file)?;
		workbook
			.calculation_properties
			.get_or_insert_with(Default::default)
			.full_calculation_on_load = Some(BooleanValue::One);
		target.xok()
	}

	/// The sheets in tab order, each with its part.
	fn sheets(&self) -> Result<Vec<(SmolStr, WorksheetPart)>> {
		let workbook_part = self.file.workbook_part()?;
		let workbook = workbook_part.root_element(&self.file)?;
		let mut parts: HashMap<String, WorksheetPart> = HashMap::default();
		for part in workbook_part.worksheet_parts(&self.file) {
			let id =
				workbook_part.get_id_of_part(&self.file, &part)?.to_string();
			parts.insert(id, part);
		}
		workbook
			.sheets
			.sheet
			.iter()
			// a chartsheet is a tab with no cells
			.filter_map(|sheet| {
				parts
					.remove(&sheet.id)
					.map(|part| (SmolStr::new(&sheet.name), part))
			})
			.collect::<Vec<_>>()
			.xok()
	}

	fn sheet(&self, name: &str) -> Result<WorksheetPart> {
		self.sheets()?
			.into_iter()
			.find(|(sheet, _)| sheet == name)
			.map(|(_, part)| part)
			.ok_or_else(|| bevyhow!("the workbook has no sheet `{name}`"))
	}

	/// Whether each cell format, by index, unlocks its cells.
	fn unlocked_styles(&self) -> Result<Vec<bool>> {
		let workbook_part = self.file.workbook_part()?;
		let Some(styles) = workbook_part.workbook_styles_part(&self.file)
		else {
			return Vec::new().xok();
		};
		styles
			.root_element(&self.file)?
			.cell_formats
			.iter()
			.flat_map(|formats| formats.xml_children.iter())
			.filter_map(|child| match child {
				sml::CellFormatsChoice::CellFormat(format) => Some(
					format
						.protection
						.as_ref()
						.and_then(|protection| protection.locked)
						.is_some_and(|locked| !locked.as_bool()),
				),
				_ => None,
			})
			.collect::<Vec<_>>()
			.xok()
	}

	fn shared_strings(&self) -> Result<Vec<String>> {
		let workbook_part = self.file.workbook_part()?;
		let Some(part) = workbook_part.shared_string_table_part(&self.file)
		else {
			return Vec::new().xok();
		};
		part.root_element(&self.file)?
			.shared_string_item
			.iter()
			.map(|item| {
				let mut text = item
					.text
					.as_ref()
					.and_then(|text| text.0.xml_content.clone())
					.unwrap_or_default();
				for run in &item.run {
					text.push_str(
						run.text.0.xml_content.as_deref().unwrap_or(""),
					);
				}
				text
			})
			.collect::<Vec<_>>()
			.xok()
	}
}

/// One sheet's cells by position, its merges and its column styles, for
/// reading a value or a lock.
struct SheetLayout<'a> {
	sheet: &'a sml::Worksheet,
	strings: &'a [String],
	/// Each merged cell's range's first cell.
	masters: HashMap<(u32, u32), (u32, u32)>,
}

impl<'a> SheetLayout<'a> {
	fn new(sheet: &'a sml::Worksheet, strings: &'a [String]) -> Self {
		let mut masters = HashMap::default();
		for merge in sheet
			.merge_cells
			.iter()
			.flat_map(|merges| merges.merge_cell.iter())
		{
			let Some((first, last)) = merge.reference.split_once(':') else {
				continue;
			};
			let (Some(first), Some(last)) = (
				SheetCellAddress::parse_a1(first),
				SheetCellAddress::parse_a1(last),
			) else {
				continue;
			};
			for row in first.1..=last.1 {
				for column in first.0..=last.0 {
					if (column, row) != first {
						masters.insert((column, row), first);
					}
				}
			}
		}
		Self {
			sheet,
			strings,
			masters,
		}
	}

	/// The range's first cell for a merged cell, else the cell itself.
	fn master(sheet: &sml::Worksheet, column: u32, row: u32) -> (u32, u32) {
		SheetLayout::new(sheet, &[])
			.masters
			.get(&(column, row))
			.copied()
			.unwrap_or((column, row))
	}

	/// A row's cells by column.
	fn row_cells(row: &sml::Row) -> HashMap<u32, &sml::Cell> {
		row.cell
			.iter()
			.filter_map(|cell| {
				let (column, _) = SheetCellAddress::parse_a1(
					cell.cell_reference.as_deref()?,
				)?;
				Some((column, cell))
			})
			.collect()
	}

	/// The style a cell is drawn with: its own when present, else its row's
	/// when the row carries one, else its column's.
	fn style(
		&self,
		row: &sml::Row,
		cell: Option<&sml::Cell>,
		column: u32,
	) -> u32 {
		match cell {
			Some(cell) => cell.style_index.unwrap_or(0),
			None => Self::inherited_style(&self.sheet.columns, row, column),
		}
	}

	fn inherited_style(
		columns: &[sml::Columns],
		row: &sml::Row,
		column: u32,
	) -> u32 {
		match row.custom_format.is_some_and(BooleanValue::as_bool) {
			true => row.style_index.unwrap_or(0),
			false => columns
				.iter()
				.flat_map(|columns| columns.column.iter())
				.find(|range| range.min <= column && column <= range.max)
				.and_then(|range| range.style)
				.unwrap_or(0),
		}
	}

	/// The value shown at a position, a merged cell showing its range's
	/// first.
	fn value(&self, column: u32, row: u32) -> String {
		let (column, row) = self
			.masters
			.get(&(column, row))
			.copied()
			.unwrap_or((column, row));
		self.sheet
			.sheet_data
			.row
			.iter()
			.find(|candidate| candidate.row_index == Some(row))
			.and_then(|row| {
				Self::row_cells(row)
					.get(&column)
					.map(|cell| self.show(cell))
			})
			.unwrap_or_default()
	}

	/// A cell's value as text: a formula as `=` and its expression, a shared
	/// string looked up, a boolean as `true` or `false`.
	fn show(&self, cell: &sml::Cell) -> String {
		if let Some(formula) = cell
			.cell_formula
			.as_ref()
			.and_then(|formula| formula.xml_content.as_deref())
		{
			return format!("={formula}");
		}
		let value = cell
			.cell_value
			.as_ref()
			.and_then(|value| value.0.xml_content.clone());
		match cell.data_type {
			Some(sml::CellValues::SharedString) => value
				.and_then(|index| {
					self.strings.get(index.parse::<usize>().ok()?).cloned()
				})
				.unwrap_or_default(),
			Some(sml::CellValues::InlineString) => cell
				.inline_string
				.as_ref()
				.map(|inline| {
					let mut text = inline
						.text
						.as_ref()
						.and_then(|text| text.0.xml_content.clone())
						.unwrap_or_default();
					for run in &inline.run {
						text.push_str(
							run.text.0.xml_content.as_deref().unwrap_or(""),
						);
					}
					text
				})
				.unwrap_or_default(),
			Some(sml::CellValues::Boolean) => match value.as_deref() {
				Some("1") => "true".into(),
				Some(_) => "false".into(),
				None => String::new(),
			},
			_ => value.unwrap_or_default(),
		}
	}

	/// The row numbered `index`, created in order when the sheet lacks it.
	fn row_mut(sheet: &mut sml::Worksheet, index: u32) -> &mut sml::Row {
		let rows = &mut sheet.sheet_data.row;
		let position = rows.iter().position(|row| {
			row.row_index.is_some_and(|existing| existing >= index)
		});
		match position {
			Some(position) if rows[position].row_index == Some(index) => {
				&mut rows[position]
			}
			position => {
				let position = position.unwrap_or(rows.len());
				rows.insert(position, sml::Row {
					row_index: Some(index),
					..Default::default()
				});
				&mut rows[position]
			}
		}
	}

	/// The row's cell at `column`, created in order with `style` when absent.
	fn cell_mut(row: &mut sml::Row, column: u32, style: u32) -> &mut sml::Cell {
		let row_index = row.row_index.unwrap_or(1);
		let column_of = |cell: &sml::Cell| {
			cell.cell_reference
				.as_deref()
				.and_then(SheetCellAddress::parse_a1)
				.map_or(0, |(column, _)| column)
		};
		let position =
			row.cell.iter().position(|cell| column_of(cell) >= column);
		match position {
			Some(position) if column_of(&row.cell[position]) == column => {
				&mut row.cell[position]
			}
			position => {
				let position = position.unwrap_or(row.cell.len());
				row.cell.insert(position, sml::Cell {
					cell_reference: Some(format!(
						"{}{row_index}",
						SheetCellAddress::column_letters(column)
					)),
					style_index: (style != 0).then_some(style),
					..Default::default()
				});
				&mut row.cell[position]
			}
		}
	}
}

/// Whether `text` is a plain decimal, `-?digits` with an optional fraction.
fn is_number(text: &str) -> bool {
	let digits = text.strip_prefix('-').unwrap_or(text);
	let (whole, fraction) = digits.split_once('.').unwrap_or((digits, "0"));
	!whole.is_empty()
		&& !fraction.is_empty()
		&& whole
			.chars()
			.chain(fraction.chars())
			.all(|char| char.is_ascii_digit())
}

#[cfg(test)]
mod test {
	use super::*;

	/// A protected sheet: `A1` a locked label, `D3:F3` an unlocked merged
	/// field holding a prompt, `D4` an unlocked formula, and column `G`
	/// unlocked by its column style where no cell is written.
	fn form() -> Workbook {
		Workbook::from_sheets(
			"<xf/><xf><protection locked=\"0\"/></xf>",
			&[(
				"Start Here",
				"<cols><col min=\"7\" max=\"7\" style=\"1\"/></cols><sheetData>\
				 <row r=\"1\"><c r=\"A1\" t=\"inlineStr\"><is><t>Business name</t></is></c></row>\
				 <row r=\"3\"><c r=\"D3\" s=\"1\" t=\"inlineStr\"><is><t>Enter it here</t></is></c>\
				 <c r=\"E3\" s=\"1\"/><c r=\"F3\" s=\"1\"/><c r=\"H3\"/></row>\
				 <row r=\"4\"><c r=\"D4\" s=\"1\"><f>1+1</f><v>2</v></c></row>\
				 </sheetData><sheetProtection sheet=\"1\"/><mergeCells count=\"1\"><mergeCell ref=\"D3:F3\"/></mergeCells>",
			)],
		)
		.unwrap()
	}

	fn cell(text: &str) -> SheetCellAddress {
		SheetCellAddress::parse(text).unwrap()
	}

	#[beet_core::test]
	fn lists_unlocked_cells() {
		form()
			.unlocked_cells()
			.unwrap()
			.iter()
			.map(ToString::to_string)
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				"| Start Here!D3 | Enter it here |".to_string(),
				"| Start Here!E3 | Enter it here |".into(),
				"| Start Here!F3 | Enter it here |".into(),
				"| Start Here!G3 | (empty) |".into(),
				"| Start Here!D4 | =1+1 |".into(),
			]);
	}

	#[beet_core::test]
	fn sets_unlocked_cells_only() {
		let mut form = form();
		form.set(&cell("Start Here!E3"), "Acme Stalls")
			.unwrap()
			.to_string()
			.xpect_eq("Start Here!D3");
		form.set(&cell("Start Here!G3"), "120.5").unwrap();
		form.set(&cell("Start Here!A1"), "x")
			.unwrap_err()
			.to_string()
			.xpect_contains("is locked");
		form.set(&cell("Start Here!G3"), "=2")
			.unwrap_err()
			.to_string()
			.xpect_contains("a formula is not a value");
		form.set(&cell("Nowhere!A1"), "x").xpect_err();

		let form = Workbook::from_bytes(form.to_bytes().unwrap()).unwrap();
		form.value(&cell("Start Here!F3"))
			.unwrap()
			.xpect_eq("Acme Stalls");
		form.value(&cell("Start Here!G3"))
			.unwrap()
			.xpect_eq("120.5");
	}
}
