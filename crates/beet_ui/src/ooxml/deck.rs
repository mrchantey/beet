//! A slide deck read as the one tree: its slides as `<section>`s in
//! presentation order, each slide's shapes in reading order.
use super::ooxml_query::*;
use crate::prelude::*;
use beet_core::prelude::*;
use ooxmlsdk::parts::PartRef;

type Ns = OoxmlNamespace;

/// A deck as its tree's root carries it: its slide size in EMU, and the slide
/// parts its presentation relates but never lists, a deleted slide's
/// leftovers.
#[derive(Debug, Default, Clone, PartialEq, Eq, Reflect, Component)]
#[reflect(Component, Default)]
pub struct Deck {
	/// The slide width in EMU, 914400 to the inch.
	pub width: i64,
	/// The slide height in EMU.
	pub height: i64,
	/// The paths of the slide parts no slide list names.
	pub unlisted: Vec<SmolStr>,
}

/// One slide as its `<section>` carries it: where it is in the deck and its
/// part, its layout, and what holds its content, for a triage of the deck.
#[derive(Debug, Default, Clone, PartialEq, Eq, Reflect, Component)]
#[reflect(Component, Default)]
pub struct Slide {
	/// Its place in presentation order, from 1.
	pub index: u32,
	/// Its part, ie `ppt/slides/slide3.xml`.
	pub part: SmolStr,
	/// Its layout's name, ie `Title Slide`.
	pub layout: SmolStr,
	/// Whether a slideshow skips it.
	pub hidden: bool,
	/// The words of its text frames, tables and SmartArt.
	pub words: usize,
	/// How many tables it holds.
	pub tables: usize,
	/// The words of its speaker notes.
	pub notes_words: usize,
	/// Its words and pictures, which two copies of one slide share.
	pub fingerprint: String,
}

/// A picture on a slide, beside its `<img>`: its native size, `vector` for
/// a metafile and empty when unknown, and how much of the slide its shape
/// covers, in percent.
#[derive(Debug, Default, Clone, PartialEq, Eq, Reflect, Component)]
#[reflect(Component, Default)]
pub struct SlidePicture {
	/// The native pixel size, ie `1920x1080`.
	pub size: SmolStr,
	/// The share of the slide the picture's shape covers, in percent.
	pub cover: u32,
}

/// Reads one slide part, its notes and its SmartArt into the one tree.
pub(crate) struct SlideProjection {
	index: u32,
	/// The slide's area in EMU squared.
	area: i64,
	/// The layout's placeholders, each its `idx`, its `type` and its frame.
	placeholders: Vec<(Option<SmolStr>, Option<SmolStr>, Frame)>,
	layout: SmolStr,
	links: HashMap<SmolStr, SmolStr>,
	/// The slide's pictures by relationship id: path and native size.
	pictures: HashMap<SmolStr, (SmolStr, SmolStr)>,
}

/// A shape's offset and extent in EMU.
type Frame = ((i64, i64), (i64, i64));

/// The changes read off one slide, beside those applied directly.
#[derive(Default)]
struct SlideRead {
	changes: Vec<Projected>,
	/// Each shape container's children in reading order.
	orders: Vec<(Entity, Vec<Entity>)>,
	/// Projection-only labels, each its parent, its position and its words.
	labels: Vec<(Entity, usize, String, String)>,
	/// The SmartArt data parts to read, each with the frame showing it.
	diagrams: Vec<(Entity, SmolStr)>,
	slide: Slide,
	section: Option<Entity>,
}

impl SlideProjection {
	/// The shape kinds a slide's tree holds.
	const SHAPES: &[&str] = &["sp", "grpSp", "graphicFrame", "pic", "cxnSp"];

	/// Reads a deck under `root`: every listed slide in presentation order
	/// a part read and projected, its notes and SmartArt with it.
	pub(crate) fn build(
		world: &mut World,
		root: Entity,
		file: &OoxmlFile,
	) -> Result {
		let presentation = file
			.parts()
			.into_iter()
			.find(|part| matches!(part, PartRef::PresentationPart(_)))
			.ok_or_else(|| bevyhow!("the deck has no presentation part"))?;
		let nodes = file
			.data(&presentation)?
			.map(OoxmlFile::read_xml)
			.transpose()?
			.unwrap_or_default();
		let document = BsxNode::document_element(&nodes);
		let size = document
			.and_then(|document| document.child("sldSz"))
			.and_then(|size| {
				Some((
					size.attribute("cx")?.parse::<i64>().ok()?,
					size.attribute("cy")?.parse::<i64>().ok()?,
				))
			})
			.unwrap_or((9_144_000, 6_858_000));
		let listed = document
			.and_then(|document| document.child("sldIdLst"))
			.map(|list| {
				list.children_named("sldId")
					.filter_map(|slide| slide.prefixed_attribute("id"))
					.map(SmolStr::new)
					.collect::<Vec<_>>()
			})
			.unwrap_or_default();
		let related = file.related(&presentation);
		let mut index = 0;
		for id in &listed {
			let Some((_, part)) = related.iter().find(|(related, part)| {
				related == id && matches!(part, PartRef::SlidePart(_))
			}) else {
				continue;
			};
			index += 1;
			Self::read_slide(world, root, file, part, index, size)?;
		}
		let mut unlisted = related
			.iter()
			.filter(|(id, part)| {
				matches!(part, PartRef::SlidePart(_)) && !listed.contains(id)
			})
			.filter_map(|(_, part)| file.path(part))
			.collect::<Vec<_>>();
		unlisted.sort();
		world.entity_mut(root).insert(Deck {
			width: size.0,
			height: size.1,
			unlisted,
		});
		Ok(())
	}

	fn read_slide(
		world: &mut World,
		root: Entity,
		file: &OoxmlFile,
		part: &PartRef,
		index: u32,
		size: (i64, i64),
	) -> Result {
		let entity = OoxmlParser::spawn_part(world, root, file, part)?;
		let related = file.related(part);
		let layout = related
			.iter()
			.find(|(_, part)| matches!(part, PartRef::SlideLayoutPart(_)))
			.map(|(_, part)| file.data(part))
			.transpose()?
			.flatten()
			.map(OoxmlFile::read_xml)
			.transpose()?
			.unwrap_or_default();
		let mut pictures = HashMap::default();
		for (id, image) in related
			.iter()
			.filter(|(_, part)| matches!(part, PartRef::ImagePart(_)))
		{
			let path = file.path(image).unwrap_or_default();
			let size = match file.data(image)?.and_then(Self::image_size) {
				Some((width, height)) => format!("{width}x{height}").into(),
				None if path.ends_with(".emf") || path.ends_with(".wmf") => {
					"vector".into()
				}
				None => SmolStr::default(),
			};
			pictures.insert(id.clone(), (path, size));
		}
		let projection = Self {
			index,
			area: (size.0 * size.1).max(1),
			placeholders: Self::placeholders(&layout),
			layout: BsxNode::document_element(&layout)
				.and_then(|layout| layout.child("cSld"))
				.and_then(|common| common.attribute("name"))
				.map(SmolStr::new)
				.unwrap_or_default(),
			links: file.hyperlinks(part),
			pictures,
		};
		let mut read = world
			.with_state::<OoxmlQuery, _>(|tree| projection.read(&tree, entity));
		read.slide.part = file.path(part).unwrap_or_default();
		Projected::apply(world, core::mem::take(&mut read.changes));
		for (container, order) in &read.orders {
			world.entity_mut(*container).replace_children(order);
		}
		for (parent, position, element, words) in read.labels.iter().rev() {
			let label = world
				.spawn((Element::new("p"), children![(
					Element::new(element.as_str()),
					children![Value::Str(words.into())]
				)]))
				.id();
			world.entity_mut(*parent).insert_child(*position, label);
		}
		for (frame, id) in &read.diagrams {
			let Some((_, diagram)) =
				related.iter().find(|(related, _)| related == id)
			else {
				continue;
			};
			let data = OoxmlParser::spawn_part(world, *frame, file, diagram)?;
			let (changes, words) = world
				.with_state::<OoxmlQuery, _>(|tree| Self::diagram(&tree, data));
			Projected::apply(world, changes);
			read.slide.words += words;
		}
		if let Some(notes) = related
			.iter()
			.find(|(_, part)| matches!(part, PartRef::NotesSlidePart(_)))
		{
			let section = read.section.unwrap_or(entity);
			let notes =
				OoxmlParser::spawn_part(world, section, file, &notes.1)?;
			read.slide.notes_words = Self::notes(world, notes);
		}
		if let Some(section) = read.section {
			let hidden = match read.slide.hidden {
				true => " (hidden)",
				false => "",
			};
			let heading = world
				.spawn((Element::new("h2"), children![Value::Str(
					format!("Slide {}{hidden}", projection.index).into()
				)]))
				.id();
			let located = world
				.spawn((Element::new("p"), children![
					(Element::new("code"), children![Value::Str(
						read.slide.part.clone()
					)]),
					Value::str(", layout "),
					(Element::new("code"), children![Value::Str(
						projection.layout.clone()
					)]),
				]))
				.id();
			world
				.entity_mut(section)
				.insert_children(0, &[heading, located])
				.insert(read.slide);
		}
		ListLevel::wrap(world, entity);
		Ok(())
	}

	fn read(&self, tree: &OoxmlQuery, part: Entity) -> SlideRead {
		let mut read = SlideRead::default();
		read.slide.index = self.index;
		read.slide.layout = self.layout.clone();
		let mut texts = Vec::<String>::new();
		let mut pictures = Vec::<String>::new();
		for entity in tree.descendants(part) {
			let Some(element) = tree.element(entity) else {
				if tree.is_text(entity) && !Self::is_read(tree, entity) {
					read.changes.push(Projected::Hide(entity));
				}
				continue;
			};
			let Some(namespace) = element.namespace.as_deref() else {
				continue;
			};
			let tag = |tag: &str| Projected::Tag(entity, Tagged::new(tag));
			match (namespace, element.local_name()) {
				(Ns::PRESENTATION, "sld") => {
					read.slide.hidden =
						tree.attribute(entity, None, "show") == Some("0");
					read.section = Some(entity);
					read.changes.push(tag("section"));
				}
				(Ns::PRESENTATION, "spTree" | "grpSp") => {
					read.orders
						.push((entity, self.reading_order(tree, entity)));
				}
				(Ns::PRESENTATION, "txBody") => {
					let text = Self::frame_text(tree, entity);
					if text.trim().is_empty() {
						continue;
					}
					let Some(shape) = tree.parent(entity) else {
						continue;
					};
					let kind = match Self::placeholder(tree, shape) {
						Some(placeholder) => tree
							.attribute(placeholder, None, "type")
							.unwrap_or("obj")
							.to_string(),
						None => "text".into(),
					};
					read.slide.words += text.split_whitespace().count();
					texts.push(text);
					let position = Self::position(tree, shape, entity);
					read.labels.push((
						shape,
						position,
						"strong".into(),
						format!("[{kind}]"),
					));
				}
				(Ns::DRAWING, "p") => {
					let level = tree
						.child(entity, Ns::DRAWING, "pPr")
						.and_then(|properties| {
							tree.attribute(properties, None, "lvl")
						})
						.and_then(|level| level.parse::<u8>().ok())
						.unwrap_or(0);
					match level {
						0 => read.changes.push(tag("p")),
						level => {
							read.changes.push(tag("li"));
							read.changes.push(Projected::insert(
								entity,
								ListLevel(level - 1),
							));
						}
					}
				}
				(Ns::DRAWING, "br") => read.changes.push(tag("br")),
				(Ns::DRAWING, "r") => {
					if let Some(href) = tree
						.child(entity, Ns::DRAWING, "rPr")
						.and_then(|properties| {
							tree.child(properties, Ns::DRAWING, "hlinkClick")
						})
						.and_then(|click| {
							tree.attribute(click, Some(Ns::RELATIONSHIPS), "id")
						})
						.and_then(|id| self.links.get(id))
					{
						read.changes.push(Projected::Tag(
							entity,
							Tagged::new("a").with("href", href.as_str()),
						));
					}
				}
				(Ns::DRAWING, "tbl") => {
					read.slide.tables += 1;
					read.changes.push(tag("table"));
					let rows = tree.children_named(entity, Ns::DRAWING, "tr");
					let columns = tree
						.child(entity, Ns::DRAWING, "tblGrid")
						.map_or(0, |grid| {
							tree.children_named(grid, Ns::DRAWING, "gridCol")
								.len()
						});
					let words = rows
						.iter()
						.flat_map(|row| {
							tree.children_named(*row, Ns::DRAWING, "tc")
						})
						.map(|cell| tree.text(cell).split_whitespace().count())
						.sum::<usize>();
					read.slide.words += words;
					if let Some(parent) = tree.parent(entity) {
						let position = Self::position(tree, parent, entity);
						read.labels.push((
							parent,
							position,
							"strong".into(),
							format!(
								"[table {}] {} rows x {columns} cols",
								read.slide.tables,
								rows.len()
							),
						));
					}
				}
				(Ns::DRAWING, "tr") => read.changes.push(tag("tr")),
				(Ns::DRAWING, "tc") => read.changes.push(tag("td")),
				(Ns::DIAGRAM, "relIds") => {
					if let (Some(id), Some(frame)) = (
						tree.attribute(entity, Some(Ns::RELATIONSHIPS), "dm"),
						tree.parent(entity),
					) {
						read.diagrams.push((frame, id.into()));
						let position = Self::position(tree, frame, entity);
						read.labels.push((
							frame,
							position,
							"strong".into(),
							"[smartart]".into(),
						));
					}
				}
				(Ns::DRAWING, "blip") => {
					let shape =
						tree.ancestors(entity).into_iter().find(|ancestor| {
							tree.element(*ancestor).is_some_and(|ancestor| {
								ancestor.namespace.as_deref()
									== Some(Ns::PRESENTATION) && Self::SHAPES
									.contains(&ancestor.local_name())
							})
						});
					let cover =
						shape.map_or(0, |shape| self.cover(tree, shape));
					let (source, size) = tree
						.attribute(entity, Some(Ns::RELATIONSHIPS), "embed")
						.and_then(|id| self.pictures.get(id))
						.cloned()
						.unwrap_or_else(|| {
							("linked or unreadable".into(), SmolStr::default())
						});
					pictures.push(format!("{source}@{size}"));
					read.changes.push(Projected::Tag(
						entity,
						Tagged::new("img")
							.with("src", source.as_str())
							.with("alt", ""),
					));
					read.changes.push(Projected::insert(
						entity,
						SlidePicture { size, cover },
					));
				}
				(Ns::COMPATIBILITY, "Fallback") => {
					read.changes.push(tag("template"))
				}
				_ => {}
			}
		}
		let text = texts
			.join(" ")
			.split_whitespace()
			.collect::<Vec<_>>()
			.join(" ");
		pictures.sort();
		read.slide.fingerprint = match text.is_empty() && pictures.is_empty() {
			true => String::new(),
			false => format!("{text}|{}", pictures.join("|")),
		};
		read
	}

	/// The child index of `entity` within `parent`.
	fn position(tree: &OoxmlQuery, parent: Entity, entity: Entity) -> usize {
		tree.children(parent)
			.iter()
			.position(|child| *child == entity)
			.unwrap_or_default()
	}

	/// A container's children with its shapes in reading order, top to bottom
	/// then left to right, after what is no shape, ie its own properties.
	fn reading_order(
		&self,
		tree: &OoxmlQuery,
		container: Entity,
	) -> Vec<Entity> {
		let (mut shapes, rest): (Vec<_>, Vec<_>) =
			tree.children(container).into_iter().partition(|child| {
				tree.element(*child).is_some_and(|element| {
					element.is(Ns::COMPATIBILITY, "AlternateContent")
						|| (element.namespace.as_deref()
							== Some(Ns::PRESENTATION)
							&& Self::SHAPES.contains(&element.local_name()))
				})
			});
		shapes.sort_by_key(|shape| {
			let shape =
				match tree.is(*shape, Ns::COMPATIBILITY, "AlternateContent") {
					// the choice is what an Office reader shows
					true => tree
						.child(*shape, Ns::COMPATIBILITY, "Choice")
						.and_then(|choice| {
							tree.children(choice).into_iter().next()
						})
						.unwrap_or(*shape),
					false => *shape,
				};
			let ((x, y), _) = self.frame(tree, shape);
			(y, x)
		});
		rest.into_iter().chain(shapes).collect()
	}

	/// How much of the slide `shape` covers, in percent.
	fn cover(&self, tree: &OoxmlQuery, shape: Entity) -> u32 {
		let (_, (cx, cy)) = self.frame(tree, shape);
		((100 * cx * cy) as f64 / self.area as f64).round() as u32
	}

	/// A shape's frame, a placeholder without its own taking its layout
	/// placeholder's.
	fn frame(&self, tree: &OoxmlQuery, shape: Entity) -> Frame {
		Self::own_frame(tree, shape)
			.or_else(|| {
				let declared = Self::placeholder(tree, shape)?;
				let index = tree.attribute(declared, None, "idx");
				let kind = tree.attribute(declared, None, "type");
				self.placeholders
					.iter()
					.find(|(other_index, other_kind, _)| match index {
						Some(index) => other_index.as_deref() == Some(index),
						None => other_kind.as_deref() == kind,
					})
					.map(|(.., frame)| *frame)
			})
			.unwrap_or_default()
	}

	/// A shape's own `a:off` and `a:ext`, from its shape, group or frame
	/// properties.
	fn own_frame(tree: &OoxmlQuery, shape: Entity) -> Option<Frame> {
		let transform = ["spPr", "grpSpPr"]
			.iter()
			.find_map(|local| tree.child(shape, Ns::PRESENTATION, local))
			.and_then(|properties| tree.child(properties, Ns::DRAWING, "xfrm"))
			.or_else(|| tree.child(shape, Ns::PRESENTATION, "xfrm"))?;
		let pair = |local: &str, first: &str, second: &str| {
			let element = tree.child(transform, Ns::DRAWING, local)?;
			Some((
				tree.attribute(element, None, first)?.parse::<i64>().ok()?,
				tree.attribute(element, None, second)?.parse::<i64>().ok()?,
			))
		};
		Some((pair("off", "x", "y")?, pair("ext", "cx", "cy")?))
	}

	/// A shape's placeholder declaration, `p:ph` under its non-visual
	/// properties.
	fn placeholder(tree: &OoxmlQuery, shape: Entity) -> Option<Entity> {
		let properties = tree.children(shape).into_iter().find(|child| {
			tree.element(*child)
				.is_some_and(|element| element.local_name().starts_with("nv"))
		})?;
		let visual = tree.child(properties, Ns::PRESENTATION, "nvPr")?;
		tree.child(visual, Ns::PRESENTATION, "ph")
	}

	/// The layout's placeholders that carry their own frame, each its
	/// `idx`, its `type` and its frame.
	fn placeholders(
		layout: &[BsxNode],
	) -> Vec<(Option<SmolStr>, Option<SmolStr>, Frame)> {
		let Some(root) = BsxNode::document_element(layout) else {
			return Vec::new();
		};
		let mut out = Vec::new();
		for shape in ["sp", "pic", "graphicFrame"]
			.iter()
			.flat_map(|local| root.descendants_named(local))
		{
			let declared = shape
				.elements()
				.find(|element| element.local_name().starts_with("nv"))
				.and_then(|properties| properties.child("nvPr"))
				.and_then(|visual| visual.child("ph"));
			let transform = shape
				.child("spPr")
				.and_then(|properties| properties.child("xfrm"))
				.or_else(|| shape.child("xfrm"));
			let pair = |local: &str, first: &str, second: &str| {
				let element = transform?.child(local)?;
				Some((
					element.attribute(first)?.parse::<i64>().ok()?,
					element.attribute(second)?.parse::<i64>().ok()?,
				))
			};
			if let (Some(declared), Some(offset), Some(extent)) =
				(declared, pair("off", "x", "y"), pair("ext", "cx", "cy"))
			{
				out.push((
					declared.attribute("idx").map(SmolStr::new),
					declared.attribute("type").map(SmolStr::new),
					(offset, extent),
				));
			}
		}
		out
	}

	/// A text frame's paragraphs' text, one a line.
	fn frame_text(tree: &OoxmlQuery, body: Entity) -> String {
		tree.children_named(body, Ns::DRAWING, "p")
			.into_iter()
			.map(|paragraph| Self::paragraph_text(tree, paragraph))
			.collect::<Vec<_>>()
			.join("\n")
	}

	/// A paragraph's text, a soft break as a line break and a field as its
	/// text.
	fn paragraph_text(tree: &OoxmlQuery, paragraph: Entity) -> String {
		tree.children(paragraph)
			.into_iter()
			.filter_map(|child| {
				tree.element(child).map(|element| (child, element))
			})
			.map(|(child, element)| match element.local_name() {
				"br" => "\n".to_string(),
				"r" | "fld" => tree.text(child),
				_ => String::new(),
			})
			.collect()
	}

	/// Reads a SmartArt data part read under its frame, its paragraphs
	/// read as any others: the changes, and its words.
	fn diagram(tree: &OoxmlQuery, data: Entity) -> (Vec<Projected>, usize) {
		let mut changes = Vec::new();
		let mut words = 0;
		for entity in tree.descendants(data) {
			match tree.element(entity) {
				Some(element) if element.is(Ns::DRAWING, "p") => {
					words += Self::paragraph_text(tree, entity)
						.split_whitespace()
						.count();
					changes.push(Projected::Tag(entity, Tagged::new("p")));
				}
				Some(element) if element.is(Ns::DRAWING, "br") => {
					changes.push(Projected::Tag(entity, Tagged::new("br")));
				}
				None if tree.is_text(entity)
					&& !Self::is_read(tree, entity) =>
				{
					changes.push(Projected::Hide(entity));
				}
				_ => {}
			}
		}
		(changes, words)
	}

	/// Whether a text node is words a reader reads: an `a:t`'s.
	fn is_read(tree: &OoxmlQuery, text: Entity) -> bool {
		tree.parent(text)
			.and_then(|parent| tree.element(parent))
			.is_some_and(|parent| parent.local_name() == "t")
	}

	/// Projects a slide's notes read under its section: the body
	/// placeholder's paragraphs an `<aside>` headed `Speaker notes`, every
	/// other shape's text, and notes holding only the deck's `Script:` and
	/// `Go to Next Slide >` stubs, kept for the writer alone. Answers the
	/// notes' words.
	fn notes(world: &mut World, notes: Entity) -> usize {
		let (changes, aside, words) =
			world.with_state::<OoxmlQuery, _>(|tree| {
				let body = tree
					.descendants_named(notes, Ns::PRESENTATION, "sp")
					.into_iter()
					.find(|shape| {
						Self::placeholder(&tree, *shape).is_some_and(
							|placeholder| {
								tree.attribute(placeholder, None, "type")
									== Some("body")
							},
						)
					})
					.and_then(|shape| {
						tree.child(shape, Ns::PRESENTATION, "txBody")
					});
				let text = body
					.map(|body| Self::frame_text(&tree, body))
					.unwrap_or_default();
				// what a reader reads: the body's text, unless only stubs
				let shown = body
					.filter(|_| !Self::is_stub(&text))
					.map(|body| {
						tree.descendants(body)
							.into_iter()
							.collect::<HashSet<_>>()
					})
					.unwrap_or_default();
				let mut changes = Vec::new();
				for entity in tree.descendants(notes) {
					let in_body = shown.contains(&entity);
					match tree.element(entity) {
						Some(element)
							if in_body && element.is(Ns::DRAWING, "p") =>
						{
							changes
								.push(Projected::Tag(entity, Tagged::new("p")))
						}
						Some(element)
							if in_body && element.is(Ns::DRAWING, "br") =>
						{
							changes
								.push(Projected::Tag(entity, Tagged::new("br")))
						}
						None if tree.is_text(entity)
							&& !(in_body && Self::is_read(&tree, entity)) =>
						{
							changes.push(Projected::Hide(entity))
						}
						_ => {}
					}
				}
				let aside = (!shown.is_empty())
					.then(|| tree.child(notes, Ns::PRESENTATION, "notes"))
					.flatten();
				let words = match shown.is_empty() {
					true => 0,
					false => text.split_whitespace().count(),
				};
				(changes, aside, words)
			});
		Projected::apply(world, changes);
		if let Some(aside) = aside {
			Tagged::new("aside").insert(world, aside);
			let heading = world
				.spawn((Element::new("h3"), children![Value::str(
					"Speaker notes"
				)]))
				.id();
			world.entity_mut(aside).insert_child(0, heading);
		}
		words
	}

	/// Whether notes hold only the deck's `Script:` and `Go to Next Slide >`
	/// stubs, or nothing.
	fn is_stub(text: &str) -> bool {
		let mut stub = text.replace("Script:", "");
		while let Some(start) = stub.find("Go to Next Slide") {
			let rest = stub[start + "Go to Next Slide".len()..].trim_start();
			match rest.strip_prefix('>') {
				Some(after) => stub = format!("{}{after}", &stub[..start]),
				None => break,
			}
		}
		stub.trim().is_empty()
	}

	/// An image's pixel size from its header: PNG, JPEG, GIF or BMP.
	fn image_size(bytes: &[u8]) -> Option<(u32, u32)> {
		let be16 = |at: usize| {
			Some(u16::from_be_bytes(bytes.get(at..at + 2)?.try_into().ok()?)
				as u32)
		};
		let be32 = |at: usize| {
			Some(u32::from_be_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
		};
		let le16 = |at: usize| {
			Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?)
				as u32)
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
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// A one-slide deck: a bulleted body with a nested level written before
	/// the title it sits below, and a two-row table.
	fn deck() -> MediaBytes {
		OoxmlFile::deck(&[
			"<p:sp><p:nvSpPr><p:cNvPr id=\"3\" name=\"Body\"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>\
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
			 </a:tbl></a:graphicData></a:graphic></p:graphicFrame>",
		])
		.unwrap()
	}

	#[beet_core::test]
	fn reads_slides_in_reading_order() {
		let (mut world, root) = super::super::word::test::parse(deck());
		super::super::word::test::markdown(&mut world, root)
			.xpect_starts_with("1 slides, 13.33 x 7.50 inches.\n\n## Slide 1\n\n`ppt/slides/slide1.xml`, layout ``\n")
			// the title sits above the body, so it reads first
			.xpect_contains(
				"**[title]**\n\nMarket research\n\n**[text]**\n\nAsk three customers\n\n- in person\n",
			)
			.xpect_contains(
				"**[table 1] 2 rows x 2 cols**\n\n| Channel | Cost |\n|---|---|\n| Stall | a\\|b |\n",
			)
			.xpect_contains("| 1 | slide1.xml | 11 | 0 | 0% | 1 |  | picture |");
		// a deck is read, never written
		RenderTargets::render(
			&mut world,
			root,
			&RequestParts::default().with_accept(MediaType::Pptx),
		)
		.xpect_err();
	}
}
