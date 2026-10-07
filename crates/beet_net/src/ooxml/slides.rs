use super::html;
use crate::prelude::*;
use beet_core::prelude::*;
use ooxmlsdk::parts::PartRef;
use ooxmlsdk::parts::presentation_document::PresentationDocument;
use ooxmlsdk::parts::slide_part::SlidePart;

type Ns = OoxmlNamespace;

/// A slide deck, `.pptx`, read for its words: the slide order and size come
/// from `ooxmlsdk`'s typed presentation part, each slide and its notes are
/// read as an [`XmlTree`], and a picture's native size from its own header.
pub struct SlideDeck {
	/// The file's parts.
	file: PresentationDocument,
}

impl SlideDeck {
	/// A picture covering at least this share of a slide, in percent, carries
	/// content rather than decoration, so a recurrence of it is reported.
	const RECURRING_FLOOR: u32 = 25;

	/// Opens a slide deck from its bytes.
	pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Result<Self> {
		Self {
			file: PresentationDocument::new(std::io::Cursor::new(
				bytes.into(),
			))?,
		}
		.xok()
	}

	/// Opens the slide deck `blob` holds.
	pub async fn open(blob: &Blob) -> Result<Self> {
		Self::from_bytes(blob.get().await?.to_vec())
	}

	/// The deck's core properties, `docProps/core.xml`.
	pub fn core_properties(&self) -> Result<CoreProperties> {
		CoreProperties::parse(
			self.file
				.core_file_properties_part()
				.map(|part| part.try_data(&self.file))
				.transpose()?
				.flatten(),
		)
	}

	/// The deck as HTML, the form every parser in beet produces, so a deck
	/// renders through the same pipelines as a page: a `<header>` with its
	/// size and core properties, then one `<section>` per slide in
	/// presentation order, headed `Slide N`, with the slide's text in reading
	/// order, each shape marked by its kind, its tables, its SmartArt read out
	/// of the diagram part, its picture inventory and its speaker notes, the
	/// deck's empty `Script:` stubs dropped. A closing report triages every
	/// slide by what carries it, words or a picture, and names hidden slides,
	/// recurring pictures, same-size pictures and exact duplicates.
	pub fn to_html(&self) -> Result<String> {
		let presentation_part = self.file.presentation_part()?;
		let presentation = presentation_part.root_element(&self.file)?;
		let (width, height) = presentation
			.slide_size
			.as_ref()
			.map_or((9_144_000, 6_858_000), |size| {
				(size.cx as i64, size.cy as i64)
			});
		let mut parts: HashMap<String, SlidePart> = HashMap::default();
		for part in presentation_part.slide_parts(&self.file) {
			let id = presentation_part
				.get_id_of_part(&self.file, &part)?
				.to_string();
			parts.insert(id, part);
		}
		let listed = presentation
			.slide_id_list
			.iter()
			.flat_map(|list| list.slide_id.iter())
			.map(|slide| slide.relationship_id.clone())
			.collect::<Vec<_>>();
		let area = (width * height).max(1);
		let mut slides = Vec::new();
		for id in &listed {
			let Some(part) = parts.remove(id) else {
				continue;
			};
			slides.push(self.read_slide(slides.len() + 1, &part, area)?);
		}
		// what the presentation relates but never lists is a deleted slide
		let mut unlisted = parts
			.values()
			.filter_map(|part| part.path(&self.file).map(str::to_string))
			.collect::<Vec<_>>();
		unlisted.sort();

		let core = self.core_properties()?;
		let mut out = format!(
			"<header><p>{} slides, {:.2} x {:.2} inches. {}</p></header>\n",
			slides.len(),
			width as f64 / 914_400.,
			height as f64 / 914_400.,
			html::text(&core.sentence())
		);
		let mut seen = HashMap::<String, usize>::default();
		for slide in &mut slides {
			let key = slide.duplicate_key();
			if !key.is_empty() {
				slide.duplicate_of = seen.get(&key).copied();
				seen.entry(key).or_insert(slide.index);
			}
			slide.write(&mut out);
		}
		Self::write_report(&mut out, &slides, &unlisted);
		out.xok()
	}

	/// One slide read: its shapes in reading order, its pictures and notes.
	fn read_slide(
		&self,
		index: usize,
		part: &SlidePart,
		area: i64,
	) -> Result<SlideRead> {
		let tree = Self::tree(part.try_data(&self.file)?)?;
		let related = part
			.parts(&self.file)
			.map(|pair| (pair.relationship_id.to_string(), pair.part))
			.collect::<HashMap<_, _>>();
		let links = part
			.hyperlink_relationships(&self.file)
			.map(|link| (link.id().to_string(), link.target().to_string()))
			.collect::<HashMap<_, _>>();
		let layout = part
			.slide_layout_part(&self.file)
			.map(|layout| Self::tree(layout.try_data(&self.file)?))
			.transpose()?;
		let mut slide = SlideRead {
			index,
			part: part.path(&self.file).unwrap_or_default().to_string(),
			layout: layout
				.as_ref()
				.and_then(|layout| layout.root.child(Ns::PRESENTATION, "cSld"))
				.and_then(|common| common.attribute(None, "name"))
				.unwrap_or_default()
				.to_string(),
			hidden: tree.root.attribute(None, "show") == Some("0"),
			..Default::default()
		};
		let reader = ShapeReader {
			deck: self,
			related: &related,
			links: &links,
			layout: layout.as_ref(),
			area,
		};
		if let Some(shapes) = tree
			.root
			.child(Ns::PRESENTATION, "cSld")
			.and_then(|common| common.child(Ns::PRESENTATION, "spTree"))
		{
			for shape in reader.walk(shapes) {
				reader.read(shape, &mut slide)?;
			}
		}
		if let Some(notes) = part.notes_slide_part(&self.file) {
			slide.notes =
				Self::notes(&Self::tree(notes.try_data(&self.file)?)?);
		}
		slide.xok()
	}

	fn tree(data: Option<&[u8]>) -> Result<XmlTree> {
		XmlTree::parse(data.ok_or_else(|| bevyhow!("a slide part is empty"))?)
	}

	/// The speaker notes, the body placeholder's text, or nothing when it
	/// holds only the deck's `Script:` and `Go to Next Slide >` stubs.
	fn notes(tree: &XmlTree) -> String {
		let body = tree
			.root
			.descendants_named(Ns::PRESENTATION, "sp")
			.into_iter()
			.find(|shape| {
				placeholder(shape).is_some_and(|placeholder| {
					placeholder.attribute(None, "type") == Some("body")
				})
			})
			.and_then(|shape| shape.child(Ns::PRESENTATION, "txBody"));
		let text = body
			.map(|body| {
				body.children_named(Ns::DRAWING, "p")
					.map(paragraph_text)
					.collect::<Vec<_>>()
					.join("\n")
			})
			.unwrap_or_default()
			.trim()
			.to_string();
		let mut stub = text.replace("Script:", "");
		while let Some(start) = stub.find("Go to Next Slide") {
			let rest = stub[start + "Go to Next Slide".len()..].trim_start();
			match rest.strip_prefix('>') {
				Some(after) => stub = format!("{}{after}", &stub[..start]),
				None => break,
			}
		}
		match stub.trim().is_empty() {
			true => String::new(),
			false => text,
		}
	}

	fn write_report(
		out: &mut String,
		slides: &[SlideRead],
		unlisted: &[String],
	) {
		out.push_str("<section>\n<h2>Report</h2>\n");
		let paragraph = |out: &mut String, text: String| {
			out.push_str(&format!("<p>{}</p>\n", html::text(&text)));
		};
		let hidden = slides
			.iter()
			.filter(|slide| slide.hidden)
			.map(|slide| slide.index.to_string())
			.collect::<Vec<_>>();
		if !hidden.is_empty() {
			paragraph(
				out,
				format!(
					"hidden slide(s) {}: in the deck and in this dump, but not shown \
				 in a slideshow and dropped from a render. Transcribe from the dump \
				 and say it is hidden.",
					hidden.join(", ")
				),
			);
		}
		if !unlisted.is_empty() {
			paragraph(
				out,
				format!(
					"{} slide part(s) the presentation never shows ({}): a deleted \
				 slide's leftovers, not a slide. Mention once, transcribe nothing.",
					unlisted.len(),
					unlisted
						.iter()
						.map(|path| path.rsplit('/').next().unwrap_or(path))
						.collect::<Vec<_>>()
						.join(", ")
				),
			);
		}
		let blank_zero = |count: usize| match count {
			0 => String::new(),
			count => count.to_string(),
		};
		let mut rows = vec![
			[
				"slide",
				"part",
				"words",
				"pics",
				"largest",
				"tables",
				"notes",
				"carried by",
			]
			.map(str::to_string)
			.to_vec(),
		];
		for slide in slides {
			rows.push(vec![
				slide.index.to_string(),
				slide
					.part
					.rsplit('/')
					.next()
					.unwrap_or_default()
					.to_string(),
				slide.words.to_string(),
				slide.pictures.len().to_string(),
				format!("{}%", slide.largest()),
				blank_zero(slide.tables),
				blank_zero(slide.notes.split_whitespace().count()),
				match slide.duplicate_of {
					Some(first) => format!("duplicate of slide {first}"),
					None => slide.carried_by().to_string(),
				},
			]);
		}
		out.push_str(&html::table(&rows));
		// a picture big enough to carry content, shown on several slides
		let mut recurring = BTreeMap::<&str, Vec<usize>>::new();
		for slide in slides {
			let mut sources = slide
				.pictures
				.iter()
				.filter(|picture| picture.cover >= Self::RECURRING_FLOOR)
				.map(|picture| picture.source.as_str())
				.collect::<Vec<_>>();
			sources.sort();
			sources.dedup();
			for source in sources {
				recurring.entry(source).or_default().push(slide.index);
			}
		}
		for (source, on) in recurring.iter().filter(|(_, on)| on.len() >= 2) {
			let span = match on.windows(2).all(|pair| pair[1] == pair[0] + 1) {
				true => format!("{}-{}", on[0], on[on.len() - 1]),
				false => on
					.iter()
					.map(ToString::to_string)
					.collect::<Vec<_>>()
					.join(", "),
			};
			paragraph(
				out,
				format!(
					"{source} is on {} slides ({span}): the same picture twice is \
				 one artefact re-shown, often with a different box drawn on it. \
				 Transcribe it once and cross-reference.",
					on.len()
				),
			);
		}
		// one capture saved twice: two part names of identical dimensions
		let mut by_size = BTreeMap::<&str, BTreeMap<&str, Vec<usize>>>::new();
		for slide in slides {
			for picture in slide.pictures.iter().filter(|picture| {
				picture.cover >= Self::RECURRING_FLOOR
					&& picture.cover < 90
					&& !picture.size.is_empty()
			}) {
				by_size
					.entry(&picture.size)
					.or_default()
					.entry(&picture.source)
					.or_default()
					.push(slide.index);
			}
		}
		for (size, names) in by_size
			.iter()
			.filter(|(_, names)| (2..=3).contains(&names.len()))
		{
			let at = names
				.iter()
				.map(|(name, on)| {
					format!(
						"{name} on {}",
						on.iter()
							.map(ToString::to_string)
							.collect::<Vec<_>>()
							.join(", ")
					)
				})
				.collect::<Vec<_>>()
				.join("; ");
			paragraph(
				out,
				format!(
					"{} pictures of the same size, {size} ({at}): often one capture \
				 saved twice. Compare them before transcribing both.",
					names.len()
				),
			);
		}
		let pictured = slides
			.iter()
			.filter(|slide| {
				slide.duplicate_of.is_none() && slide.carried_by() == "picture"
			})
			.map(|slide| slide.index.to_string())
			.collect::<Vec<_>>();
		if !pictured.is_empty() {
			paragraph(
				out,
				format!(
					"read and transcribe these slides' pictures: {}",
					pictured.join(", ")
				),
			);
		}
		let duplicates = slides
			.iter()
			.filter_map(|slide| {
				slide
					.duplicate_of
					.map(|first| format!("{}=={first}", slide.index))
			})
			.collect::<Vec<_>>();
		if !duplicates.is_empty() {
			paragraph(
				out,
				format!(
					"duplicate slides (say so once, do not transcribe twice): {}",
					duplicates.join(", ")
				),
			);
		}
		out.push_str("</section>\n");
	}
}

/// What one slide holds, as the HTML and the report read it.
#[derive(Default)]
struct SlideRead {
	index: usize,
	part: String,
	layout: String,
	hidden: bool,
	/// The HTML blocks of the slide's shapes, in reading order.
	body: Vec<String>,
	words: usize,
	tables: usize,
	pictures: Vec<Picture>,
	notes: String,
	duplicate_of: Option<usize>,
}

/// One image a shape carries.
struct Picture {
	/// The image part's path, or why there is none.
	source: String,
	/// Its native pixel size, `vector` for a metafile, empty when unknown.
	size: String,
	/// How much of the slide the shape covers, in percent.
	cover: u32,
}

impl SlideRead {
	fn write(&self, out: &mut String) {
		let hidden = match self.hidden {
			true => " (hidden)",
			false => "",
		};
		out.push_str(&format!(
			"<section>\n<h2>Slide {}{hidden}</h2>\n<p><code>{}</code>, layout <code>{}</code></p>\n",
			self.index,
			html::text(&self.part),
			html::text(&self.layout)
		));
		match self.body.is_empty() {
			true => out.push_str("<p><em>no text</em></p>\n"),
			false => {
				for block in &self.body {
					out.push_str(block);
				}
			}
		}
		if !self.pictures.is_empty() {
			out.push_str("<h3>Pictures</h3>\n");
			let mut rows = vec![
				["#", "source", "native px", "covers"]
					.map(str::to_string)
					.to_vec(),
			];
			for (number, picture) in self.pictures.iter().enumerate() {
				rows.push(vec![
					(number + 1).to_string(),
					picture.source.clone(),
					picture.size.clone(),
					format!("{}%", picture.cover),
				]);
			}
			out.push_str(&html::table(&rows));
		}
		if !self.notes.is_empty() {
			out.push_str("<h3>Speaker notes</h3>\n");
			for paragraph in self
				.notes
				.split('\n')
				.filter(|line| !line.trim().is_empty())
			{
				out.push_str(&format!(
					"<p>{}</p>\n",
					html::text(paragraph.trim())
				));
			}
		}
		out.push_str("</section>\n");
	}

	/// The same text and the same pictures is the same slide.
	fn duplicate_key(&self) -> String {
		let text = self
			.body
			.join(" ")
			.split_whitespace()
			.collect::<Vec<_>>()
			.join(" ");
		let mut pictures = self
			.pictures
			.iter()
			.map(|picture| format!("{}@{}", picture.source, picture.size))
			.collect::<Vec<_>>();
		pictures.sort();
		match text.is_empty() && pictures.is_empty() {
			true => String::new(),
			false => format!("{text}|{}", pictures.join("|")),
		}
	}

	fn largest(&self) -> u32 {
		self.pictures
			.iter()
			.map(|picture| picture.cover)
			.max()
			.unwrap_or(0)
	}

	/// Which half of the slide holds the content: a picture covering the
	/// whole slide under real text is a background photo, one covering half
	/// of it is a diagram or a screenshot to be read.
	fn carried_by(&self) -> &'static str {
		let total = self
			.pictures
			.iter()
			.map(|picture| picture.cover)
			.sum::<u32>();
		match (self.words, self.pictures.len(), self.largest()) {
			(0, 0, _) => "blank",
			(words, _, _) if words < 25 => "picture",
			(_, _, largest) if largest >= 90 => "text over photo",
			(words, _, largest)
				if largest >= 40 || (total >= 25 && words < 40) =>
			{
				"picture"
			}
			_ => "text",
		}
	}
}

/// Reads one slide's shapes against its relationships and layout.
struct ShapeReader<'a> {
	deck: &'a SlideDeck,
	/// The slide's related parts by relationship id.
	related: &'a HashMap<String, PartRef>,
	/// The slide's hyperlink targets by relationship id.
	links: &'a HashMap<String, String>,
	/// The slide's layout, which a placeholder takes its position from.
	layout: Option<&'a XmlTree>,
	/// The slide's area in EMU squared.
	area: i64,
}

impl ShapeReader<'_> {
	/// Every shape under `tree` in reading order, top to bottom then left to
	/// right, a group flattened into its members at its own position.
	fn walk<'b>(&self, tree: &'b XmlElement) -> Vec<&'b XmlElement> {
		let mut shapes = tree
			.elements()
			.flat_map(|element| {
				match element.is(Ns::COMPATIBILITY, "AlternateContent") {
					// the choice is what an Office reader shows
					true => element
						.child(Ns::COMPATIBILITY, "Choice")
						.map(|choice| choice.elements().collect::<Vec<_>>())
						.unwrap_or_default(),
					false => vec![element],
				}
			})
			.filter(|element| {
				element.namespace.as_deref() == Some(Ns::PRESENTATION)
					&& ["sp", "grpSp", "graphicFrame", "pic", "cxnSp"]
						.contains(&element.local_name())
			})
			.collect::<Vec<_>>();
		shapes.sort_by_key(|shape| {
			let (x, y) = self.frame(shape).0;
			(y, x)
		});
		shapes
			.into_iter()
			.flat_map(|shape| match shape.is(Ns::PRESENTATION, "grpSp") {
				true => self.walk(shape),
				false => vec![shape],
			})
			.collect()
	}

	fn read(&self, shape: &XmlElement, slide: &mut SlideRead) -> Result {
		if let Some(body) = shape.child(Ns::PRESENTATION, "txBody") {
			let text = body
				.children_named(Ns::DRAWING, "p")
				.map(paragraph_text)
				.collect::<Vec<_>>()
				.join("\n");
			if !text.trim().is_empty() {
				let kind = match placeholder(shape) {
					Some(placeholder) => {
						placeholder.attribute(None, "type").unwrap_or("obj")
					}
					None => "text",
				};
				slide.words += text.split_whitespace().count();
				slide
					.body
					.push(format!("<p><strong>[{kind}]</strong></p>\n"));
				slide.body.push(self.text_blocks(body));
			}
		}
		for table in shape.descendants_named(Ns::DRAWING, "tbl") {
			slide.tables += 1;
			let rows =
				table.children_named(Ns::DRAWING, "tr").collect::<Vec<_>>();
			let columns =
				table.child(Ns::DRAWING, "tblGrid").map_or(0, |grid| {
					grid.children_named(Ns::DRAWING, "gridCol").count()
				});
			slide.body.push(format!(
				"<p><strong>[table {}]</strong> {} rows x {columns} cols</p>\n",
				slide.tables,
				rows.len()
			));
			slide.body.push(html::table(
				&rows
					.iter()
					.map(|row| {
						row.children_named(Ns::DRAWING, "tc")
							.map(|cell| cell_lines(cell).join("\n"))
							.collect::<Vec<_>>()
					})
					.collect::<Vec<_>>(),
			));
			slide.words += rows
				.iter()
				.flat_map(|row| row.children_named(Ns::DRAWING, "tc"))
				.map(|cell| {
					cell_lines(cell).join(" ").split_whitespace().count()
				})
				.sum::<usize>();
		}
		let smartart = self.smartart(shape)?;
		if !smartart.is_empty() {
			slide.words += smartart
				.iter()
				.map(|line| line.split_whitespace().count())
				.sum::<usize>();
			slide
				.body
				.push("<p><strong>[smartart]</strong></p>\n".into());
			for line in smartart {
				slide.body.push(format!("<p>{}</p>\n", html::text(&line)));
			}
		}
		let cover = self.cover(shape);
		for blip in shape.descendants_named(Ns::DRAWING, "blip") {
			let Some(id) = blip.attribute(Some(Ns::RELATIONSHIPS), "embed")
			else {
				slide.pictures.push(Picture {
					source: "linked or unreadable".into(),
					size: String::new(),
					cover,
				});
				continue;
			};
			slide.pictures.push(self.picture(id, cover)?);
		}
		Ok(())
	}

	/// A text frame's paragraphs as HTML: a nested level as a nested list
	/// item, a soft break as a line break, a hyperlink kept.
	fn text_blocks(&self, body: &XmlElement) -> String {
		let mut out = String::new();
		let mut lists = 0usize;
		for paragraph in body.children_named(Ns::DRAWING, "p") {
			let level = paragraph
				.child(Ns::DRAWING, "pPr")
				.and_then(|properties| properties.attribute(None, "lvl"))
				.and_then(|level| level.parse::<usize>().ok())
				.unwrap_or(0);
			let mut lines = vec![String::new()];
			for element in paragraph.elements() {
				let line = lines.last_mut().unwrap();
				match element.local_name() {
					"br" => lines.push(String::new()),
					"fld" => line.push_str(&html::text(
						&element.text_of(Ns::DRAWING, "t"),
					)),
					"r" => {
						let run =
							html::text(&element.text_of(Ns::DRAWING, "t"));
						match element
							.child(Ns::DRAWING, "rPr")
							.and_then(|properties| {
								properties.child(Ns::DRAWING, "hlinkClick")
							})
							.and_then(|click| {
								click.attribute(Some(Ns::RELATIONSHIPS), "id")
							})
							.and_then(|id| self.links.get(id))
						{
							Some(link) => line.push_str(&format!(
								"<a href=\"{}\">{run}</a>",
								html::attribute(link)
							)),
							None => line.push_str(&run),
						}
					}
					_ => {}
				}
			}
			let lines = lines
				.iter()
				.map(|line| line.trim())
				.filter(|line| !line.is_empty())
				.collect::<Vec<_>>();
			if lines.is_empty() {
				continue;
			}
			while lists > level {
				out.push_str("</ul>\n");
				lists -= 1;
			}
			while lists < level {
				out.push_str("<ul>\n");
				lists += 1;
			}
			let tag = match level {
				0 => "p",
				_ => "li",
			};
			out.push_str(&format!("<{tag}>{}</{tag}>\n", lines.join("<br>")));
		}
		for _ in 0..lists {
			out.push_str("</ul>\n");
		}
		out
	}

	/// The text of any SmartArt the shape carries, in the diagram's own
	/// order: none of it is in the slide's shape tree.
	fn smartart(&self, shape: &XmlElement) -> Result<Vec<String>> {
		let mut lines: Vec<String> = Vec::new();
		for ids in shape.descendants_named(Ns::DIAGRAM, "relIds") {
			let Some(PartRef::DiagramDataPart(data)) = ids
				.attribute(Some(Ns::RELATIONSHIPS), "dm")
				.and_then(|id| self.related.get(id))
			else {
				continue;
			};
			let Some(bytes) = data.try_data(&self.deck.file)? else {
				continue;
			};
			for text in XmlTree::parse(bytes)?
				.root
				.descendants_named(Ns::DRAWING, "t")
			{
				let text = text.text().trim().to_string();
				if !text.is_empty() && lines.last() != Some(&text) {
					lines.push(text);
				}
			}
		}
		lines.xok()
	}

	fn picture(&self, id: &str, cover: u32) -> Result<Picture> {
		let Some(PartRef::ImagePart(image)) = self.related.get(id) else {
			return Picture {
				source: "linked or unreadable".into(),
				size: String::new(),
				cover,
			}
			.xok();
		};
		let source = image
			.path(&self.deck.file)
			.unwrap_or_default()
			.trim_start_matches('/')
			.to_string();
		let size = match image.try_data(&self.deck.file)?.and_then(image_size) {
			Some((width, height)) => format!("{width}x{height}"),
			None if source.ends_with(".emf") || source.ends_with(".wmf") => {
				"vector".into()
			}
			None => String::new(),
		};
		Picture {
			source,
			size,
			cover,
		}
		.xok()
	}

	/// How much of the slide the shape covers, in percent.
	fn cover(&self, shape: &XmlElement) -> u32 {
		let (cx, cy) = self.frame(shape).1;
		((100 * cx * cy) as f64 / self.area as f64).round() as u32
	}

	/// The shape's offset and extent in EMU, a placeholder without its own
	/// taking its layout placeholder's.
	fn frame(&self, shape: &XmlElement) -> ((i64, i64), (i64, i64)) {
		own_frame(shape)
			.or_else(|| {
				let declared = placeholder(shape)?;
				let layout = self.layout?;
				let index = declared.attribute(None, "idx");
				let kind = declared.attribute(None, "type");
				layout
					.root
					.descendants()
					.into_iter()
					.filter(|candidate| own_frame(candidate).is_some())
					.find(|candidate| {
						placeholder(candidate).is_some_and(
							|other| match index {
								Some(index) => {
									other.attribute(None, "idx") == Some(index)
								}
								None => other.attribute(None, "type") == kind,
							},
						)
					})
					.and_then(own_frame)
			})
			.unwrap_or_default()
	}
}

/// A shape's own `a:off` and `a:ext`, from its shape, group or frame
/// properties.
fn own_frame(shape: &XmlElement) -> Option<((i64, i64), (i64, i64))> {
	let transform = ["spPr", "grpSpPr"]
		.iter()
		.find_map(|local| shape.child(Ns::PRESENTATION, local))
		.and_then(|properties| properties.child(Ns::DRAWING, "xfrm"))
		.or_else(|| shape.child(Ns::PRESENTATION, "xfrm"))?;
	let pair = |local: &str, first: &str, second: &str| {
		let element = transform.child(Ns::DRAWING, local)?;
		Some((
			element.attribute(None, first)?.parse::<i64>().ok()?,
			element.attribute(None, second)?.parse::<i64>().ok()?,
		))
	};
	Some((pair("off", "x", "y")?, pair("ext", "cx", "cy")?))
}

/// A shape's placeholder declaration, `p:ph` under its non-visual
/// properties.
fn placeholder(shape: &XmlElement) -> Option<&XmlElement> {
	shape
		.elements()
		.find(|element| element.local_name().starts_with("nv"))?
		.child(Ns::PRESENTATION, "nvPr")?
		.child(Ns::PRESENTATION, "ph")
}

/// A paragraph's text, a soft break as a newline and a field as its text.
fn paragraph_text(paragraph: &XmlElement) -> String {
	paragraph
		.elements()
		.map(|element| match element.local_name() {
			"br" => "\n".to_string(),
			"r" | "fld" => element.text_of(Ns::DRAWING, "t"),
			_ => String::new(),
		})
		.collect()
}

/// A table cell's lines, whitespace collapsed and empty ones dropped.
fn cell_lines(cell: &XmlElement) -> Vec<String> {
	cell.child(Ns::DRAWING, "txBody")
		.map(|body| {
			body.children_named(Ns::DRAWING, "p")
				.map(paragraph_text)
				.collect::<Vec<_>>()
				.join("\n")
		})
		.unwrap_or_default()
		.lines()
		.map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
		.filter(|line| !line.is_empty())
		.collect()
}

/// An image's pixel size from its header: PNG, JPEG, GIF or BMP.
fn image_size(bytes: &[u8]) -> Option<(u32, u32)> {
	let be16 = |at: usize| {
		Some(u16::from_be_bytes(bytes.get(at..at + 2)?.try_into().ok()?) as u32)
	};
	let be32 = |at: usize| {
		Some(u32::from_be_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
	};
	let le16 = |at: usize| {
		Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?) as u32)
	};
	let le32 = |at: usize| {
		Some(
			i32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?)
				.unsigned_abs(),
		)
	};
	if bytes.starts_with(b"\x89PNG") {
		return Some((be32(16)?, be32(20)?));
	}
	if bytes.starts_with(b"GIF8") {
		return Some((le16(6)?, le16(8)?));
	}
	if bytes.starts_with(b"BM") {
		return Some((le32(18)?, le32(22)?));
	}
	if bytes.starts_with(&[0xFF, 0xD8]) {
		// walk the segments to the frame header
		let mut at = 2;
		while at + 9 < bytes.len() {
			if bytes[at] != 0xFF {
				return None;
			}
			let marker = bytes[at + 1];
			let length = be16(at + 2)? as usize;
			if (0xC0..=0xCF).contains(&marker)
				&& ![0xC4, 0xC8, 0xCC].contains(&marker)
			{
				return Some((be16(at + 7)?, be16(at + 5)?));
			}
			at += 2 + length;
		}
	}
	None
}

#[cfg(test)]
mod test {
	use super::*;

	/// A one-slide deck: a title placeholder, a bulleted body with a nested
	/// level and a two-row table.
	fn deck() -> SlideDeck {
		let (presentation_ns, drawing_ns, relationships_ns) =
			(Ns::PRESENTATION, Ns::DRAWING, Ns::RELATIONSHIPS);
		let mut file = PresentationDocument::create(Default::default());
		let presentation = file.add_presentation_part().unwrap();
		let slide: SlidePart =
			presentation.add_new_part(&mut file, "rIdSlide1").unwrap();
		slide
			.set_data(
				&mut file,
				format!(
					"<p:sld xmlns:p=\"{presentation_ns}\" xmlns:a=\"{drawing_ns}\"><p:cSld><p:spTree>\
					 <p:sp><p:nvSpPr><p:cNvPr id=\"3\" name=\"Body\"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>\
					 <p:spPr><a:xfrm><a:off x=\"0\" y=\"2000\"/><a:ext cx=\"100\" cy=\"100\"/></a:xfrm></p:spPr>\
					 <p:txBody><a:p><a:r><a:t>Ask three customers</a:t></a:r></a:p>\
					 <a:p><a:pPr lvl=\"1\"/><a:r><a:t>in person</a:t></a:r></a:p></p:txBody></p:sp>\
					 <p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"Title\"/><p:cNvSpPr/><p:nvPr><p:ph type=\"title\"/></p:nvPr></p:nvSpPr>\
					 <p:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"100\" cy=\"100\"/></a:xfrm></p:spPr>\
					 <p:txBody><a:p><a:r><a:t>Market research</a:t></a:r></a:p></p:txBody></p:sp>\
					 <p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id=\"4\" name=\"Table\"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>\
					 <p:xfrm><a:off x=\"0\" y=\"4000\"/><a:ext cx=\"100\" cy=\"100\"/></p:xfrm>\
					 <a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/table\"><a:tbl>\
					 <a:tblGrid><a:gridCol w=\"10\"/><a:gridCol w=\"10\"/></a:tblGrid>\
					 <a:tr h=\"10\"><a:tc><a:txBody><a:p><a:r><a:t>Channel</a:t></a:r></a:p></a:txBody></a:tc>\
					 <a:tc><a:txBody><a:p><a:r><a:t>Cost</a:t></a:r></a:p></a:txBody></a:tc></a:tr>\
					 <a:tr h=\"10\"><a:tc><a:txBody><a:p><a:r><a:t>Stall</a:t></a:r></a:p></a:txBody></a:tc>\
					 <a:tc><a:txBody><a:p><a:r><a:t>a|b</a:t></a:r></a:p></a:txBody></a:tc></a:tr>\
					 </a:tbl></a:graphicData></a:graphic></p:graphicFrame>\
					 </p:spTree></p:cSld></p:sld>"
				),
			)
			.unwrap();
		presentation
			.set_data(
				&mut file,
				format!(
					"<p:presentation xmlns:p=\"{presentation_ns}\" xmlns:r=\"{relationships_ns}\">\
					 <p:sldIdLst><p:sldId id=\"256\" r:id=\"rIdSlide1\"/></p:sldIdLst>\
					 <p:sldSz cx=\"12192000\" cy=\"6858000\"/><p:notesSz cx=\"6858000\" cy=\"9144000\"/>\
					 </p:presentation>"
				),
			)
			.unwrap();
		SlideDeck::from_bytes(file.to_package_bytes().unwrap()).unwrap()
	}

	#[beet_core::test]
	fn transcodes_in_reading_order() {
		deck()
			.to_html()
			.unwrap()
			.xpect_contains("<header><p>1 slides, 13.33 x 7.50 inches.")
			// the title sits above the body, so it reads first
			.xpect_contains(
				"<p><strong>[title]</strong></p>\n<p>Market research</p>\n<p><strong>[text]</strong></p>\n<p>Ask three customers</p>\n<ul>\n<li>in person</li>\n</ul>\n",
			)
			.xpect_contains(
				"<p><strong>[table 1]</strong> 2 rows x 2 cols</p>\n<table>\n<tr><th>Channel</th><th>Cost</th></tr>\n<tr><td>Stall</td><td>a|b</td></tr>\n</table>\n",
			)
			.xpect_contains("<tr><td>1</td><td>slide1.xml</td><td>11</td><td>0</td><td>0%</td><td>1</td><td></td><td>picture</td></tr>");
	}
}
