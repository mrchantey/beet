use crate::prelude::*;
use beet_core::prelude::*;
use ooxmlsdk::parts::wordprocessing_document::WordprocessingDocument;

const W: &str = WordDocument::NAMESPACE;
const W14: &str = WordDocument::NAMESPACE_2010;
/// The empty and the ticked checkbox glyphs.
const BOX: char = '\u{2610}';
const TICKED: char = '\u{2612}';

/// A Word file, `.docx`: its parts as `ooxmlsdk` holds them, with the main
/// document part open as an [`XmlTree`] that the edits work on and a save
/// writes back. Opened from bytes or a [`Blob`] and saved to either; nothing
/// is unzipped beside it.
///
/// Every operation matches text with no-break spaces read as spaces, since
/// Word's placeholders carry them.
pub struct WordDocument {
	/// The file's parts.
	file: WordprocessingDocument,
	/// The main document part, `word/document.xml`.
	main: XmlTree,
}

/// One table cell of a [`WordDocument`]: its address and its paragraphs'
/// text, trimmed, the empty ones dropped and the rest joined by ` / `.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordCell {
	/// Where the cell is.
	pub address: TableCellAddress,
	/// What it says.
	pub text: String,
}

/// The cells dump's row, `| t1r1c1 | text |`, an empty cell `(empty)`.
impl core::fmt::Display for WordCell {
	fn fmt(
		&self,
		formatter: &mut core::fmt::Formatter<'_>,
	) -> core::fmt::Result {
		write!(
			formatter,
			"| {} | {} |",
			self.address,
			super::dump_text(&self.text)
		)
	}
}

impl WordDocument {
	/// WordprocessingML's main namespace, the `w:` prefix.
	pub const NAMESPACE: &str =
		"http://schemas.openxmlformats.org/wordprocessingml/2006/main";
	/// Word 2010's namespace, the `w14:` prefix, home of the checkbox
	/// control.
	pub const NAMESPACE_2010: &str =
		"http://schemas.microsoft.com/office/word/2010/wordml";

	/// Opens a Word file from its bytes.
	pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Result<Self> {
		let file =
			WordprocessingDocument::new(std::io::Cursor::new(bytes.into()))?;
		let main = file
			.main_document_part()?
			.try_data(&file)?
			.ok_or_else(|| bevyhow!("the Word file's main part is empty"))?
			.xmap(XmlTree::parse)?;
		Self { file, main }.xok()
	}

	/// A new Word file whose body is `body`, WordprocessingML with the `w:`
	/// and `w14:` prefixes bound, ie for a test fixture.
	pub fn from_body(body: &str) -> Result<Self> {
		let mut file = WordprocessingDocument::create(Default::default());
		let part = file.add_main_document_part()?;
		part.set_data(
			&mut file,
			format!(
				"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\r\n\
				 <w:document xmlns:w=\"{W}\" xmlns:w14=\"{W14}\" \
				 xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">\
				 <w:body>{body}</w:body></w:document>"
			),
		)?;
		let bytes = file.to_package_bytes()?;
		Self::from_bytes(bytes)
	}

	/// Opens the Word file `blob` holds.
	pub async fn open(blob: &Blob) -> Result<Self> {
		Self::from_bytes(blob.get().await?.to_vec())
	}

	/// The file's bytes, the main part written back from its tree.
	pub fn to_bytes(&mut self) -> Result<Vec<u8>> {
		self.file
			.main_document_part()?
			.set_data(&mut self.file, self.main.to_bytes())?;
		self.file.to_package_bytes()?.xok()
	}

	/// Writes the file to `blob`.
	pub async fn save(&mut self, blob: &Blob) -> Result {
		let bytes = self.to_bytes()?;
		blob.insert(bytes).await
	}

	/// The main document part's tree.
	pub fn main(&self) -> &XmlTree { &self.main }

	/// The file's parts, for what the edits here do not reach.
	pub fn file(&self) -> &WordprocessingDocument { &self.file }

	/// The file's core properties, `docProps/core.xml`.
	pub fn core_properties(&self) -> Result<CoreProperties> {
		CoreProperties::parse(
			self.file
				.core_file_properties_part()
				.map(|part| part.try_data(&self.file))
				.transpose()?
				.flatten(),
		)
	}

	/// Every table cell in document order, tables numbered as they open.
	pub fn cells(&self) -> Vec<WordCell> {
		let mut cells = Vec::new();
		let tables = self.main.root.descendants_named(W, "tbl");
		for (table_index, table) in tables.into_iter().enumerate() {
			for (row_index, row) in table.children_named(W, "tr").enumerate() {
				for (cell_index, cell) in
					row.children_named(W, "tc").enumerate()
				{
					cells.push(WordCell {
						address: TableCellAddress {
							table: table_index as u32 + 1,
							row: row_index as u32 + 1,
							cell: cell_index as u32 + 1,
						},
						text: Self::cell_text(cell),
					});
				}
			}
		}
		cells
	}

	/// Replaces the cell's paragraphs with one per line of `text`, each in
	/// the paragraph style of the cell's first paragraph and the run style of
	/// its first run with text.
	pub fn set_cell(
		&mut self,
		address: TableCellAddress,
		text: &str,
	) -> Result {
		self.write_cell(address, text, false)
	}

	/// Adds one paragraph per line of `text` after the cell's own, for a
	/// one-cell box whose prompt shares the cell with its answer.
	pub fn append_cell(
		&mut self,
		address: TableCellAddress,
		text: &str,
	) -> Result {
		self.write_cell(address, text, true)
	}

	/// Ticks every checkbox control whose paragraph carries `label`: the
	/// control marked checked, its glyph ticked, and the glyph's run made bold
	/// and highlighted so the mark survives an upload that drops the control.
	/// Answers how many it ticked.
	pub fn check(&mut self, label: &str) -> Result<usize> {
		let needle = plain(label);
		let checkboxes = self.main.root.paths(|element| {
			element.is(W, "sdt")
				&& element.child(W, "sdtPr").is_some_and(|properties| {
					!properties.descendants_named(W14, "checkbox").is_empty()
				})
		});
		let mut ticked = 0;
		for path in checkboxes {
			let labelled = (0..path.len()).rev().find_map(|len| {
				self.main
					.root
					.at(&path[..len])
					.filter(|element| element.is(W, "p"))
			});
			match labelled {
				Some(paragraph)
					if plain(&paragraph.text_of(W, "t")).contains(&needle) =>
				{
					self.tick(&path)?;
					ticked += 1;
				}
				_ => {}
			}
		}
		ticked.xok()
	}

	/// Removes every paragraph carrying `text`, ie a red instruction
	/// sentence; a cell's last paragraph is emptied rather than removed, since
	/// a cell must end in one. Answers how many it removed.
	pub fn delete_paragraphs(&mut self, text: &str) -> Result<usize> {
		let needle = Self::needle(text)?;
		let paths = self.paragraphs_carrying(&needle);
		// last first, so every earlier path still points where it did
		for path in paths.iter().rev() {
			let (parent_path, index) = path.split_at(path.len() - 1);
			let parent = self.main.root.at(parent_path).ok_or_else(|| {
				bevyhow!("a matched paragraph lost its parent")
			})?;
			let replacement = match parent.is(W, "tc")
				&& parent.children_named(W, "p").count() == 1
			{
				true => Some(self.paragraph(parent.at(index), "")?),
				false => None,
			};
			let parent = self.main.root.at_mut(parent_path).unwrap();
			match replacement {
				Some(empty) => {
					parent.children[index[0]] = XmlNode::Element(empty)
				}
				None => {
					parent.children.remove(index[0]);
				}
			}
		}
		paths.len().xok()
	}

	/// Replaces `old` with `new` inside the runs that carry it, touching only
	/// the runs the match covers so the rest keep their formatting and a
	/// highlighted placeholder stays highlighted. Answers how many it
	/// replaced.
	pub fn replace_text(&mut self, old: &str, new: &str) -> Result<usize> {
		let needle = Self::needle(old)?;
		let preserve =
			self.main
				.attribute(XmlTree::XML_NAMESPACE, "space", "preserve")?;
		let mut replaced = 0;
		for path in self.paragraphs_carrying(&needle) {
			let paragraph = self.main.root.at_mut(&path).unwrap();
			replaced +=
				Self::replace_in_paragraph(paragraph, &needle, new, &preserve);
		}
		replaced.xok()
	}

	/// A cell's paragraphs' text as the cells dump shows it.
	fn cell_text(cell: &XmlElement) -> String {
		cell.children_named(W, "p")
			.map(|paragraph| paragraph.text_of(W, "t").trim().to_string())
			.filter(|text| !text.is_empty())
			.collect::<Vec<_>>()
			.join(" / ")
	}

	/// Refuses an empty match, which every paragraph carries.
	fn needle(text: &str) -> Result<String> {
		match plain(text) {
			needle if needle.is_empty() => {
				bevybail!("an empty text matches every paragraph")
			}
			needle => needle.xok(),
		}
	}

	fn paragraphs_carrying(&self, needle: &str) -> Vec<Vec<usize>> {
		self.main.root.paths(|element| {
			element.is(W, "p")
				&& plain(&element.text_of(W, "t")).contains(needle)
		})
	}

	fn write_cell(
		&mut self,
		address: TableCellAddress,
		text: &str,
		append: bool,
	) -> Result {
		let path = self.cell_path(address)?;
		let cell = self.main.root.at(&path).unwrap();
		let model = cell.child(W, "p");
		let paragraphs = text
			.split('\n')
			.map(|line| self.paragraph(model, line).map(XmlNode::Element))
			.collect::<Result<Vec<_>>>()?;
		let cell = self.main.root.at_mut(&path).unwrap();
		if !append {
			cell.children.retain(
				|node| !matches!(node, XmlNode::Element(element) if element.is(W, "p")),
			);
		}
		// a cell's properties come first and its paragraphs last, so
		// appending keeps the order
		cell.children.extend(paragraphs);
		Ok(())
	}

	/// The child index path of the cell at `address`.
	fn cell_path(&self, address: TableCellAddress) -> Result<Vec<usize>> {
		let missing = || bevyhow!("the Word file has no cell {address}");
		let mut path = self
			.main
			.root
			.paths(|element| element.is(W, "tbl"))
			.into_iter()
			.nth(address.table as usize - 1)
			.ok_or_else(missing)?;
		for (local, nth) in [("tr", address.row), ("tc", address.cell)] {
			let parent = self.main.root.at(&path).unwrap();
			let index = parent
				.children
				.iter()
				.enumerate()
				.filter(|(_, node)| {
					matches!(node, XmlNode::Element(element) if element.is(W, local))
				})
				.nth(nth as usize - 1)
				.map(|(index, _)| index)
				.ok_or_else(missing)?;
			path.push(index);
		}
		path.xok()
	}

	/// A new paragraph of one run of `text`, carrying the paragraph
	/// properties of `model` and the run properties of its first run with
	/// text, else its first run.
	fn paragraph(
		&self,
		model: Option<&XmlElement>,
		text: &str,
	) -> Result<XmlElement> {
		let mut paragraph = self.main.element(W, "p")?;
		if let Some(properties) = model.and_then(|model| model.child(W, "pPr"))
		{
			paragraph
				.children
				.push(XmlNode::Element(properties.clone()));
		}
		let mut run = self.main.element(W, "r")?;
		let runs = model
			.map(|model| model.descendants_named(W, "r"))
			.unwrap_or_default();
		let model_run = runs
			.iter()
			.find(|run| !run.descendants_named(W, "t").is_empty())
			.or(runs.first());
		if let Some(properties) = model_run.and_then(|run| run.child(W, "rPr"))
		{
			run.children.push(XmlNode::Element(properties.clone()));
		}
		let mut text_element = self.main.element(W, "t")?;
		text_element.set_attribute(self.main.attribute(
			XmlTree::XML_NAMESPACE,
			"space",
			"preserve",
		)?);
		text_element.set_text(text);
		run.children.push(XmlNode::Element(text_element));
		paragraph.children.push(XmlNode::Element(run));
		paragraph.xok()
	}

	/// Marks the checkbox control at `path` checked, ticks its glyphs and
	/// makes their runs bold and highlighted.
	fn tick(&mut self, path: &[usize]) -> Result {
		let checked = self.main.attribute(W14, "val", "1")?;
		let mut checked_element = self.main.element(W14, "checked")?;
		checked_element.set_attribute(checked.clone());
		let bold = self.main.element(W, "b")?;
		let mut highlight = self.main.element(W, "highlight")?;
		highlight.set_attribute(self.main.attribute(W, "val", "yellow")?);
		let empty_properties = self.main.element(W, "rPr")?;

		let control = self.main.root.at_mut(path).unwrap();
		let checkbox_path = control
			.paths(|element| element.is(W14, "checkbox"))
			.into_iter()
			.next()
			.unwrap();
		let checkbox = control.at_mut(&checkbox_path).unwrap();
		match checkbox.child_mut(W14, "checked") {
			Some(existing) => existing.set_attribute(checked),
			// the schema puts `checked` first
			None => checkbox
				.children
				.insert(0, XmlNode::Element(checked_element)),
		}
		let glyphs = control.paths(|element| {
			element.is(W, "t") && element.text().contains(BOX)
		});
		for glyph_path in glyphs {
			let glyph = control.at_mut(&glyph_path).unwrap();
			let ticked = glyph.text().replacen(BOX, &TICKED.to_string(), 1);
			glyph.set_text(ticked);
			let run =
				control.at_mut(&glyph_path[..glyph_path.len() - 1]).unwrap();
			if run.child(W, "rPr").is_none() {
				run.children
					.insert(0, XmlNode::Element(empty_properties.clone()));
			}
			let properties = run.child_mut(W, "rPr").unwrap();
			for mark in [&bold, &highlight] {
				if properties.child(W, mark.local_name()).is_none() {
					insert_run_property(properties, mark.clone());
				}
			}
		}
		Ok(())
	}

	/// Replaces every `needle` in `paragraph`'s text runs, splicing only the
	/// runs each match covers; the first covered run takes the replacement.
	fn replace_in_paragraph(
		paragraph: &mut XmlElement,
		needle: &str,
		new: &str,
		preserve: &XmlAttribute,
	) -> usize {
		let text_paths = paragraph.paths(|element| element.is(W, "t"));
		let mut replaced = 0;
		// bounded, since a replacement may itself carry the needle
		for _ in 0..50 {
			let texts = text_paths
				.iter()
				.map(|path| plain(&paragraph.at(path).unwrap().text()))
				.collect::<Vec<_>>();
			let Some(at) = texts.concat().find(needle) else {
				return replaced;
			};
			let end = at + needle.len();
			let mut position = 0;
			let mut placed = false;
			for (path, text) in text_paths.iter().zip(&texts) {
				let (start, stop) = (position, position + text.len());
				position = stop;
				if stop <= at || start >= end {
					continue;
				}
				let before = &text[..at.saturating_sub(start).min(text.len())];
				let after = &text[(end - start).min(text.len())..];
				let element = paragraph.at_mut(path).unwrap();
				element.set_text(format!(
					"{before}{}{after}",
					if placed { "" } else { new }
				));
				element.set_attribute(preserve.clone());
				placed = true;
			}
			replaced += 1;
		}
		replaced
	}
}

/// The order `CT_RPr` fixes for a run's properties, which Word holds a file
/// to.
const RUN_PROPERTY_ORDER: [&str; 40] = [
	"rStyle",
	"rFonts",
	"b",
	"bCs",
	"i",
	"iCs",
	"caps",
	"smallCaps",
	"strike",
	"dstrike",
	"outline",
	"shadow",
	"emboss",
	"imprint",
	"noProof",
	"snapToGrid",
	"vanish",
	"webHidden",
	"color",
	"spacing",
	"w",
	"kern",
	"position",
	"sz",
	"szCs",
	"highlight",
	"u",
	"effect",
	"bdr",
	"shd",
	"fitText",
	"vertAlign",
	"rtl",
	"cs",
	"em",
	"lang",
	"eastAsianLayout",
	"specVanish",
	"oMath",
	"rPrChange",
];

/// Inserts `property` into a run's `w:rPr` where the schema orders it.
fn insert_run_property(properties: &mut XmlElement, property: XmlElement) {
	let rank = |local: &str| {
		RUN_PROPERTY_ORDER
			.iter()
			.position(|name| *name == local)
			.unwrap_or(RUN_PROPERTY_ORDER.len())
	};
	let own = rank(property.local_name());
	let index = properties
		.children
		.iter()
		.position(|node| {
			matches!(node, XmlNode::Element(element)
				if element.namespace.as_deref() == Some(W)
					&& rank(element.local_name()) > own)
		})
		.unwrap_or(properties.children.len());
	properties
		.children
		.insert(index, XmlNode::Element(property));
}

/// Text as matching reads it, a no-break space as a space.
fn plain(text: &str) -> String { text.replace('\u{a0}', " ") }

#[cfg(test)]
mod test {
	use super::*;

	/// A cell with a prompt and a checkbox, a red sentence and a highlighted
	/// placeholder, as the forms a builder fills carry them.
	fn form() -> WordDocument {
		WordDocument::from_body(
			"<w:tbl><w:tr>\
			 <w:tc><w:p><w:r><w:rPr><w:sz w:val=\"20\"/></w:rPr><w:t>Name</w:t></w:r></w:p></w:tc>\
			 <w:tc><w:p><w:pPr><w:jc w:val=\"left\"/></w:pPr></w:p></w:tc>\
			 </w:tr><w:tr><w:tc>\
			 <w:p><w:r><w:t>Describe it: (Please delete this sentence once completed)</w:t></w:r></w:p>\
			 </w:tc><w:tc><w:p>\
			 <w:sdt><w:sdtPr><w14:checkbox><w14:checked w14:val=\"0\"/></w14:checkbox></w:sdtPr>\
			 <w:sdtContent><w:r><w:rPr><w:sz w:val=\"20\"/></w:rPr><w:t>\u{2610}</w:t></w:r></w:sdtContent></w:sdt>\
			 <w:r><w:t xml:space=\"preserve\"> Surveys</w:t></w:r></w:p></w:tc>\
			 </w:tr></w:tbl>\
			 <w:p><w:r><w:t>The business is </w:t></w:r>\
			 <w:r><w:rPr><w:highlight w:val=\"yellow\"/></w:rPr><w:t>(Insert</w:t></w:r>\
			 <w:r><w:rPr><w:highlight w:val=\"yellow\"/></w:rPr><w:t>\u{a0}name)</w:t></w:r></w:p>",
		)
		.unwrap()
	}

	#[beet_core::test]
	fn dumps_cells_in_document_order() {
		form()
			.cells()
			.iter()
			.map(ToString::to_string)
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				"| t1r1c1 | Name |".to_string(),
				"| t1r1c2 | (empty) |".into(),
				"| t1r2c1 | Describe it: (Please delete this sentence once completed) |".into(),
				"| t1r2c2 | \u{2610} Surveys |".into(),
			]);
	}

	#[beet_core::test]
	fn fills_and_round_trips() {
		let mut form = form();
		let address = |text| TableCellAddress::parse(text).unwrap();
		form.set_cell(address("t1r1c2"), "Acme Stalls\nSecond line")
			.unwrap();
		form.append_cell(address("t1r1c1"), "appended").unwrap();
		form.check("Surveys").unwrap().xpect_eq(1);
		form.check("Nothing").unwrap().xpect_eq(0);
		form.delete_paragraphs("Please delete this sentence")
			.unwrap()
			.xpect_eq(1);
		form.replace_text("(Insert name)", "Acme")
			.unwrap()
			.xpect_eq(1);
		form.set_cell(address("t9r1c1"), "x").xpect_err();
		form.delete_paragraphs("").xpect_err();

		let form = WordDocument::from_bytes(form.to_bytes().unwrap()).unwrap();
		form.cells()
			.iter()
			.map(|cell| cell.text.clone())
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				"Name / appended".to_string(),
				"Acme Stalls / Second line".into(),
				String::new(),
				"\u{2612} Surveys".into(),
			]);
		let xml = String::from_utf8(form.main().to_bytes()).unwrap();
		xml.clone()
			.xpect_contains("<w14:checked w14:val=\"1\"/>")
			// bold goes before the size, highlight after, as the schema orders
			.xpect_contains("<w:rPr><w:b/><w:sz w:val=\"20\"/><w:highlight w:val=\"yellow\"/></w:rPr>")
			// the new paragraph keeps the cell's paragraph style
			.xpect_contains("<w:pPr><w:jc w:val=\"left\"/></w:pPr><w:r><w:t xml:space=\"preserve\">Acme Stalls</w:t>")
			.xpect_contains("The business is </w:t></w:r><w:r><w:rPr><w:highlight w:val=\"yellow\"/></w:rPr><w:t xml:space=\"preserve\">Acme</w:t>");
	}
}
