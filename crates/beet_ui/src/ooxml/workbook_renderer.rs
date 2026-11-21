//! [`WorkbookRenderer`], any document holding tables written as a workbook,
//! by the [module docs](super)' workbook mapping.
use super::office_style::*;
use super::xml_part::XmlPart;
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::RequestParts;

/// Writes any document holding tables as a workbook, from the tree alone:
/// every table not nested in another a sheet, in document order, named by
/// its enclosing `article data-worksheet`, else its `caption`, else
/// `Sheet<n>`. A cell's text is typed as a person typing it would, a
/// `data-formula` writes as a formula, a locked cell protects its sheet,
/// and each distinct look the cascade resolves for a cell in the `Screen`
/// medium writes as one cell format. A document holding no table is
/// refused. See the [module docs](super) for what a write can say.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct WorkbookRenderer;

/// One sheet as the writer gathers it from a table.
struct SheetWrite {
	/// Its valid, unique tab name.
	name: String,
	/// Whether its enclosing article is `hidden`.
	hidden: bool,
	/// Every cell with its place in the grid.
	cells: Vec<CellWrite>,
}

/// One cell as the writer gathers it.
struct CellWrite {
	/// Its place in the grid, its entity the one whose look the cascade
	/// resolves.
	place: GridCell,
	/// Its text.
	text: String,
	/// Its formula without the `=`.
	formula: Option<String>,
	/// Whether a person may not type into it.
	locked: bool,
	/// The `href` of the first link in it.
	href: Option<String>,
}

/// A cell format being written: the look and protection a `cellXfs` entry
/// says.
#[derive(Debug, Clone, PartialEq)]
struct FormatWrite {
	look: OfficeLook,
	unlocked: bool,
}

/// A cell's value as typed.
enum Typed<'a> {
	Number(&'a str),
	Boolean(bool),
	Error(&'a str),
	Text(&'a str),
}

impl WorkbookRenderer {
	const WORKBOOK_TYPE: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml";
	const WORKSHEET_TYPE: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml";
	const STYLES_TYPE: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml";
	const SHARED_STRINGS_TYPE: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml";
	const THEME_TYPE: &str =
		"application/vnd.openxmlformats-officedocument.theme+xml";
	const WORKSHEET: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet";
	const SHARED_STRINGS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings";
	/// The error codes a cell's text types as an error.
	const ERRORS: [&str; 8] = [
		"#NULL!",
		"#DIV/0!",
		"#VALUE!",
		"#REF!",
		"#NAME?",
		"#NUM!",
		"#N/A",
		"#GETTING_DATA",
	];

	/// Writes `root` as a workbook's bytes.
	fn write(
		world: &mut World,
		root: Entity,
		request: &RequestParts,
	) -> Result<Vec<u8>> {
		let mut media = MediaContext::new(Medium::Screen);
		if let Some(scheme) = OfficeStyleSheet::scheme(world, request) {
			media = media.with_scheme(scheme);
		}
		let styles = OfficeStyleSheet::resolve(world, root, media)?;
		let sheets = Self::gather(world, root)?;
		// every cell's look, what it resolves differently from the root's
		let entities = sheets
			.iter()
			.flat_map(|sheet| sheet.cells.iter().map(|cell| cell.place.entity))
			.collect::<Vec<_>>();
		let mut looks = styles.direct(world, &entities, None)?.into_iter();
		let mut formats = vec![FormatWrite {
			look: OfficeLook::default(),
			unlocked: false,
		}];
		let mut strings = Vec::<String>::new();
		let mut package = OpcPackage::default();
		let workbook = "xl/workbook.xml";
		let mut listed = XmlPart::fragment();
		// a workbook shows at least one sheet
		let first_visible =
			sheets.iter().position(|sheet| !sheet.hidden).unwrap_or(0);
		for (index, sheet) in sheets.iter().enumerate() {
			let protected = sheet.cells.iter().any(|cell| cell.locked);
			let cell_formats = sheet
				.cells
				.iter()
				.map(|cell| {
					let format = FormatWrite {
						look: looks.next().unwrap_or_default(),
						unlocked: protected && !cell.locked,
					};
					match formats.iter().position(|found| *found == format) {
						Some(index) => index,
						None => {
							formats.push(format);
							formats.len() - 1
						}
					}
				})
				.collect::<Vec<_>>();
			let path = format!("xl/worksheets/sheet{}.xml", index + 1);
			let xml = Self::sheet_xml(
				&mut package,
				&path,
				sheet,
				&cell_formats,
				protected,
				&mut strings,
			);
			package.insert(path.as_str(), Self::WORKSHEET_TYPE, xml);
			let id = package.relate(Some(workbook), Self::WORKSHEET, &path);
			let sheet_id = (index + 1).to_string();
			let mut attributes = vec![
				("name", sheet.name.as_str()),
				("sheetId", sheet_id.as_str()),
				("r:id", id.as_str()),
			];
			if sheet.hidden && index != first_visible {
				attributes.push(("state", "hidden"));
			}
			listed.empty("sheet", &attributes);
		}
		let mut xml = XmlPart::new();
		xml.open("workbook", &[
			("xmlns", OoxmlNamespace::SPREADSHEET),
			("xmlns:r", OoxmlNamespace::RELATIONSHIPS),
		])
		.open("sheets", &[])
		.raw(listed.as_str())
		.close("sheets")
		// nothing here computes, so the workbook recalculates on open
		.empty("calcPr", &[("calcId", "191029"), ("fullCalcOnLoad", "1")])
		.close("workbook");
		package.insert(workbook, Self::WORKBOOK_TYPE, xml.into_bytes());
		package.relate(None, OpcRelationship::OFFICE_DOCUMENT, workbook);
		package.insert(
			"xl/styles.xml",
			Self::STYLES_TYPE,
			Self::styles_xml(&styles, &formats),
		);
		package.relate(
			Some(workbook),
			OpcRelationship::STYLES,
			"xl/styles.xml",
		);
		package.insert(
			"xl/sharedStrings.xml",
			Self::SHARED_STRINGS_TYPE,
			Self::shared_strings_xml(&strings),
		);
		package.relate(
			Some(workbook),
			Self::SHARED_STRINGS,
			"xl/sharedStrings.xml",
		);
		package.insert(
			"xl/theme/theme1.xml",
			Self::THEME_TYPE,
			styles.palette().theme_part(),
		);
		package.relate(
			Some(workbook),
			OpcRelationship::THEME,
			"xl/theme/theme1.xml",
		);
		if let Some(meta) = world.entity(root).get::<DocMeta>() {
			meta.write_package(&mut package);
		}
		package.to_bytes()
	}

	/// The sheets of `root`: every table not nested in another, in document
	/// order, refusing a document holding none.
	fn gather(world: &mut World, root: Entity) -> Result<Vec<SheetWrite>> {
		let tables = world
			.with_state::<(Query<&Children>, Query<&Element>, Query<&ChildOf>), _>(
				|(children, elements, parents)| {
					let is_table = |entity: Entity| {
						elements
							.get(entity)
							.is_ok_and(|element| element.tag() == "table")
					};
					children
						.iter_descendants_depth_first(root)
						.filter(|entity| is_table(*entity))
						.filter(|table| {
							!parents
								.iter_ancestors(*table)
								.take_while(|ancestor| *ancestor != root)
								.any(is_table)
						})
						.collect::<Vec<_>>()
				},
			);
		if tables.is_empty() {
			bevybail!("the document holds no table, so it writes no workbook");
		}
		let mut sheets = Vec::<SheetWrite>::new();
		for (index, table) in tables.into_iter().enumerate() {
			let sheet =
				world.with_state::<(
					TableGrid,
					DocumentText,
					ElementQuery,
					Query<&ChildOf>,
					Query<&Children>,
				), _>(|(grid, text, elements, parents, children)| {
					// the enclosing worksheet article, its name and whether it is hidden
					let article =
						parents.iter_ancestors(table).find_map(|ancestor| {
							let view = elements.get(ancestor).ok()?;
							let name = view.attribute("data-worksheet")?;
							Some((
								name.value.to_string(),
								view.attribute("hidden").is_some(),
							))
						});
					let caption = children
						.get(table)
						.into_iter()
						.flatten()
						.find(|child| {
							elements
								.get(**child)
								.is_ok_and(|view| view.tag() == "caption")
						})
						.map(|caption| text.text(*caption).trim().to_string())
						.filter(|caption| !caption.is_empty());
					let cells = grid
						.cells(table)
						.into_iter()
						.map(|place| {
							let cell = place.entity;
							let view = elements.get(cell).ok();
							let formula = view
								.as_ref()
								.and_then(|view| view.attribute("data-formula"))
								.map(|formula| {
									let formula = formula.value.to_string();
									formula
										.strip_prefix('=')
										.unwrap_or(&formula)
										.to_string()
								})
								.filter(|formula| !formula.trim().is_empty());
							let href = children
								.iter_descendants(cell)
								.find_map(|child| {
									let view = elements.get(child).ok()?;
									(view.tag() == "a")
										.then(|| {
											view.attribute("href").map(|href| {
												href.value.to_string()
											})
										})
										.flatten()
								});
							CellWrite {
								place,
								text: text
									.text(cell)
									.trim_end_matches('\n')
									.to_string(),
								formula,
								locked: grid.is_locked(cell),
								href,
							}
						})
						.collect::<Vec<_>>();
					let (name, hidden) = match article {
						Some((name, hidden)) => (Some(name), hidden),
						None => (caption, false),
					};
					(name, hidden, cells)
				});
			let (name, hidden, cells) = sheet;
			let name = Self::sheet_name(
				name.as_deref(),
				index,
				sheets.iter().map(|sheet| sheet.name.as_str()),
			);
			sheets.push(SheetWrite {
				name,
				hidden,
				cells,
			});
		}
		sheets.xok()
	}

	/// A valid tab name for the sheet at `index`: `name` without `[]:*?/\`,
	/// at most 31 characters, else `Sheet<n>`, made unique among `taken`.
	fn sheet_name<'a>(
		name: Option<&str>,
		index: usize,
		taken: impl Iterator<Item = &'a str>,
	) -> String {
		let taken = taken.map(str::to_lowercase).collect::<Vec<_>>();
		let base = name
			.map(|name| {
				name.chars()
					.filter(|char| !"[]:*?/\\".contains(*char))
					.collect::<String>()
					.trim()
					.trim_matches('\'')
					.to_string()
			})
			.filter(|name| !name.is_empty())
			.unwrap_or_else(|| format!("Sheet{}", index + 1));
		let fit = |name: &str, suffix: &str| {
			let room = 31 - suffix.chars().count();
			name.chars().take(room).collect::<String>() + suffix
		};
		(1..)
			.map(|count| match count {
				1 => fit(&base, ""),
				count => fit(&base, &format!(" ({count})")),
			})
			.find(|name| !taken.contains(&name.to_lowercase()))
			.unwrap_or(base)
	}

	/// How a person typing `text` into a cell has it read: a number in its
	/// canonical decimal form, `TRUE` or `FALSE`, an error code, else text.
	fn typed(text: &str) -> Typed<'_> {
		match text {
			"TRUE" => Typed::Boolean(true),
			"FALSE" => Typed::Boolean(false),
			text if Self::ERRORS.contains(&text) => Typed::Error(text),
			text if Self::is_canonical_number(text) => Typed::Number(text),
			text => Typed::Text(text),
		}
	}

	/// Whether `text` is a number written as its canonical decimal form: an
	/// optional minus, no leading zero but a lone one, and a fraction with
	/// no trailing zero, so `007` and `1.0` stay text.
	fn is_canonical_number(text: &str) -> bool {
		let unsigned = text.strip_prefix('-').unwrap_or(text);
		let (whole, fraction) = match unsigned.split_once('.') {
			Some((whole, fraction)) => (whole, Some(fraction)),
			None => (unsigned, None),
		};
		let digits = |part: &str| {
			!part.is_empty() && part.chars().all(|char| char.is_ascii_digit())
		};
		digits(whole)
			&& (whole == "0" || !whole.starts_with('0'))
			&& fraction.is_none_or(|fraction| {
				digits(fraction) && !fraction.ends_with('0')
			}) && !(text.starts_with('-') && whole == "0" && fraction.is_none())
	}

	/// One worksheet, its cells written by `formats`' indices, its strings
	/// shared through `strings`.
	fn sheet_xml(
		package: &mut OpcPackage,
		path: &str,
		sheet: &SheetWrite,
		formats: &[usize],
		protected: bool,
		strings: &mut Vec<String>,
	) -> Vec<u8> {
		let mut rows = std::collections::BTreeMap::<u32, XmlPart>::new();
		let mut merges = Vec::new();
		let mut links = Vec::new();
		for (cell, format) in sheet.cells.iter().zip(formats) {
			let xml = rows
				.entry(cell.place.start.y)
				.or_insert_with(XmlPart::fragment);
			let reference = A1Reference(cell.place.start).to_string();
			let format = format.to_string();
			let mut attributes = vec![("r", reference.as_str())];
			if format != "0" {
				attributes.push(("s", format.as_str()));
			}
			let typed = Self::typed(&cell.text);
			match (&cell.formula, &typed) {
				(None, Typed::Text("")) => {
					xml.empty("c", &attributes);
				}
				(None, Typed::Text(text)) => {
					let index =
						match strings.iter().position(|found| found == text) {
							Some(index) => index,
							None => {
								strings.push(text.to_string());
								strings.len() - 1
							}
						};
					attributes.push(("t", "s"));
					xml.open("c", &attributes)
						.text_element("v", &[], &index.to_string())
						.close("c");
				}
				(formula, typed) => {
					let (kind, value) = match typed {
						Typed::Number(number) => (None, number.to_string()),
						Typed::Boolean(value) => {
							(Some("b"), (*value as u8).to_string())
						}
						Typed::Error(code) => (Some("e"), code.to_string()),
						Typed::Text(text) => (Some("str"), text.to_string()),
					};
					if let Some(kind) = kind {
						attributes.push(("t", kind));
					}
					xml.open("c", &attributes);
					if let Some(formula) = formula {
						xml.text_element("f", &[], formula);
					}
					// a formula's empty text is no cached value
					if formula.is_none() || !cell.text.is_empty() {
						xml.text_element("v", &[], &value);
					}
					xml.close("c");
				}
			}
			if cell.place.span != UVec2::ONE {
				merges.push(format!(
					"{reference}:{}",
					A1Reference(cell.place.last())
				));
			}
			if let Some(href) = &cell.href {
				links.push((reference, href.clone()));
			}
		}
		let mut xml = XmlPart::new();
		xml.open("worksheet", &[
			("xmlns", OoxmlNamespace::SPREADSHEET),
			("xmlns:r", OoxmlNamespace::RELATIONSHIPS),
		])
		.open("sheetData", &[]);
		for (row, cells) in rows {
			// a row's number counts from 1
			xml.open("row", &[("r", &(row as u64 + 1).to_string())])
				.raw(cells.as_str())
				.close("row");
		}
		xml.close("sheetData");
		if protected {
			xml.empty("sheetProtection", &[("sheet", "1")]);
		}
		if !merges.is_empty() {
			xml.open("mergeCells", &[("count", &merges.len().to_string())]);
			for merge in &merges {
				xml.empty("mergeCell", &[("ref", merge)]);
			}
			xml.close("mergeCells");
		}
		if !links.is_empty() {
			xml.open("hyperlinks", &[]);
			for (reference, href) in links {
				match href.strip_prefix('#') {
					Some(location) => {
						xml.empty("hyperlink", &[
							("ref", &reference),
							("location", location),
						]);
					}
					None => {
						let id = package.relate_external(
							Some(path),
							OpcRelationship::HYPERLINK,
							&href,
						);
						xml.empty("hyperlink", &[
							("ref", &reference),
							("r:id", &id),
						]);
					}
				}
			}
			xml.close("hyperlinks");
		}
		xml.close("worksheet");
		xml.into_bytes()
	}

	/// The styles part: a font and fill per distinct look, each cell format
	/// the root's look with its own over it, the first the root's alone.
	fn styles_xml(
		styles: &OfficeStyleSheet,
		formats: &[FormatWrite],
	) -> Vec<u8> {
		let defaults = styles.defaults();
		let mut fonts = Vec::<String>::new();
		// fills 0 and 1 are reserved, none and the gray pattern
		let mut fills = vec![
			"<fill><patternFill patternType=\"none\"/></fill>".to_string(),
			"<fill><patternFill patternType=\"gray125\"/></fill>".to_string(),
		];
		let intern = |list: &mut Vec<String>, xml: String| match list
			.iter()
			.position(|found| *found == xml)
		{
			Some(index) => index,
			None => {
				list.push(xml);
				list.len() - 1
			}
		};
		let mut xfs = XmlPart::fragment();
		for format in formats {
			let look = Self::over(defaults, &format.look);
			let font = intern(&mut fonts, Self::font_xml(styles, &look));
			let fill = match look.background {
				Some(color) => intern(
					&mut fills,
					format!(
						"<fill><patternFill patternType=\"solid\">{}<bgColor indexed=\"64\"/></patternFill></fill>",
						Self::color_xml(styles, "fgColor", color)
					),
				),
				None => 0,
			};
			let (font, fill) = (font.to_string(), fill.to_string());
			let mut attributes = vec![
				("numFmtId", "0"),
				("fontId", font.as_str()),
				("fillId", fill.as_str()),
				("borderId", "0"),
				("xfId", "0"),
			];
			let align = match look.text_align {
				Some(TextAlign::Center) => Some("center"),
				Some(TextAlign::Right) => Some("right"),
				_ => None,
			};
			if align.is_some() {
				attributes.push(("applyAlignment", "1"));
			}
			if format.unlocked {
				attributes.push(("applyProtection", "1"));
			}
			match (align, format.unlocked) {
				(None, false) => {
					xfs.empty("xf", &attributes);
				}
				(align, unlocked) => {
					xfs.open("xf", &attributes);
					if let Some(align) = align {
						xfs.empty("alignment", &[("horizontal", align)]);
					}
					if unlocked {
						xfs.empty("protection", &[("locked", "0")]);
					}
					xfs.close("xf");
				}
			}
		}
		let mut xml = XmlPart::new();
		xml.open("styleSheet", &[("xmlns", OoxmlNamespace::SPREADSHEET)])
			.open("fonts", &[("count", &fonts.len().to_string())])
			.raw(&fonts.concat())
			.close("fonts")
			.open("fills", &[("count", &fills.len().to_string())])
			.raw(&fills.concat())
			.close("fills")
			.raw("<borders count=\"1\"><border><left/><right/><top/><bottom/><diagonal/></border></borders>")
			.raw(&format!(
				"<cellStyleXfs count=\"1\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"{}\" borderId=\"0\"/></cellStyleXfs>",
				match defaults.background {
					Some(_) => 2,
					None => 0,
				}
			))
			.open("cellXfs", &[("count", &formats.len().to_string())])
			.raw(xfs.as_str())
			.close("cellXfs")
			.raw("<cellStyles count=\"1\"><cellStyle name=\"Normal\" xfId=\"0\" builtinId=\"0\"/></cellStyles>")
			.close("styleSheet");
		xml.into_bytes()
	}

	/// `look` laid over `base`, each field `look` says replacing `base`'s.
	fn over(base: &OfficeLook, look: &OfficeLook) -> OfficeLook {
		OfficeLook {
			bold: look.bold.or(base.bold),
			italic: look.italic.or(base.italic),
			underline: look.underline.or(base.underline),
			strike: look.strike.or(base.strike),
			color: look.color.or(base.color),
			background: look.background.or(base.background),
			font_size_px: look.font_size_px.or(base.font_size_px),
			text_align: look.text_align.or(base.text_align),
		}
	}

	/// A font element of `look`.
	fn font_xml(styles: &OfficeStyleSheet, look: &OfficeLook) -> String {
		let mut xml = XmlPart::fragment();
		xml.open("font", &[]);
		for (set, name) in [
			(look.bold, "b"),
			(look.italic, "i"),
			(look.strike, "strike"),
			(look.underline, "u"),
		] {
			if set == Some(true) {
				xml.empty(name, &[]);
			}
		}
		let size = OfficeUnits::pt(
			look.font_size_px.unwrap_or(OfficeUnits::PX_PER_REM),
		);
		let size = format!("{:.2}", size)
			.trim_end_matches('0')
			.trim_end_matches('.')
			.to_string();
		xml.empty("sz", &[("val", &size)]);
		if let Some(color) = look.color {
			xml.raw(&Self::color_xml(styles, "color", color));
		}
		xml.empty("name", &[("val", "Calibri")])
			.empty("family", &[("val", "2")])
			.close("font");
		xml.as_str().to_string()
	}

	/// A colour element `name`: a reference to the first theme slot holding
	/// `color`, else its own value.
	fn color_xml(
		styles: &OfficeStyleSheet,
		name: &str,
		color: Color,
	) -> String {
		let mut xml = XmlPart::fragment();
		match styles.theme_slot(color) {
			Some(slot) => {
				xml.empty(name, &[(
					"theme",
					&slot.spreadsheet_index().to_string(),
				)]);
			}
			None => {
				xml.empty(name, &[(
					"rgb",
					&format!("FF{}", OfficeColor::hex(color)),
				)]);
			}
		}
		xml.as_str().to_string()
	}

	/// The shared strings part.
	fn shared_strings_xml(strings: &[String]) -> Vec<u8> {
		let mut xml = XmlPart::new();
		let count = strings.len().to_string();
		xml.open("sst", &[
			("xmlns", OoxmlNamespace::SPREADSHEET),
			("count", &count),
			("uniqueCount", &count),
		]);
		for string in strings {
			let edges = string.starts_with(char::is_whitespace)
				|| string.ends_with(char::is_whitespace);
			let attributes: &[(&str, &str)] = match edges {
				true => &[("xml:space", "preserve")],
				false => &[],
			};
			xml.open("si", &[])
				.text_element("t", attributes, string)
				.close("si");
		}
		xml.close("sst");
		xml.into_bytes()
	}
}

impl Installer for WorkbookRenderer {
	fn install(self, app: &mut App) { DocRenderers::register_on(app, self); }
}

impl DocRenderer for WorkbookRenderer {
	fn media_types(&self) -> Vec<MediaType> { vec![MediaType::Xlsx] }

	fn render(
		&mut self,
		cx: &mut RenderContext,
	) -> Result<MediaBytes, RenderError> {
		cx.check_accepts(&[MediaType::Xlsx])?;
		let (root, request) = (cx.entity, cx.request);
		let bytes = Self::write(cx.world, root, request)?;
		MediaBytes::new(MediaType::Xlsx, bytes).xok()
	}
}

#[cfg(test)]
mod test {
	use super::super::office_test::*;
	use super::super::workbook_parser::test::*;
	use crate::prelude::*;
	use crate::style::material::*;
	use crate::style::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// `root` written as a workbook for `request`.
	fn write(
		world: &mut World,
		root: Entity,
		request: RequestParts,
	) -> Result<MediaBytes> {
		super::super::office_test::write(world, root, MediaType::Xlsx, request)
	}

	/// The model's round trip: a workbook read, written and read again
	/// renders the same HTML, its properties too.
	#[beet_core::test]
	fn round_trips_the_model() {
		let (mut world, root) = parse(&budget()).unwrap();
		let first = html(&mut world, root);
		let written = write(&mut world, root, default()).unwrap();
		let (mut again, again_root) = parse(&written).unwrap();
		html(&mut again, again_root).xpect_eq(first);
		again
			.entity(again_root)
			.get::<DocMeta>()
			.cloned()
			.xpect_eq(world.entity(root).get::<DocMeta>().cloned());
		// the sheet typed as a person typing it would, and recalculated on open
		part(&written, "xl/worksheets/sheet1.xml")
			.xpect_contains("<c r=\"C2\"><v>120.5</v></c>")
			.xpect_contains("<c r=\"A3\" t=\"b\"><v>1</v></c>")
			.xpect_contains("<c r=\"B3\" t=\"e\"><v>#DIV/0!</v></c>")
			.xpect_contains("<c r=\"C3\"><f>C2*2</f><v>241</v></c>")
			.xpect_contains("<mergeCell ref=\"A5:B5\"/>");
		part(&written, "xl/workbook.xml")
			.xpect_contains(
				"<sheet name=\"Form\" sheetId=\"2\" r:id=\"rId2\" state=\"hidden\"/>",
			)
			.xpect_contains("fullCalcOnLoad=\"1\"");
		part(&written, "xl/worksheets/sheet2.xml")
			.xpect_contains("<sheetProtection sheet=\"1\"/>");
	}

	/// A workbook's theme colours read as tokens, and written for each
	/// scheme its theme part holds that scheme's tones, its cells naming
	/// the slots.
	#[beet_core::test]
	fn writes_each_schemes_theme() {
		let bytes = WorkbookFixture::default()
			.with_font("<font><sz val=\"11\"/><color theme=\"4\"/></font>")
			.with_format("<xf fontId=\"1\" fillId=\"0\"/>")
			.with_sheet(
				"Sheet",
				"<sheetData><row r=\"1\"><c r=\"A1\" s=\"1\" t=\"inlineStr\"><is><t>accent</t></is></c></row></sheetData>",
			)
			.bytes();
		let (mut world, root) = parse_in(
			(
				TemplatePlugin,
				DocumentPlugin,
				OoxmlPlugin,
				MaterialStylePlugin,
			)
				.into_world(),
			&bytes,
		)
		.unwrap();
		let primary = |world: &mut World, scheme: ColorScheme| {
			world.with_state::<RuleSetQuery, _>(|query| {
				query
					.resolve(
						root,
						colors::Primary,
						&mut CascadeMemo::for_media(
							MediaContext::new(Medium::Screen)
								.with_scheme(scheme),
						),
					)
					.unwrap()
			})
		};
		let mut accents = Vec::new();
		for scheme in [ColorScheme::Dark, ColorScheme::Light] {
			let written = write(
				&mut world,
				root,
				RequestParts::default().with_param(
					"color-scheme",
					if scheme == ColorScheme::Dark {
						"dark"
					} else {
						"light"
					},
				),
			)
			.unwrap();
			let theme = OfficeTheme::parse(
				&OpcPackage::read_xml(
					part(&written, "xl/theme/theme1.xml").as_bytes(),
				)
				.unwrap(),
			);
			let accent = theme.color(ThemeSlot::Accent1);
			OfficeColor::hex(accent)
				.xpect_eq(OfficeColor::hex(primary(&mut world, scheme)));
			part(&written, "xl/styles.xml")
				.xpect_contains("<color theme=\"4\"/>");
			accents.push(OfficeColor::hex(accent));
		}
		(accents[0] != accents[1]).xpect_true();
	}

	/// A page of two tables writes as a workbook of two sheets, one named by
	/// its caption and the other `Sheet2`, its cells typed; a page holding
	/// no table is refused.
	#[cfg(feature = "markdown_parser")]
	#[beet_core::test]
	fn writes_a_page_of_tables() {
		let page = |markdown: &str| {
			parse_in(
				(TemplatePlugin, DocumentPlugin, OoxmlPlugin).into_world(),
				&MediaBytes::new(
					MediaType::Markdown,
					markdown.as_bytes().to_vec(),
				),
			)
			.unwrap()
		};
		let (mut world, root) = page(
			"# Report\n\n<table><caption>Sales</caption><tr><td>Q1</td><td>12</td></tr></table>\n\n\
			 | item | count |\n|---|---|\n| apples | 007 |\n| pears | 3.5 |\n",
		);
		let written = write(&mut world, root, default()).unwrap();
		part(&written, "xl/workbook.xml")
			.xpect_contains("<sheet name=\"Sales\" sheetId=\"1\"")
			.xpect_contains("<sheet name=\"Sheet2\" sheetId=\"2\"");
		part(&written, "xl/worksheets/sheet1.xml")
			.xpect_contains("<c r=\"B1\"><v>12</v></c>");
		part(&written, "xl/worksheets/sheet2.xml")
			.xpect_contains("<c r=\"B2\" t=\"s\">")
			.xpect_contains("<c r=\"B3\"><v>3.5</v></c>");
		part(&written, "xl/sharedStrings.xml").xpect_contains("<t>007</t>");
		let (mut world, root) = page("# Nothing tabular\n");
		write(&mut world, root, default())
			.unwrap_err()
			.to_string()
			.xpect_contains("holds no table");
	}

	/// Sheet names are made valid and unique.
	#[beet_core::test]
	fn names_sheets_validly() {
		let taken = ["Sales"];
		WorkbookRenderer::sheet_name(
			Some("Q1: [draft]?"),
			0,
			taken.into_iter(),
		)
		.xpect_eq("Q1 draft");
		WorkbookRenderer::sheet_name(Some("sales"), 1, taken.into_iter())
			.xpect_eq("sales (2)");
		WorkbookRenderer::sheet_name(Some("/*"), 2, taken.into_iter())
			.xpect_eq("Sheet3");
		WorkbookRenderer::sheet_name(
			Some(&"x".repeat(40)),
			0,
			taken.into_iter(),
		)
		.chars()
		.count()
		.xpect_eq(31);
		for (text, number) in [
			("120.5", true),
			("0", true),
			("-3", true),
			("007", false),
			("1.0", false),
			(".5", false),
			("1e3", false),
		] {
			WorkbookRenderer::is_canonical_number(text).xpect_eq(number);
		}
	}

	/// LibreOffice opens a written workbook headless and converts it, the
	/// proxy for opening without a repair prompt, its values intact. Skips
	/// where `soffice` is absent.
	#[beet_core::test]
	fn libreoffice_opens_a_written_workbook() {
		if !has_soffice() {
			return;
		}
		let (mut world, root) = parse(&budget()).unwrap();
		let written = write(&mut world, root, default()).unwrap();
		soffice_convert(&written, "budget.xlsx", "csv")
			.xpect_contains("Name,Code,Total")
			.xpect_contains("Acme,007,120.5")
			.xpect_contains("TRUE,#DIV/0!,241")
			.xpect_contains("Merged");
	}
}
