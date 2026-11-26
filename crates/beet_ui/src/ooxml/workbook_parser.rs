//! [`WorkbookParser`], a workbook read as one article of one table per
//! worksheet, by the [module docs](super)' workbook mapping.
use super::office_style::*;
use super::read_element::*;
use crate::prelude::*;
use beet_core::prelude::*;
use beet_scene::prelude::*;

/// Reads a workbook into one `article data-worksheet` per worksheet in tab
/// order, each holding one `table` dense from `A1` to its last used cell,
/// with nothing Office-specific left on any entity. See the
/// [module docs](super) for the mapping, what it loses and the padding
/// bound.
#[derive(Debug, Default, Clone, Get, SetWith)]
pub struct WorkbookParser {
	/// How the workbook's looks are read.
	options: OfficeReadOptions,
}

/// What every sheet of a workbook shares: its strings, cell formats and
/// theme.
struct WorkbookRead {
	/// The shared strings, by index.
	strings: Vec<String>,
	/// The cell formats, `cellXfs`, by index.
	formats: Vec<CellFormat>,
	/// The theme its colours name slots of.
	theme: OfficeTheme,
}

/// One cell format of a styles part.
#[derive(Debug, Default, Clone, PartialEq)]
struct CellFormat {
	/// Its font.
	font: FontRead,
	/// Its solid fill.
	fill: Option<OfficeColor>,
	/// Its horizontal alignment.
	align: Option<TextAlign>,
	/// Whether it unlocks its cells on a protected sheet.
	unlocked: bool,
}

/// A font of a styles part.
#[derive(Debug, Default, Clone, PartialEq)]
struct FontRead {
	bold: bool,
	italic: bool,
	underline: bool,
	strike: bool,
	/// The size in points.
	size: Option<f32>,
	color: Option<OfficeColor>,
}

/// One worksheet as read.
struct SheetRead {
	/// Its tab name.
	name: SmolStr,
	/// Whether it is hidden from the tabs.
	hidden: bool,
	/// Whether it is protected, without which no cell is locked.
	protected: bool,
	/// Each column range's format, `(first, last, format)`, its columns
	/// counted from 0.
	columns: Vec<(u32, u32, u32)>,
	/// Each row's format, by the row counted from 0, where the row carries
	/// one for its absent cells.
	rows: HashMap<u32, u32>,
	/// Every cell the sheet writes, by its grid slot.
	cells: HashMap<UVec2, CellRead>,
	/// Every merged range, its first and last slot inclusive.
	merges: Vec<URect>,
	/// Every hyperlink, its range inclusive and its `href`.
	links: Vec<(URect, SmolStr)>,
}

/// One cell a sheet writes.
#[derive(Debug, Default, Clone)]
struct CellRead {
	/// Its value's text, `None` when it holds none.
	value: Option<String>,
	/// Its formula without the `=`.
	formula: Option<String>,
	/// Its format's index.
	format: u32,
}

impl WorkbookParser {
	/// The most cells one sheet pads to, its last used column times its last
	/// used row, before its read is refused.
	pub const MAX_SHEET_CELLS: u64 = 100_000;
	/// The most cells a workbook's sheets pad to together.
	pub const MAX_WORKBOOK_CELLS: u64 = 1_000_000;

	/// The relationship type of a worksheet.
	const WORKSHEET: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet";
	/// The relationship type of the shared strings.
	const SHARED_STRINGS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings";

	/// Reads `package`'s workbook into `root`.
	fn read(
		&self,
		world: &mut World,
		root: Entity,
		package: &OpcPackage,
	) -> Result {
		let workbook = package
			.main_part()
			.ok_or_else(|| bevyhow!("the workbook has no workbook part"))?
			.to_string();
		let shared = WorkbookRead {
			strings: package
				.related(Some(&workbook), Self::SHARED_STRINGS)
				.map(|path| package.xml(path))
				.transpose()?
				.flatten()
				.map(|nodes| Self::shared_strings(&nodes))
				.unwrap_or_default(),
			formats: package
				.related(Some(&workbook), OpcRelationship::STYLES)
				.map(|path| package.xml(path))
				.transpose()?
				.flatten()
				.map(|nodes| Self::cell_formats(&nodes))
				.unwrap_or_default(),
			theme: package
				.related(Some(&workbook), OpcRelationship::THEME)
				.map(|path| package.xml(path))
				.transpose()?
				.flatten()
				.map(|nodes| OfficeTheme::parse(&nodes))
				.unwrap_or_default(),
		};
		let nodes = package.xml(&workbook)?.unwrap_or_default();
		let sheets = BsxNode::document_element(&nodes)
			.and_then(|root| root.child("sheets"))
			.map(|sheets| {
				sheets
					.children_named("sheet")
					.filter_map(|sheet| {
						let id = sheet.prefixed_attribute("id")?;
						// a chartsheet has no cells to read
						let relationship =
							package.relationship(Some(&workbook), id).filter(
								|relationship| relationship.is(Self::WORKSHEET),
							)?;
						Some((
							SmolStr::new(sheet.attribute("name")?),
							matches!(
								sheet.attribute("state"),
								Some("hidden" | "veryHidden")
							),
							relationship.target.clone(),
						))
					})
					.collect::<Vec<_>>()
			})
			.unwrap_or_default();
		let mut read = Vec::new();
		let mut total = 0;
		for (name, hidden, path) in sheets {
			let nodes = package.xml(&path)?.unwrap_or_default();
			let Some(sheet_root) = BsxNode::document_element(&nodes) else {
				continue;
			};
			let sheet = Self::read_sheet(
				package, &path, name, hidden, sheet_root, &shared,
			)?;
			let size = sheet.extent(&shared);
			let area = size.x as u64 * size.y as u64;
			if area > Self::MAX_SHEET_CELLS {
				bevybail!(
					"sheet `{}`: its last used cell `{}` pads it to {area} cells, past the {} a sheet may hold",
					sheet.name,
					Self::corner(size),
					Self::MAX_SHEET_CELLS
				);
			}
			total += area;
			if total > Self::MAX_WORKBOOK_CELLS {
				bevybail!(
					"sheet `{}`: its last used cell `{}` pads the workbook to {total} cells, past the {} a workbook may hold",
					sheet.name,
					Self::corner(size),
					Self::MAX_WORKBOOK_CELLS
				);
			}
			read.push((sheet, size));
		}
		for (sheet, size) in read {
			sheet.article(&shared, size).spawn(
				world,
				root,
				&self.options,
				&shared.theme,
			)?;
		}
		Ok(())
	}

	/// The A1 reference of the last slot of a grid of `size`.
	fn corner(size: UVec2) -> String {
		A1Reference(size.max(UVec2::ONE) - UVec2::ONE).to_string()
	}

	/// The shared strings, each its text or its runs' text joined, a
	/// phonetic run left out.
	fn shared_strings(nodes: &[BsxNode]) -> Vec<String> {
		BsxNode::document_element(nodes)
			.map(|root| {
				root.children_named("si").map(Self::rich_text).collect()
			})
			.unwrap_or_default()
	}

	/// The text of a string item, `si` or `is`: its `t`, else its runs'.
	fn rich_text(item: &BsxElement) -> String {
		match item.child("t") {
			Some(text) => text.text(),
			None => item
				.children_named("r")
				.filter_map(|run| run.child("t"))
				.map(|text| text.text())
				.collect(),
		}
	}

	/// The cell formats of a styles part, `cellXfs`, each with its font and
	/// fill resolved.
	fn cell_formats(nodes: &[BsxNode]) -> Vec<CellFormat> {
		let Some(root) = BsxNode::document_element(nodes) else {
			return Vec::new();
		};
		let fonts = root
			.child("fonts")
			.map(|fonts| {
				fonts
					.children_named("font")
					.map(Self::font)
					.collect::<Vec<_>>()
			})
			.unwrap_or_default();
		let fills = root
			.child("fills")
			.map(|fills| {
				fills
					.children_named("fill")
					.map(|fill| {
						fill.child("patternFill")
							.filter(|pattern| {
								pattern.attribute("patternType")
									== Some("solid")
							})
							.and_then(|pattern| pattern.child("fgColor"))
							.and_then(Self::color)
					})
					.collect::<Vec<_>>()
			})
			.unwrap_or_default();
		let index = |format: &BsxElement, key: &str| {
			format
				.attribute(key)
				.and_then(|id| id.parse::<usize>().ok())
				.unwrap_or(0)
		};
		root.child("cellXfs")
			.map(|formats| {
				formats
					.children_named("xf")
					.map(|format| CellFormat {
						font: fonts
							.get(index(format, "fontId"))
							.cloned()
							.unwrap_or_default(),
						fill: fills
							.get(index(format, "fillId"))
							.cloned()
							.flatten(),
						align: format
							.child("alignment")
							.and_then(|alignment| {
								alignment.attribute("horizontal")
							})
							.and_then(|horizontal| match horizontal {
								"left" => Some(TextAlign::Left),
								"center" | "centerContinuous" => {
									Some(TextAlign::Center)
								}
								"right" => Some(TextAlign::Right),
								_ => None,
							}),
						unlocked: format
							.child("protection")
							.and_then(|protection| {
								protection.attribute("locked")
							})
							.is_some_and(|locked| {
								matches!(locked, "0" | "false")
							}),
					})
					.collect()
			})
			.unwrap_or_default()
	}

	/// A font element's look.
	fn font(font: &BsxElement) -> FontRead {
		let flag = |local: &str| {
			font.child(local).is_some_and(|element| {
				!matches!(
					element.attribute("val"),
					Some("0" | "false" | "none")
				)
			})
		};
		FontRead {
			bold: flag("b"),
			italic: flag("i"),
			underline: flag("u"),
			strike: flag("strike"),
			size: font
				.child("sz")
				.and_then(|size| size.attribute("val"))
				.and_then(|size| size.parse().ok()),
			color: font.child("color").and_then(Self::color),
		}
	}

	/// A SpreadsheetML colour: its own value, a theme slot tinted, or an
	/// entry of the legacy indexed palette; `auto` and the system colours
	/// none.
	fn color(color: &BsxElement) -> Option<OfficeColor> {
		if let Some(rgb) = color.attribute("rgb") {
			return OfficeColor::parse_hex(rgb).map(OfficeColor::Rgb);
		}
		if let Some(theme) = color.attribute("theme") {
			return ThemeSlot::from_spreadsheet_index(theme.parse().ok()?).map(
				|slot| OfficeColor::Slot {
					slot,
					tint: color
						.attribute("tint")
						.and_then(|tint| tint.parse().ok())
						.unwrap_or(0.),
				},
			);
		}
		color
			.attribute("indexed")
			.and_then(|indexed| indexed.parse::<usize>().ok())
			.and_then(|indexed| Self::INDEXED_COLORS.get(indexed))
			.and_then(|hex| OfficeColor::parse_hex(hex))
			.map(OfficeColor::Rgb)
	}

	/// The legacy 64 colour palette an `indexed` colour names; 64 and 65,
	/// the system's text and window colours, name none.
	const INDEXED_COLORS: [&str; 64] = [
		"000000", "FFFFFF", "FF0000", "00FF00", "0000FF", "FFFF00", "FF00FF",
		"00FFFF", "000000", "FFFFFF", "FF0000", "00FF00", "0000FF", "FFFF00",
		"FF00FF", "00FFFF", "800000", "008000", "000080", "808000", "800080",
		"008080", "C0C0C0", "808080", "9999FF", "993366", "FFFFCC", "CCFFFF",
		"660066", "FF8080", "0066CC", "CCCCFF", "000080", "FF00FF", "FFFF00",
		"00FFFF", "800080", "800000", "008080", "0000FF", "00CCFF", "CCFFFF",
		"CCFFCC", "FFFF99", "99CCFF", "FF99CC", "CC99FF", "FFCC99", "3366FF",
		"33CCCC", "99CC00", "FFCC00", "FF9900", "FF6600", "666699", "969696",
		"003366", "339966", "003300", "333300", "993300", "993366", "333399",
		"333333",
	];

	/// Reads one worksheet, refusing a row number, cell reference or merged
	/// range that is present but malformed: rows and columns count from 1,
	/// and a row or cell written without one follows the one before.
	fn read_sheet(
		package: &OpcPackage,
		path: &str,
		name: SmolStr,
		hidden: bool,
		root: &BsxElement,
		shared: &WorkbookRead,
	) -> Result<SheetRead> {
		let mut sheet = SheetRead {
			name,
			hidden,
			protected: root.child("sheetProtection").is_some_and(
				|protection| {
					matches!(protection.attribute("sheet"), Some("1" | "true"))
				},
			),
			columns: Vec::new(),
			rows: HashMap::default(),
			cells: HashMap::default(),
			merges: Vec::new(),
			links: Vec::new(),
		};
		let number = |element: &BsxElement, key: &str| {
			element
				.attribute(key)
				.and_then(|value| value.parse::<u32>().ok())
		};
		let a1 = |sheet: &SheetRead, reference: &str| {
			A1Reference::parse(reference)
				.map(|reference| reference.0)
				.map_err(|err| bevyhow!("sheet `{}`: {err}", sheet.name))
		};
		for column in root
			.child("cols")
			.into_iter()
			.flat_map(|cols| cols.children_named("col"))
		{
			// `min` and `max` count from 1
			if let (Some(first), Some(last)) = (
				number(column, "min").and_then(|min| min.checked_sub(1)),
				number(column, "max").and_then(|max| max.checked_sub(1)),
			) {
				sheet.columns.push((
					first,
					last,
					number(column, "style").unwrap_or(0),
				));
			}
		}
		// shared formulas by their index: the master's position and text
		let mut masters = HashMap::<SmolStr, (UVec2, String)>::default();
		// an unnumbered row or cell follows the one before it
		let mut next_row = 0;
		for row in root
			.child("sheetData")
			.into_iter()
			.flat_map(|data| data.children_named("row"))
		{
			let row_index = match row.attribute("r") {
				Some(number) => number
					.parse::<u32>()
					.ok()
					.and_then(|number| number.checked_sub(1))
					.ok_or_else(|| {
						bevyhow!(
							"sheet `{}`: row `{number}` is not a row number, which counts from 1",
							sheet.name
						)
					})?,
				None => next_row,
			};
			next_row = row_index + 1;
			if matches!(row.attribute("customFormat"), Some("1" | "true")) {
				sheet.rows.insert(row_index, number(row, "s").unwrap_or(0));
			}
			let mut next_column = 0;
			for cell in row.children_named("c") {
				let column = match cell.attribute("r") {
					Some(reference) => a1(&sheet, reference)?.x,
					None => next_column,
				};
				next_column = column + 1;
				let value = cell.child("v").map(|value| value.text());
				let value = match cell.attribute("t") {
					Some("s") => value.map(|index| {
						index
							.trim()
							.parse::<usize>()
							.ok()
							.and_then(|index| shared.strings.get(index))
							.cloned()
							.unwrap_or_default()
					}),
					Some("b") => value.map(|value| match value.trim() {
						"1" | "true" => "TRUE".into(),
						_ => "FALSE".into(),
					}),
					Some("inlineStr") => cell.child("is").map(Self::rich_text),
					_ => value,
				};
				let slot = UVec2::new(column, row_index);
				let formula = cell.child("f").and_then(|formula| {
					let text = formula.text();
					match formula.attribute("t") {
						Some("shared") => {
							let index = SmolStr::new(formula.attribute("si")?);
							match text.trim().is_empty() {
								// a dependent, the master's offset to it
								true => {
									masters.get(&index).map(|(master, text)| {
										FormulaOffset::apply(
											text,
											slot.as_ivec2() - master.as_ivec2(),
										)
									})
								}
								false => {
									masters.insert(index, (slot, text.clone()));
									Some(text)
								}
							}
						}
						_ => (!text.trim().is_empty()).then_some(text),
					}
				});
				sheet.cells.insert(slot, CellRead {
					value,
					formula,
					format: number(cell, "s").unwrap_or(0),
				});
			}
		}
		for merge in root
			.child("mergeCells")
			.into_iter()
			.flat_map(|merges| merges.children_named("mergeCell"))
		{
			let range = merge.attribute("ref").unwrap_or("");
			let Some((first, last)) = range.split_once(':') else {
				bevybail!(
					"sheet `{}`: `{range}` is not a merged range",
					sheet.name
				);
			};
			// a range written last corner first is the same range
			sheet.merges.push(URect::from_corners(
				a1(&sheet, first)?,
				a1(&sheet, last)?,
			));
		}
		for link in root
			.child("hyperlinks")
			.into_iter()
			.flat_map(|links| links.children_named("hyperlink"))
		{
			let Some(range) = link.attribute("ref") else {
				continue;
			};
			let (first, last) = range.split_once(':').unwrap_or((range, range));
			let target = link
				.prefixed_attribute("id")
				.and_then(|id| package.relationship(Some(path), id))
				.map(|relationship| relationship.target.to_string());
			let href = match (target, link.attribute("location")) {
				(Some(target), Some(location)) => {
					format!("{target}#{location}")
				}
				(Some(target), None) => target,
				(None, Some(location)) => format!("#{location}"),
				(None, None) => continue,
			};
			let range =
				URect::from_corners(a1(&sheet, first)?, a1(&sheet, last)?);
			sheet.links.push((range, href.into()));
		}
		sheet.xok()
	}
}

impl SheetRead {
	/// The columns and rows the sheet pads to: through its last used cell,
	/// one holding a value or a formula, a merged range's last slot, or an
	/// unlocked cell of a protected sheet.
	fn extent(&self, shared: &WorkbookRead) -> UVec2 {
		self.cells
			.iter()
			.filter(|(_, cell)| {
				cell.value.is_some()
					|| cell.formula.is_some()
					|| (self.protected && shared.format(cell.format).unlocked)
			})
			.map(|(slot, _)| *slot)
			.chain(self.merges.iter().map(|merge| merge.max))
			.fold(UVec2::ZERO, |size, last| size.max(last + UVec2::ONE))
	}

	/// The article of this sheet: its table of `size` cells, a merged
	/// range's covered cells absent.
	fn article(&self, shared: &WorkbookRead, size: UVec2) -> ReadElement {
		let mut body = ReadElement::new("tbody");
		for row in 0..size.y {
			let mut tr = ReadElement::new("tr");
			for slot in (0..size.x).map(|column| UVec2::new(column, row)) {
				if self.is_covered(slot) {
					continue;
				}
				tr = tr.with_child(self.cell(shared, slot));
			}
			body = body.with_child(tr);
		}
		let mut article = ReadElement::new("article")
			.with_attribute("data-worksheet", self.name.to_string());
		if self.hidden {
			article = article.with_flag("hidden");
		}
		article.with_child(ReadElement::new("table").with_child(body))
	}

	/// The `td` at `slot`.
	fn cell(&self, shared: &WorkbookRead, slot: UVec2) -> ReadElement {
		let read = self.cells.get(&slot);
		let format = read
			.map(|cell| cell.format)
			.unwrap_or_else(|| self.inherited_format(slot));
		let mut td = ReadElement::new("td");
		if let Some(span) = self.span(slot) {
			if span.x > 1 {
				td = td.with_attribute("colspan", span.x.to_string());
			}
			if span.y > 1 {
				td = td.with_attribute("rowspan", span.y.to_string());
			}
		}
		if self.protected && !shared.format(format).unlocked {
			td = td.with_attribute("contenteditable", "false");
		}
		if let Some(formula) = read.and_then(|cell| cell.formula.as_ref()) {
			td = td.with_attribute("data-formula", format!("={formula}"));
		}
		let text = read.and_then(|cell| cell.value.clone()).unwrap_or_default();
		td = match self.link(slot) {
			Some(href) if !text.is_empty() => td.with_child(
				ReadElement::new("a")
					.with_attribute("href", href.to_string())
					.with_text(text),
			),
			_ => td.with_text(text),
		};
		td.with_look(shared.look(format))
	}

	/// The format a cell the sheet does not write is drawn with: its row's
	/// when the row carries one, else its column's.
	fn inherited_format(&self, slot: UVec2) -> u32 {
		self.rows.get(&slot.y).copied().unwrap_or_else(|| {
			self.columns
				.iter()
				.find(|(first, last, _)| (*first..=*last).contains(&slot.x))
				.map(|(.., format)| *format)
				.unwrap_or(0)
		})
	}

	/// The columns and rows of the merged range starting at `slot`.
	fn span(&self, slot: UVec2) -> Option<UVec2> {
		self.merges
			.iter()
			.find(|merge| merge.min == slot)
			.map(|merge| merge.size() + UVec2::ONE)
	}

	/// Whether a merged range covers `slot` other than as its first.
	fn is_covered(&self, slot: UVec2) -> bool {
		self.merges
			.iter()
			.any(|merge| merge.min != slot && merge.contains(slot))
	}

	/// The `href` of the hyperlink over `slot`.
	fn link(&self, slot: UVec2) -> Option<&SmolStr> {
		self.links
			.iter()
			.find(|(range, _)| range.contains(slot))
			.map(|(_, href)| href)
	}
}

impl WorkbookRead {
	/// The format at `index`, the default where the styles hold none.
	fn format(&self, index: u32) -> CellFormat {
		self.formats
			.get(index as usize)
			.cloned()
			.unwrap_or_default()
	}

	/// What the format at `index` says differently from the default format,
	/// the first: a written workbook's defaults are its root's look, never
	/// a cell's own.
	fn look(&self, index: u32) -> DirectLook {
		let (format, base) = (self.format(index), self.format(0));
		let (font, base_font) = (&format.font, &base.font);
		let flag = |set: bool, base: bool| (set != base).then_some(set);
		DirectLook {
			color: (font.color != base_font.color)
				.then_some(font.color)
				.flatten(),
			background: (format.fill != base.fill)
				.then_some(format.fill)
				.flatten(),
			font_size_pt: (font.size != base_font.size)
				.then_some(font.size)
				.flatten(),
			bold: flag(font.bold, base_font.bold),
			italic: flag(font.italic, base_font.italic),
			underline: flag(font.underline, base_font.underline),
			strike: flag(font.strike, base_font.strike),
			// left is where text sits unaligned
			text_align: (format.align != base.align)
				.then_some(format.align)
				.flatten()
				.filter(|align| *align != TextAlign::Left),
		}
	}
}

/// A shared formula's text offset to a dependent cell: every relative A1
/// reference moved by the dependent's distance from the master, an absolute
/// `$` part kept, and string literals and quoted sheet names left alone.
struct FormulaOffset;

impl FormulaOffset {
	/// `formula` with its relative references moved `offset.x` columns
	/// right and `offset.y` rows down.
	fn apply(formula: &str, offset: IVec2) -> String {
		let chars = formula.chars().collect::<Vec<_>>();
		let mut out = String::with_capacity(formula.len());
		let mut index = 0;
		while index < chars.len() {
			let char = chars[index];
			// a string literal or a quoted sheet name, its doubled quote an
			// escape
			if char == '"' || char == '\'' {
				let end = Self::quoted_end(&chars, index);
				out.extend(&chars[index..end]);
				index = end;
				continue;
			}
			let follows_name = index > 0
				&& (chars[index - 1].is_ascii_alphanumeric()
					|| matches!(chars[index - 1], '_' | '.'));
			if !follows_name
				&& let Some((end, moved)) =
					Self::reference(&chars, index, offset)
			{
				out.push_str(&moved);
				index = end;
				continue;
			}
			// the rest of a name, so a reference is never read inside one
			if char.is_ascii_alphanumeric() || char == '_' {
				let end = (index..chars.len())
					.find(|end| {
						!(chars[*end].is_ascii_alphanumeric()
							|| matches!(chars[*end], '_' | '.'))
					})
					.unwrap_or(chars.len());
				out.extend(&chars[index..end]);
				index = end;
				continue;
			}
			out.push(char);
			index += 1;
		}
		out
	}

	/// The index past the quoted run starting at `start`.
	fn quoted_end(chars: &[char], start: usize) -> usize {
		let quote = chars[start];
		let mut index = start + 1;
		while index < chars.len() {
			if chars[index] == quote {
				if chars.get(index + 1) == Some(&quote) {
					index += 2;
					continue;
				}
				return index + 1;
			}
			index += 1;
		}
		chars.len()
	}

	/// The A1 reference starting at `start`, `$`s included, and its moved
	/// text, or `None` where none starts or the name is a function's.
	fn reference(
		chars: &[char],
		start: usize,
		offset: IVec2,
	) -> Option<(usize, String)> {
		let mut index = start;
		let column_fixed = chars.get(index) == Some(&'$');
		index += column_fixed as usize;
		let letters_start = index;
		while chars.get(index).is_some_and(char::is_ascii_uppercase) {
			index += 1;
		}
		let letters = chars[letters_start..index].iter().collect::<String>();
		if letters.is_empty() || letters.len() > 3 {
			return None;
		}
		let row_fixed = chars.get(index) == Some(&'$');
		index += row_fixed as usize;
		let digits_start = index;
		while chars.get(index).is_some_and(char::is_ascii_digit) {
			index += 1;
		}
		let digits = chars[digits_start..index].iter().collect::<String>();
		// a function's name, ie `LOG10(`, or a longer name, is no reference
		if digits.is_empty()
			|| chars.get(index).is_some_and(|next| {
				*next == '(' || next.is_ascii_alphanumeric() || *next == '_'
			}) {
			return None;
		}
		let slot = A1Reference::parse(&format!("{letters}{digits}")).ok()?.0;
		let moved = slot.as_ivec2()
			+ IVec2::new(
				(!column_fixed) as i32 * offset.x,
				(!row_fixed) as i32 * offset.y,
			);
		// a reference moved off the grid is left as written
		let moved = UVec2::try_from(moved)
			.ok()
			.filter(|moved| moved.cmple(A1Reference::MAX).all())?;
		Some((
			index,
			format!(
				"{}{}{}{}",
				if column_fixed { "$" } else { "" },
				A1Reference::column_letters(moved.x),
				if row_fixed { "$" } else { "" },
				// the row as A1 counts it, from 1
				moved.y + 1,
			),
		))
	}
}

impl Installer for WorkbookParser {
	fn install(self, app: &mut App) { DocParsers::register_on(app, self); }
}

impl DocParser for WorkbookParser {
	fn media_types(&self) -> Vec<MediaType> { vec![MediaType::Xlsx] }

	fn parse(
		&mut self,
		bytes: &MediaBytes,
		parsers: &DocParsers,
	) -> Result<ParsedDocument, ParseError> {
		let package = OoxmlPlugin::open(bytes, MediaType::Xlsx)?;
		OoxmlPlugin::read_body(&package, parsers, |world, root| {
			self.read(world, root, &package)
		})
	}
}

#[cfg(test)]
pub(crate) mod test {
	use super::super::office_test::*;
	use super::*;
	use crate::style::*;

	/// A synthetic workbook built over [`OpcPackage`], the one fixture the
	/// reader's and writer's tests share: a styles part whose first font,
	/// fill and cell format are the defaults, shared strings, a theme, core
	/// properties and its sheets in tab order.
	pub(crate) struct WorkbookFixture {
		fonts: Vec<String>,
		fills: Vec<String>,
		formats: Vec<String>,
		strings: Vec<String>,
		properties: Option<String>,
		theme: Option<OfficeTheme>,
		/// Each sheet's name, whether hidden, its inner SpreadsheetML from
		/// `<sheetData>` on, and its hyperlinks' ids and targets.
		sheets: Vec<(String, bool, String, Vec<(String, String)>)>,
	}

	impl Default for WorkbookFixture {
		fn default() -> Self {
			Self {
				fonts: vec![
					"<font><sz val=\"11\"/><color theme=\"1\"/><name val=\"Calibri\"/></font>".into(),
				],
				fills: vec![
					"<fill><patternFill patternType=\"none\"/></fill>".into(),
					"<fill><patternFill patternType=\"gray125\"/></fill>".into(),
				],
				formats: vec!["<xf fontId=\"0\" fillId=\"0\" borderId=\"0\"/>".into()],
				strings: Vec::new(),
				properties: None,
				theme: None,
				sheets: Vec::new(),
			}
		}
	}

	impl WorkbookFixture {
		/// Adds a font, its index the count before it.
		pub fn with_font(mut self, xml: &str) -> Self {
			self.fonts.push(xml.into());
			self
		}

		/// Adds a fill, its index the count before it.
		pub fn with_fill(mut self, xml: &str) -> Self {
			self.fills.push(xml.into());
			self
		}

		/// Adds a cell format, an `<xf>`, its index the count before it.
		pub fn with_format(mut self, xml: &str) -> Self {
			self.formats.push(xml.into());
			self
		}

		/// Sets the shared strings.
		pub fn with_strings(mut self, strings: &[&str]) -> Self {
			self.strings =
				strings.iter().map(|string| string.to_string()).collect();
			self
		}

		/// Sets the core properties' inner elements, the `dc:` prefix bound.
		pub fn with_properties(mut self, xml: &str) -> Self {
			self.properties = Some(xml.into());
			self
		}

		/// Sets the theme part.
		pub fn with_theme(mut self, theme: OfficeTheme) -> Self {
			self.theme = Some(theme);
			self
		}

		/// Adds a sheet of `inner` SpreadsheetML, from `<sheetData>` on.
		pub fn with_sheet(mut self, name: &str, inner: &str) -> Self {
			self.sheets
				.push((name.into(), false, inner.into(), Vec::new()));
			self
		}

		/// Adds a hidden sheet.
		pub fn with_hidden_sheet(mut self, name: &str, inner: &str) -> Self {
			self.sheets
				.push((name.into(), true, inner.into(), Vec::new()));
			self
		}

		/// Relates the last sheet to the external `target` by `id`.
		pub fn with_link(mut self, id: &str, target: &str) -> Self {
			if let Some(sheet) = self.sheets.last_mut() {
				sheet.3.push((id.into(), target.into()));
			}
			self
		}

		/// The workbook's bytes.
		pub fn bytes(self) -> MediaBytes {
			let (main, relationships) =
				(OoxmlNamespace::SPREADSHEET, OoxmlNamespace::RELATIONSHIPS);
			let mut package = OpcPackage::default();
			let workbook = "xl/workbook.xml";
			let mut listed = String::new();
			for (index, (name, hidden, inner, links)) in
				self.sheets.iter().enumerate()
			{
				let path = format!("xl/worksheets/sheet{}.xml", index + 1);
				package.insert(
					path.as_str(),
					"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml",
					format!("<worksheet xmlns=\"{main}\" xmlns:r=\"{relationships}\">{inner}</worksheet>"),
				);
				for (id, target) in links {
					// the fixture names its own ids, so add them in order
					let added = package.relate_external(
						Some(&path),
						OpcRelationship::HYPERLINK,
						target,
					);
					assert_eq!(
						&added, id,
						"fixture link ids are `rId1`, `rId2`, .."
					);
				}
				let id = package.relate(
					Some(workbook),
					WorkbookParser::WORKSHEET,
					&path,
				);
				let state = match hidden {
					true => " state=\"hidden\"",
					false => "",
				};
				listed.push_str(&format!(
					"<sheet name=\"{name}\" sheetId=\"{}\" r:id=\"{id}\"{state}/>",
					index + 1
				));
			}
			package.insert(
				workbook,
				"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml",
				format!(
					"<workbook xmlns=\"{main}\" xmlns:r=\"{relationships}\"><sheets>{listed}</sheets></workbook>"
				),
			);
			package.relate(None, OpcRelationship::OFFICE_DOCUMENT, workbook);
			package.insert(
				"xl/styles.xml",
				"application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml",
				format!(
					"<styleSheet xmlns=\"{main}\"><fonts>{}</fonts><fills>{}</fills>\
					 <borders><border/></borders><cellXfs>{}</cellXfs></styleSheet>",
					self.fonts.concat(),
					self.fills.concat(),
					self.formats.concat(),
				),
			);
			package.relate(
				Some(workbook),
				OpcRelationship::STYLES,
				"xl/styles.xml",
			);
			if !self.strings.is_empty() {
				let items = self
					.strings
					.iter()
					.map(|string| format!("<si><t>{string}</t></si>"))
					.collect::<String>();
				package.insert(
					"xl/sharedStrings.xml",
					"application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml",
					format!("<sst xmlns=\"{main}\">{items}</sst>"),
				);
				package.relate(
					Some(workbook),
					WorkbookParser::SHARED_STRINGS,
					"xl/sharedStrings.xml",
				);
			}
			if let Some(theme) = self.theme {
				package.insert(
					"xl/theme/theme1.xml",
					"application/vnd.openxmlformats-officedocument.theme+xml",
					theme.theme_part(),
				);
				package.relate(
					Some(workbook),
					OpcRelationship::THEME,
					"xl/theme/theme1.xml",
				);
			}
			if let Some(properties) = self.properties {
				package.insert(
					DocMeta::CORE_PROPERTIES_PATH,
					OpcPackage::CORE_PROPERTIES_TYPE,
					format!(
						"<cp:coreProperties xmlns:cp=\"{}\" xmlns:dc=\"{}\">{properties}</cp:coreProperties>",
						OoxmlNamespace::CORE_PROPERTIES,
						OoxmlNamespace::DUBLIN_CORE,
					),
				);
				package.relate(
					None,
					OpcRelationship::CORE_PROPERTIES,
					DocMeta::CORE_PROPERTIES_PATH,
				);
			}
			MediaBytes::new(MediaType::Xlsx, package.to_bytes().unwrap())
		}
	}

	/// The round trip's workbook: a sheet of strings, numbers, `007` as a
	/// string, a boolean, an error, a formula and a shared formula, a merge,
	/// a hyperlink and bold, coloured, filled and aligned cells, then a
	/// hidden protected sheet of a locked label and an unlocked field.
	pub(crate) fn budget() -> MediaBytes {
		WorkbookFixture::default()
			.with_font("<font><b/><sz val=\"11\"/><color theme=\"1\"/></font>")
			.with_font("<font><sz val=\"11\"/><color rgb=\"FFFF0000\"/></font>")
			.with_fill("<fill><patternFill patternType=\"solid\"><fgColor rgb=\"FFFFFF00\"/></patternFill></fill>")
			.with_format("<xf fontId=\"1\" fillId=\"0\"/>")
			.with_format("<xf fontId=\"2\" fillId=\"0\"/>")
			.with_format("<xf fontId=\"0\" fillId=\"2\"><alignment horizontal=\"center\"/></xf>")
			.with_format("<xf fontId=\"0\" fillId=\"0\"><protection locked=\"0\"/></xf>")
			.with_strings(&["Name", "Code", "Total", "007", "Merged", "Site"])
			.with_properties("<dc:title>Budget</dc:title>")
			.with_sheet(
				"Data",
				"<sheetData>\
				 <row r=\"1\"><c r=\"A1\" t=\"s\"><v>0</v></c><c r=\"B1\" t=\"s\"><v>1</v></c><c r=\"C1\" s=\"1\" t=\"s\"><v>2</v></c></row>\
				 <row r=\"2\"><c r=\"A2\" t=\"inlineStr\"><is><t>Acme</t></is></c><c r=\"B2\" t=\"s\"><v>3</v></c><c r=\"C2\"><v>120.5</v></c></row>\
				 <row r=\"3\"><c r=\"A3\" t=\"b\"><v>1</v></c><c r=\"B3\" t=\"e\"><v>#DIV/0!</v></c><c r=\"C3\"><f>C2*2</f><v>241</v></c></row>\
				 <row r=\"4\"><c r=\"A4\" s=\"2\"><f t=\"shared\" ref=\"A4:B4\" si=\"0\">C2+1</f><v>121.5</v></c>\
				 <c r=\"B4\" s=\"3\"><f t=\"shared\" si=\"0\"/><v>1</v></c></row>\
				 <row r=\"5\"><c r=\"A5\" t=\"s\"><v>4</v></c><c r=\"B5\"><v>9</v></c></row>\
				 <row r=\"6\"><c r=\"A6\" t=\"s\"><v>5</v></c></row>\
				 </sheetData>\
				 <mergeCells count=\"1\"><mergeCell ref=\"B5:A5\"/></mergeCells>\
				 <hyperlinks><hyperlink ref=\"A6\" r:id=\"rId1\"/></hyperlinks>",
			)
			.with_link("rId1", "https://example.com/site")
			.with_hidden_sheet(
				"Form",
				"<sheetData><row r=\"1\"><c r=\"A1\" t=\"inlineStr\"><is><t>Label</t></is></c><c r=\"B1\" s=\"4\"/></row></sheetData>\
				 <sheetProtection sheet=\"1\"/>",
			)
			.bytes()
	}

	/// The first table of each article with its cells' positions and texts.
	fn sheets(world: &mut World) -> Vec<(String, UVec2)> {
		let tables = world
			.query::<(Entity, &Element)>()
			.iter(world)
			.filter(|(_, element)| element.tag() == "table")
			.map(|(entity, _)| entity)
			.collect::<Vec<_>>();
		world.with_state::<(TableGrid, ElementQuery, Query<&ChildOf>), _>(
			|(grid, elements, parents)| {
				tables
					.into_iter()
					.map(|table| {
						let article = parents.get(table).unwrap().parent();
						let name = elements
							.get(article)
							.unwrap()
							.attribute("data-worksheet")
							.unwrap()
							.value
							.to_string();
						(name, grid.size(table))
					})
					.collect()
			},
		)
	}

	/// The workbook reads as articles of plain tables: strings and numbers as
	/// written, a shared formula's dependent offset, a merge as a span with
	/// its covered cell absent, a link, a hidden sheet and a locked cell.
	#[beet_core::test]
	fn reads_sheets_as_tables() {
		let (mut world, root) = parse(&budget()).unwrap();
		world
			.entity(root)
			.get::<DocMeta>()
			.unwrap()
			.title
			.clone()
			.xpect_eq(Some("Budget".into()));
		html(&mut world, root)
			.xpect_starts_with("<article data-worksheet=\"Data\"><table><tbody><tr><td>Name</td><td>Code</td><td class=")
			.xpect_contains("<td>Acme</td><td>007</td><td>120.5</td>")
			.xpect_contains("<td>TRUE</td><td>#DIV/0!</td><td data-formula=\"=C2*2\">241</td>")
			.xpect_contains("<td data-formula=\"=C2+1\" class=")
			.xpect_contains("<td data-formula=\"=D2+1\" class=")
			// a merge's covered cell absent, every row padded to the last column
			.xpect_contains("<tr><td colspan=\"2\">Merged</td><td></td></tr>")
			.xpect_contains("<td><a href=\"https://example.com/site\">Site</a></td><td></td><td></td>")
			.xpect_contains("<article data-worksheet=\"Form\" hidden><table><tbody><tr><td contenteditable=\"false\">Label</td><td></td></tr>")
			.xnot()
			.xpect_contains(">9<");
		sheets(&mut world).xpect_eq(vec![
			("Data".to_string(), UVec2::new(3, 6)),
			("Form".into(), UVec2::new(2, 1)),
		]);
	}

	/// The cell looks read as direct looks resolving through the cascade:
	/// bold, a colour, a fill and an alignment.
	#[beet_core::test]
	fn reads_cell_looks() {
		let (mut world, _) = parse(&budget()).unwrap();
		let cell = |world: &mut World, words: &str| {
			let text = world
				.query::<(Entity, &Value)>()
				.iter(world)
				.find(|(_, value)| value.as_str().ok() == Some(words))
				.map(|(entity, _)| entity)
				.unwrap();
			world.entity(text).get::<ChildOf>().unwrap().parent()
		};
		let (total, red, filled) = (
			cell(&mut world, "Total"),
			cell(&mut world, "121.5"),
			cell(&mut world, "1"),
		);
		world.with_state::<CascadeQuery, _>(|cascade| {
			let memo =
				&mut CascadeMemo::for_media(MediaContext::new(Medium::Screen));
			let mut visual = |cell| cascade.resolve(cell, memo).unwrap().visual;
			visual(total).font_weight.is_bold().xpect_true();
			visual(red)
				.foreground
				.xpect_eq(Some(Color::srgb(1., 0., 0.)));
			let filled = visual(filled);
			filled.background.xpect_eq(Some(Color::srgb(1., 1., 0.)));
			filled.text_align.xpect_eq(TextAlign::Center);
		});
	}

	/// A tinted slot reads as the colour it computes to in the file's own
	/// theme, as does every slot read `Verbatim`.
	#[beet_core::test]
	fn computes_colours_against_the_files_theme() {
		let bytes = WorkbookFixture::default()
			.with_font("<font><sz val=\"11\"/><color theme=\"4\" tint=\"0.5\"/></font>")
			.with_font("<font><sz val=\"11\"/><color theme=\"4\"/></font>")
			.with_format("<xf fontId=\"1\" fillId=\"0\"/>")
			.with_format("<xf fontId=\"2\" fillId=\"0\"/>")
			.with_theme(
				OfficeTheme::default()
					.with_color(ThemeSlot::Accent1, Color::srgb(1., 0., 0.)),
			)
			.with_sheet(
				"Sheet",
				"<sheetData><row r=\"1\"><c r=\"A1\" s=\"1\" t=\"inlineStr\"><is><t>tint</t></is></c>\
				 <c r=\"B1\" s=\"2\" t=\"inlineStr\"><is><t>slot</t></is></c></row></sheetData>",
			)
			.bytes();
		let colors = |options: OfficeReadOptions| {
			let mut world =
				(TemplatePlugin, DocumentPlugin, OoxmlPlugin).into_world();
			world
				.resource_mut::<DocParsers>()
				.register(WorkbookParser::default().with_options(options));
			let (mut world, _) = parse_in(world, &bytes).unwrap();
			let cells = world
				.query::<(Entity, &Element)>()
				.iter(&world)
				.filter(|(_, element)| element.tag() == "td")
				.map(|(entity, _)| entity)
				.collect::<Vec<_>>();
			world.with_state::<CascadeQuery, _>(|cascade| {
				let memo = &mut CascadeMemo::for_media(MediaContext::new(
					Medium::Screen,
				));
				cells
					.iter()
					.map(|cell| {
						cascade
							.resolve(*cell, memo)
							.unwrap()
							.visual
							.foreground
							.map(OfficeColor::hex)
					})
					.collect::<Vec<_>>()
			})
		};
		// the tint against the file's red, the bare slot a token no theme
		// resolves here
		colors(OfficeReadOptions::default())
			.xpect_eq(vec![Some("FF8080".to_string()), None]);
		colors(
			OfficeReadOptions::default()
				.with_theme_colors(ThemeColors::Verbatim),
		)
		.xpect_eq(vec![Some("FF8080".to_string()), Some("FF0000".into())]);
	}

	/// A row or cell written without its number follows the one before, and
	/// one malformed, a row numbered 0 included, refuses the read.
	#[beet_core::test]
	fn numbers_rows_and_cells_from_one() {
		let read = |data: &str| {
			parse(
				&WorkbookFixture::default()
					.with_sheet(
						"Sheet",
						&format!("<sheetData>{data}</sheetData>"),
					)
					.bytes(),
			)
		};
		let (mut world, _) =
			read("<row r=\"2\"><c r=\"B2\"><v>1</v></c><c><v>2</v></c></row><row><c><v>3</v></c></row>")
				.unwrap();
		sheets(&mut world)
			.xpect_eq(vec![("Sheet".to_string(), UVec2::new(3, 3))]);
		read("<row r=\"0\"><c/></row>")
			.unwrap_err()
			.to_string()
			.xpect_contains("row `0` is not a row number");
		read("<row r=\"1\"><c r=\"A0\"/></row>")
			.unwrap_err()
			.to_string()
			.xpect_contains("sheet `Sheet`: `A0` is not an A1 reference");
	}

	/// A stray value pads the sheet to it, a style alone pads nothing, a
	/// value at the last cell a sheet holds is refused naming the sheet and
	/// the cell, and so are sheets padding a workbook past its bound.
	#[beet_core::test]
	fn pads_to_the_last_used_cell() {
		let sheet = |cells: &str| {
			format!(
				"<sheetData><row r=\"1\"><c r=\"A1\"><v>1</v></c></row>{cells}</sheetData>"
			)
		};
		let (mut world, _) = parse(
			&WorkbookFixture::default()
				.with_sheet(
					"Stray",
					&sheet("<row r=\"99\"><c r=\"Z99\"><v>2</v></c></row>"),
				)
				.with_sheet(
					"Styled",
					&sheet(
						"<row r=\"1048576\"><c r=\"XFD1048576\" s=\"0\"/></row>",
					),
				)
				.bytes(),
		)
		.unwrap();
		sheets(&mut world).xpect_eq(vec![
			("Stray".to_string(), UVec2::new(26, 99)),
			("Styled".into(), UVec2::new(1, 1)),
		]);
		parse(
			&WorkbookFixture::default()
				.with_sheet(
					"Huge",
					&sheet(
						"<row r=\"1048576\"><c r=\"XFD1048576\"><v>1</v></c></row>",
					),
				)
				.bytes(),
		)
		.unwrap_err()
		.to_string()
		.xpect_contains("sheet `Huge`: its last used cell `XFD1048576`");
		// eleven sheets each at the sheet bound pass it, but not the workbook's
		let full = sheet("<row r=\"1000\"><c r=\"CV1000\"><v>1</v></c></row>");
		(0..11)
			.fold(WorkbookFixture::default(), |fixture, index| {
				fixture.with_sheet(&format!("S{index}"), &full)
			})
			.bytes()
			.xref()
			.xmap(parse)
			.unwrap_err()
			.to_string()
			.xpect_contains("sheet `S10`")
			.xpect_contains("past the 1000000 a workbook may hold");
	}

	#[beet_core::test]
	fn offsets_shared_formulas() {
		FormulaOffset::apply("A1*2+$B$1+C$2+$D3", IVec2::new(1, 2))
			.xpect_eq("B3*2+$B$1+D$2+$D5");
		// literals, quoted sheet names and function names stay
		FormulaOffset::apply(
			"IF(A1>0,\"A1\",'Sheet A1'!A1)&LOG10(B2)",
			IVec2::new(0, 1),
		)
		.xpect_eq("IF(A2>0,\"A1\",'Sheet A1'!A2)&LOG10(B3)");
		FormulaOffset::apply("SUM(Sheet2!A1:A3)", IVec2::new(2, 0))
			.xpect_eq("SUM(Sheet2!C1:C3)");
	}
}
