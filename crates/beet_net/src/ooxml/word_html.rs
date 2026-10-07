use super::html;
use crate::prelude::*;
use beet_core::prelude::*;
use ooxmlsdk::parts::PartRef;

type Ns = OoxmlNamespace;

impl WordDocument {
	/// The file as HTML, the form every parser in beet produces, so a Word
	/// file renders through the same pipelines as a page: a `<header>` naming
	/// the core properties, headings by their paragraph style, list items by
	/// their numbering level, tables headed `<!-- t<n> -->` with the number
	/// the cells dump gives them, a text box as an `<aside>` after the
	/// paragraph anchoring it, and an image by its part. What a form signals
	/// by look survives: a highlight is a `<mark>`, a run's shading a `<mark>`
	/// with its background, a colour other than black a `<span>` with its
	/// colour, bold and italic `<strong>` and `<em>`.
	pub fn to_html(&self) -> Result<String> {
		let core = self.core_properties()?;
		let main = self.file().main_document_part()?;
		let links = main
			.hyperlink_relationships(self.file())
			.map(|link| (link.id().to_string(), link.target().to_string()))
			.collect();
		let images = main
			.parts(self.file())
			.filter_map(|pair| match pair.part {
				PartRef::ImagePart(image) => {
					image.path(self.file()).map(|path| {
						(
							pair.relationship_id.to_string(),
							path.trim_start_matches('/').to_string(),
						)
					})
				}
				_ => None,
			})
			.collect();
		let root = &self.main().root;
		let tables = root
			.descendants_named(Ns::WORD, "tbl")
			.into_iter()
			.enumerate()
			.map(|(index, table)| (table as *const XmlElement, index + 1))
			.collect();
		let writer = HtmlWriter {
			links,
			images,
			tables,
		};
		let body = root
			.child(Ns::WORD, "body")
			.map(|body| writer.blocks(body))
			.unwrap_or_default();
		format!(
			"<header><p>{}</p></header>\n{body}",
			html::text(&core.sentence())
		)
		.xok()
	}
}

/// Writes a Word part's blocks as HTML against its relationships and table
/// numbers.
struct HtmlWriter {
	/// Hyperlink targets by relationship id.
	links: HashMap<String, String>,
	/// Image part paths by relationship id.
	images: HashMap<String, String>,
	/// Each table's number in the cells dump, by identity.
	tables: HashMap<*const XmlElement, usize>,
}

/// A run of text sharing one look.
#[derive(Default, Clone, PartialEq)]
struct Segment {
	/// The look's markup, opening and closing, ie `<mark>` and `</mark>`.
	wrap: Option<(String, &'static str)>,
	bold: bool,
	italic: bool,
	/// The text, already HTML.
	html: String,
}

impl HtmlWriter {
	/// A container's blocks: paragraphs, lists of numbered paragraphs, tables
	/// and the content of controls and alternate content.
	fn blocks(&self, container: &XmlElement) -> String {
		let mut out = String::new();
		// the levels of the lists open around the next paragraph
		let mut lists = 0usize;
		for element in container.elements() {
			let level = (element.is(Ns::WORD, "p"))
				.then(|| Self::list_level(element))
				.flatten();
			let wanted = level.map_or(0, |level| level + 1);
			while lists > wanted {
				out.push_str("</ul>\n");
				lists -= 1;
			}
			while lists < wanted {
				out.push_str("<ul>\n");
				lists += 1;
			}
			match (element.namespace.as_deref(), element.local_name()) {
				(Some(Ns::WORD), "p") => {
					let (html, boxes) =
						self.paragraph(element, level.is_some());
					out.push_str(&html);
					for text_box in boxes {
						out.push_str(&format!("<aside>\n{text_box}</aside>\n"));
					}
				}
				(Some(Ns::WORD), "tbl") => out.push_str(&self.table(element)),
				(Some(Ns::WORD), "sdt") => {
					if let Some(content) = element.child(Ns::WORD, "sdtContent")
					{
						out.push_str(&self.blocks(content));
					}
				}
				(Some(Ns::WORD), "customXml") => {
					out.push_str(&self.blocks(element))
				}
				(Some(Ns::COMPATIBILITY), "AlternateContent") => {
					if let Some(choice) =
						element.child(Ns::COMPATIBILITY, "Choice")
					{
						out.push_str(&self.blocks(choice));
					}
				}
				_ => {}
			}
		}
		for _ in 0..lists {
			out.push_str("</ul>\n");
		}
		out
	}

	/// A numbered paragraph's level, from 0.
	fn list_level(paragraph: &XmlElement) -> Option<usize> {
		let numbering =
			paragraph.child(Ns::WORD, "pPr")?.child(Ns::WORD, "numPr")?;
		numbering
			.child(Ns::WORD, "ilvl")
			.and_then(|level| level.attribute(Some(Ns::WORD), "val"))
			.and_then(|level| level.parse().ok())
			.unwrap_or(0)
			.xsome()
	}

	/// A paragraph as its block, a heading by its style or a list item, with
	/// the text boxes its runs anchor; an empty one is no block.
	fn paragraph(
		&self,
		paragraph: &XmlElement,
		listed: bool,
	) -> (String, Vec<String>) {
		let style = paragraph
			.child(Ns::WORD, "pPr")
			.and_then(|properties| properties.child(Ns::WORD, "pStyle"))
			.and_then(|style| style.attribute(Some(Ns::WORD), "val"))
			.unwrap_or_default();
		let tag = match style {
			"Title" => "h1".to_string(),
			style => match style
				.strip_prefix("Heading")
				.and_then(|level| level.parse::<usize>().ok())
			{
				Some(level) if (1..=6).contains(&level) => format!("h{level}"),
				_ if listed => "li".into(),
				_ => "p".into(),
			},
		};
		let mut segments = Vec::new();
		let mut boxes = Vec::new();
		self.inline(paragraph, &mut segments, &mut boxes);
		let html = Self::render(&segments);
		let block = match html.trim().is_empty() {
			true => String::new(),
			false => format!("<{tag}>{html}</{tag}>\n"),
		};
		(block, boxes)
	}

	/// The runs under `element` as segments, through every wrapper a run may
	/// sit in.
	fn inline(
		&self,
		element: &XmlElement,
		segments: &mut Vec<Segment>,
		boxes: &mut Vec<String>,
	) {
		for child in element.elements() {
			match (child.namespace.as_deref(), child.local_name()) {
				(Some(Ns::WORD), "r") => self.run(child, segments, boxes),
				(Some(Ns::WORD), "hyperlink") => {
					let mut inner = Vec::new();
					self.inline(child, &mut inner, boxes);
					match child
						.attribute(Some(Ns::RELATIONSHIPS), "id")
						.and_then(|id| self.links.get(id))
					{
						Some(target) => segments.push(Segment {
							html: format!(
								"<a href=\"{}\">{}</a>",
								html::attribute(target),
								Self::render(&inner)
							),
							..Default::default()
						}),
						None => segments.extend(inner),
					}
				}
				(Some(Ns::WORD), "del" | "moveFrom" | "pPr") => {}
				(Some(Ns::WORD), "sdt") => {
					if let Some(content) = child.child(Ns::WORD, "sdtContent") {
						self.inline(content, segments, boxes);
					}
				}
				(Some(Ns::COMPATIBILITY), "AlternateContent") => {
					if let Some(choice) =
						child.child(Ns::COMPATIBILITY, "Choice")
					{
						self.inline(choice, segments, boxes);
					}
				}
				_ => self.inline(child, segments, boxes),
			}
		}
	}

	/// One run: its text in its look, its images in place, its text boxes
	/// for after the paragraph.
	fn run(
		&self,
		run: &XmlElement,
		segments: &mut Vec<Segment>,
		boxes: &mut Vec<String>,
	) {
		let properties = run.child(Ns::WORD, "rPr");
		let property = |local: &str| {
			properties.and_then(|properties| properties.child(Ns::WORD, local))
		};
		let switched_on = |local: &str| {
			property(local).is_some_and(|element| {
				!matches!(
					element.attribute(Some(Ns::WORD), "val"),
					Some("0" | "false")
				)
			})
		};
		let mut html = String::new();
		for child in run.elements() {
			match (child.namespace.as_deref(), child.local_name()) {
				(Some(Ns::WORD), "t") => {
					html.push_str(&html::text(&child.text()))
				}
				(Some(Ns::WORD), "tab") => html.push(' '),
				(Some(Ns::WORD), "br" | "cr") => html.push_str("<br>"),
				(Some(Ns::WORD), "noBreakHyphen") => html.push('-'),
				(Some(Ns::WORD), "drawing" | "pict")
				| (Some(Ns::COMPATIBILITY), "AlternateContent") => {
					let content =
						match child.is(Ns::COMPATIBILITY, "AlternateContent") {
							true => child.child(Ns::COMPATIBILITY, "Choice"),
							false => Some(child),
						};
					let Some(content) = content else { continue };
					for blip in content.descendants_named(Ns::DRAWING, "blip") {
						let source = blip
							.attribute(Some(Ns::RELATIONSHIPS), "embed")
							.and_then(|id| self.images.get(id))
							.cloned()
							.unwrap_or_default();
						let described = content
							.descendants_named(Ns::WORD_DRAWING, "docPr")
							.first()
							.map(|picture| {
								[
									picture.attribute(None, "name"),
									picture.attribute(None, "descr"),
								]
								.into_iter()
								.flatten()
								.filter(|part| !part.is_empty())
								.collect::<Vec<_>>()
								.join(", ")
							})
							.unwrap_or_default();
						html.push_str(&format!(
							"<img src=\"{}\" alt=\"{}\">",
							html::attribute(&source),
							html::attribute(&described)
						));
					}
					for text_box in
						content.descendants_named(Ns::WORD, "txbxContent")
					{
						let blocks = self.blocks(text_box);
						if !blocks.trim().is_empty() {
							boxes.push(blocks);
						}
					}
				}
				_ => {}
			}
		}
		if html.is_empty() {
			return;
		}
		let segment = Segment {
			wrap: Self::look(
				property("highlight"),
				property("shd"),
				property("color"),
			),
			bold: switched_on("b"),
			italic: switched_on("i"),
			html,
		};
		match segments.last_mut() {
			Some(last)
				if last.wrap == segment.wrap
					&& last.bold == segment.bold
					&& last.italic == segment.italic =>
			{
				last.html.push_str(&segment.html)
			}
			_ => segments.push(segment),
		}
	}

	/// The markup a run's look wraps it in: its highlight, else its
	/// shading, else a colour other than black or white.
	fn look(
		highlight: Option<&XmlElement>,
		shading: Option<&XmlElement>,
		colour: Option<&XmlElement>,
	) -> Option<(String, &'static str)> {
		let value = |element: Option<&XmlElement>, local: &str| {
			element
				.and_then(|element| element.attribute(Some(Ns::WORD), local))
				.map(str::to_string)
		};
		value(highlight, "val")
			.filter(|highlight| highlight != "none")
			.map(|highlight| match highlight.as_str() {
				"yellow" => ("<mark>".to_string(), "</mark>"),
				other => {
					(format!("<mark style=\"background: {other}\">"), "</mark>")
				}
			})
			.or_else(|| {
				value(shading, "fill")
					.filter(|fill| !Self::is_plain(fill))
					.map(|fill| {
						(
							format!("<mark style=\"background: #{fill}\">"),
							"</mark>",
						)
					})
			})
			.or_else(|| {
				value(colour, "val")
					.filter(|colour| {
						!Self::is_plain(colour) && colour != "000000"
					})
					.map(|colour| {
						(
							format!("<span style=\"color: #{colour}\">"),
							"</span>",
						)
					})
			})
	}

	/// Automatic or white, which is no look at all.
	fn is_plain(colour: &str) -> bool {
		matches!(colour.to_ascii_lowercase().as_str(), "auto" | "ffffff")
	}

	/// Segments as HTML, emphasis inside the look and a segment's edge
	/// whitespace outside both, so a bold `Date ` reads `**Date** `.
	fn render(segments: &[Segment]) -> String {
		segments
			.iter()
			.map(|segment| {
				let core = segment.html.trim();
				if core.is_empty() {
					return segment.html.clone();
				}
				let lead = &segment.html
					[..segment.html.len() - segment.html.trim_start().len()];
				let trail = &segment.html[segment.html.trim_end().len()..];
				let mut html = core.to_string();
				if segment.italic {
					html = format!("<em>{html}</em>");
				}
				if segment.bold {
					html = format!("<strong>{html}</strong>");
				}
				if let Some((open, close)) = &segment.wrap {
					html = format!("{open}{html}{close}");
				}
				format!("{lead}{html}{trail}")
			})
			.collect()
	}

	/// A table headed by its number in the cells dump; a spanned cell
	/// carries its `colspan`, a vertically merged one is left empty, a cell's
	/// shading is its background, and a nested table is named in its cell
	/// and follows its parent.
	fn table(&self, table: &XmlElement) -> String {
		let number = |table: &XmlElement| {
			self.tables
				.get(&(table as *const XmlElement))
				.copied()
				.unwrap_or(0)
		};
		let mut nested = Vec::new();
		let mut out = format!("<!-- t{} -->\n<table>\n", number(table));
		for row in table.children_named(Ns::WORD, "tr") {
			out.push_str("<tr>");
			for cell in row.children_named(Ns::WORD, "tc") {
				let properties = cell.child(Ns::WORD, "tcPr");
				let property = |local: &str| {
					properties.and_then(|properties| {
						properties.child(Ns::WORD, local)
					})
				};
				let mut attributes = String::new();
				if let Some(span) = property("gridSpan")
					.and_then(|span| span.attribute(Some(Ns::WORD), "val"))
					.filter(|span| *span != "1")
				{
					attributes.push_str(&format!(
						" colspan=\"{}\"",
						html::attribute(span)
					));
				}
				if let Some(fill) = property("shd")
					.and_then(|shading| {
						shading.attribute(Some(Ns::WORD), "fill")
					})
					.filter(|fill| !Self::is_plain(fill))
				{
					attributes.push_str(&format!(
						" style=\"background: #{}\"",
						html::attribute(fill)
					));
				}
				let merged = property("vMerge").is_some_and(|merge| {
					merge.attribute(Some(Ns::WORD), "val") != Some("restart")
				});
				let content = match merged {
					true => String::new(),
					false => {
						let mut content = String::new();
						for element in cell.elements() {
							match element.is(Ns::WORD, "tbl") {
								true => {
									nested.push(element);
									content.push_str(&format!(
										"<p>[table t{}]</p>",
										number(element)
									));
								}
								false => {
									let mut single =
										XmlElement::new("cell", None);
									single.children.push(XmlNode::Element(
										element.clone(),
									));
									content.push_str(
										self.blocks(&single).trim_end(),
									);
								}
							}
						}
						content
					}
				};
				out.push_str(&format!("<td{attributes}>{content}</td>"));
			}
			out.push_str("</tr>\n");
		}
		out.push_str("</table>\n");
		for inner in nested {
			out.push_str(&self.table(inner));
		}
		out
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	fn transcodes_to_html() {
		WordDocument::from_body(
			"<w:p><w:pPr><w:pStyle w:val=\"Heading1\"/></w:pPr><w:r><w:t>Cover</w:t></w:r></w:p>\
			 <w:p><w:r><w:t xml:space=\"preserve\">Where is it? </w:t></w:r>\
			 <w:r><w:rPr><w:color w:val=\"FF0000\"/></w:rPr><w:t>(Please delete this)</w:t></w:r></w:p>\
			 <w:p><w:pPr><w:numPr><w:ilvl w:val=\"0\"/></w:numPr></w:pPr><w:r><w:t>one</w:t></w:r></w:p>\
			 <w:p><w:pPr><w:numPr><w:ilvl w:val=\"1\"/></w:numPr></w:pPr><w:r><w:t>nested</w:t></w:r></w:p>\
			 <w:tbl><w:tr><w:tc><w:p><w:r><w:rPr><w:b/></w:rPr><w:t>Name</w:t></w:r></w:p></w:tc>\
			 <w:tc><w:tcPr><w:gridSpan w:val=\"2\"/></w:tcPr><w:p><w:r><w:rPr><w:highlight w:val=\"yellow\"/></w:rPr><w:t>a &lt; b</w:t></w:r></w:p></w:tc></w:tr></w:tbl>\
			 <w:p><w:r><w:rPr><w:b/></w:rPr><w:t xml:space=\"preserve\">Date </w:t></w:r><w:r><w:t>due</w:t></w:r></w:p>",
		)
		.unwrap()
		.to_html()
		.unwrap()
		.xpect_contains("<h1>Cover</h1>\n<p>Where is it? <span style=\"color: #FF0000\">(Please delete this)</span></p>\n")
		.xpect_contains("<ul>\n<li>one</li>\n<ul>\n<li>nested</li>\n</ul>\n</ul>\n")
		.xpect_contains(
			"<!-- t1 -->\n<table>\n<tr><td><p><strong>Name</strong></p></td><td colspan=\"2\"><p><mark>a &lt; b</mark></p></td></tr>\n</table>\n",
		)
		// edge whitespace sits outside the emphasis
		.xpect_contains("<p><strong>Date</strong> due</p>");
	}
}
