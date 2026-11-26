//! [`DeckParser`], a slide deck read as one `article data-card` per slide,
//! by the [module docs](super)' deck mapping.
use super::office_picture::*;
use super::office_style::*;
use super::read_element::*;
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::math::Affine2;

/// Reads a slide deck into one `article data-card` per listed slide in
/// presentation order, its front laid out as the card stack lays a card
/// out, then an `hr` and its speaker notes, with nothing Office-specific
/// left on any entity. See the [module docs](super) for the mapping and
/// what it loses.
#[derive(Debug, Default, Clone, Get, SetWith)]
pub struct DeckParser {
	/// How the deck's looks are read.
	options: OfficeReadOptions,
}

/// What a shape is to the card's layout, by its placeholder type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
	/// `ctrTitle` or `title`.
	Title,
	/// `subTitle`.
	Subtitle,
	/// `body`, `obj`, or an untyped placeholder.
	Body,
	/// A date, footer, slide number or header placeholder: the layout's
	/// chrome, never content.
	Chrome,
	/// Any other shape: a text box, picture, table or SmartArt, or a
	/// placeholder of another type.
	Other,
}

/// A paragraph's bullet, as its own properties or a list style say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Bullet {
	None,
	Unordered,
	Ordered,
}

/// One shape as read, before the slide is laid out.
#[derive(Debug)]
struct ShapeRead {
	role: Role,
	frame: Rect,
	blocks: Vec<ReadElement>,
}

/// A placeholder a layout or master declares: its `idx` and `type`, its
/// frame and its list style.
#[derive(Debug, Clone)]
struct Placeholder {
	index: Option<SmolStr>,
	kind: Option<SmolStr>,
	frame: Option<Rect>,
	list_style: Option<BsxElement>,
}

/// One slide part being read: the package, the part's path for its
/// relationships, and what its layout and master declare.
struct SlideRead<'a> {
	package: &'a OpcPackage,
	part: String,
	/// The layout's placeholders, then the master's.
	placeholders: Vec<Placeholder>,
	/// The master's body text style, `p:txStyles/p:bodyStyle`.
	body_style: Option<BsxElement>,
}

/// A group's mapping from its children's coordinates, in EMU, to its
/// parent's: an offset and a positive scale, so it maps a [`Rect`]'s
/// corners to the corners of the mapped rect.
#[derive(Debug, Default, Clone, Copy)]
struct GroupTransform(Affine2);

impl GroupTransform {
	/// `frame` in the parent's coordinates.
	fn apply(&self, frame: Rect) -> Rect {
		Rect::from_corners(
			self.0.transform_point2(frame.min),
			self.0.transform_point2(frame.max),
		)
	}

	/// The transform a group's `a:xfrm` makes, nested in `self`: its
	/// `chOff`/`chExt` child space mapped onto its `off`/`ext` frame.
	fn nest(&self, transform: &BsxElement) -> Self {
		let (Some(offset), Some(extent)) = (
			Xfrm::point(transform, "off", "x", "y"),
			Xfrm::point(transform, "ext", "cx", "cy"),
		) else {
			return *self;
		};
		let child_offset =
			Xfrm::point(transform, "chOff", "x", "y").unwrap_or(offset);
		let child_extent =
			Xfrm::point(transform, "chExt", "cx", "cy").unwrap_or(extent);
		// a degenerate child extent maps one to one
		let scale = Vec2::select(
			child_extent.cmpgt(Vec2::ZERO),
			extent / child_extent,
			Vec2::ONE,
		);
		let local = Affine2::from_translation(offset)
			* Affine2::from_scale(scale)
			* Affine2::from_translation(-child_offset);
		Self(self.0 * local)
	}
}

/// Reading an `a:xfrm`, a shape's offset and extent in EMU.
struct Xfrm;

impl Xfrm {
	/// A shape's frame in slide space, from its `a:xfrm`.
	fn frame(transform: &BsxElement) -> Option<Rect> {
		let offset = Self::point(transform, "off", "x", "y")?;
		let extent = Self::point(transform, "ext", "cx", "cy")?;
		Rect::from_corners(offset, offset + extent).xsome()
	}

	/// The `first` and `second` attributes of `transform`'s `local` child.
	fn point(
		transform: &BsxElement,
		local: &str,
		first: &str,
		second: &str,
	) -> Option<Vec2> {
		let element = transform.child(local)?;
		Vec2::new(
			element.attribute(first)?.parse().ok()?,
			element.attribute(second)?.parse().ok()?,
		)
		.xsome()
	}

	/// How far `frame` and `other` overlap, across and down, in EMU.
	fn overlap(frame: &Rect, other: &Rect) -> Vec2 {
		frame.intersect(*other).size()
	}

	/// `frame`'s size in EMU, for a picture's `width` and `height`.
	fn emu(frame: &Rect) -> Option<Vec2> {
		(!frame.is_empty()).then(|| frame.size())
	}
}

impl Placeholder {
	/// The placeholders of a layout or master part with a frame or list
	/// style of their own.
	fn parse(nodes: &[BsxNode]) -> Vec<Self> {
		let Some(tree) = BsxNode::document_element(nodes)
			.and_then(|root| root.child("cSld"))
			.and_then(|common| common.child("spTree"))
		else {
			return Vec::new();
		};
		["sp", "pic", "graphicFrame"]
			.iter()
			.flat_map(|local| tree.descendants_named(local))
			.filter_map(|shape| {
				let declared = SlideRead::placeholder(shape)?;
				Some(Self {
					index: declared.attribute("idx").map(SmolStr::new),
					kind: declared.attribute("type").map(SmolStr::new),
					frame: SlideRead::own_frame(shape),
					list_style: shape
						.child("txBody")
						.and_then(|body| body.child("lstStyle"))
						.cloned(),
				})
			})
			.collect()
	}
}

impl DeckParser {
	/// Reads `package`'s presentation.
	fn read(&self, package: &OpcPackage) -> Result<ReadBody> {
		let presentation = package
			.main_part()
			.ok_or_else(|| bevyhow!("the deck has no presentation part"))?
			.to_string();
		let nodes = package.xml(&presentation)?.unwrap_or_default();
		// only the listed slides, a deleted slide's leftover part skipped
		let slides = BsxNode::document_element(&nodes)
			.and_then(|document| document.child("sldIdLst"))
			.map(|list| {
				list.children_named("sldId")
					.filter_map(|slide| slide.prefixed_attribute("id"))
					.filter_map(|id| {
						package.relationship(Some(&presentation), id)
					})
					.filter(|relationship| {
						!relationship.external
							&& relationship.is(OpcRelationship::SLIDE)
					})
					.map(|relationship| relationship.target.to_string())
					.collect::<Vec<_>>()
			})
			.unwrap_or_default();
		let mut body = ReadBody::default();
		for slide in slides {
			let (article, theme) = SlideRead::new(package, slide)?.read()?;
			body.push(article, &self.options, &theme)?;
		}
		body.xok()
	}
}

impl<'a> SlideRead<'a> {
	/// The slide at `part`, its layout's and master's placeholders read.
	fn new(package: &'a OpcPackage, part: String) -> Result<Self> {
		let layout = package
			.related(Some(&part), OpcRelationship::SLIDE_LAYOUT)
			.map(str::to_string);
		let master = layout.as_deref().and_then(|layout| {
			package
				.related(Some(layout), OpcRelationship::SLIDE_MASTER)
				.map(str::to_string)
		});
		let mut placeholders = Vec::new();
		let mut body_style = None;
		for (path, is_master) in [(layout, false), (master, true)]
			.into_iter()
			.filter_map(|(path, is_master)| path.map(|path| (path, is_master)))
		{
			let nodes = package.xml(&path)?.unwrap_or_default();
			placeholders.extend(Placeholder::parse(&nodes));
			if is_master {
				body_style = BsxNode::document_element(&nodes)
					.and_then(|master| master.child("txStyles"))
					.and_then(|styles| styles.child("bodyStyle"))
					.cloned();
			}
		}
		Self {
			package,
			part,
			placeholders,
			body_style,
		}
		.xok()
	}

	/// The slide's article and the theme its colours name slots of.
	fn read(&self) -> Result<(ReadElement, OfficeTheme)> {
		let nodes = self.package.xml(&self.part)?.unwrap_or_default();
		let slide = BsxNode::document_element(&nodes)
			.ok_or_else(|| bevyhow!("the slide `{}` is empty", self.part))?;
		let mut article = ReadElement::new("article").with_flag("data-card");
		if slide.attribute("show") == Some("0") {
			article = article.with_flag("hidden");
		}
		let shapes = slide
			.child("cSld")
			.and_then(|common| common.child("spTree"))
			.map(|tree| self.shapes(tree, &GroupTransform::default()))
			.unwrap_or_default();
		for block in Self::front(shapes) {
			article = article.with_child(block);
		}
		let notes = self.notes()?;
		if !notes.is_empty() {
			article = article.with_child(ReadElement::new("hr"));
			for block in notes {
				article = article.with_child(block);
			}
		}
		(article, self.theme()).xok()
	}

	/// The theme the slide's master relates, Office's default where none.
	fn theme(&self) -> OfficeTheme {
		let package = self.package;
		package
			.related(Some(&self.part), OpcRelationship::SLIDE_LAYOUT)
			.and_then(|layout| {
				package.related(Some(layout), OpcRelationship::SLIDE_MASTER)
			})
			.and_then(|master| {
				package.related(Some(master), OpcRelationship::THEME)
			})
			.and_then(|theme| package.xml(theme).ok().flatten())
			.map(|nodes| OfficeTheme::parse(&nodes))
			.unwrap_or_default()
	}

	/// The slide's front laid out by its placeholders: a title with only
	/// subtitles centered, a title with anything else a heading over
	/// columns, and no title its shapes in order.
	fn front(shapes: Vec<ShapeRead>) -> Vec<ReadElement> {
		let shapes = shapes
			.into_iter()
			.filter(|shape| {
				shape.role != Role::Chrome && !shape.blocks.is_empty()
			})
			.collect::<Vec<_>>();
		let Some(title) =
			shapes.iter().position(|shape| shape.role == Role::Title)
		else {
			return shapes.into_iter().flat_map(|shape| shape.blocks).collect();
		};
		let heading = Self::heading(&shapes[title].blocks);
		let rest = shapes
			.into_iter()
			.enumerate()
			.filter(|(index, _)| *index != title)
			.map(|(_, shape)| shape)
			.collect::<Vec<_>>();
		if rest.iter().all(|shape| shape.role == Role::Subtitle) {
			let center = rest.into_iter().flat_map(|shape| shape.blocks).fold(
				ReadElement::new("div")
					.with_attribute("class", "card-center")
					.with_child(heading),
				ReadElement::with_child,
			);
			return vec![center];
		}
		let columns = Self::columns(rest)
			.into_iter()
			.map(|blocks| {
				blocks.into_iter().fold(
					ReadElement::new("div")
						.with_attribute("class", "card-column"),
					ReadElement::with_child,
				)
			})
			.fold(
				ReadElement::new("div").with_attribute("class", "card-columns"),
				ReadElement::with_child,
			);
		vec![
			ReadElement::new("div")
				.with_attribute("class", "card-content")
				.with_child(
					ReadElement::new("header")
						.with_attribute("class", "card-heading")
						.with_child(heading),
				)
				.with_child(columns),
		]
	}

	/// The title's paragraphs as one `h1`, a line break between each.
	fn heading(blocks: &[ReadElement]) -> ReadElement {
		let mut heading = ReadElement::new("h1");
		for (index, block) in blocks.iter().enumerate() {
			if index > 0 {
				heading = heading.with_child(ReadElement::new("br"));
			} else {
				heading.look.text_align = block.look.text_align;
			}
			heading.children.extend(block.children.iter().cloned());
		}
		heading
	}

	/// The blocks of each column in reading order: one per body
	/// placeholder when two or more sit side by side, else one, each other
	/// shape joining the column it overlaps most.
	fn columns(shapes: Vec<ShapeRead>) -> Vec<Vec<ReadElement>> {
		let mut bodies = shapes
			.iter()
			.filter(|shape| shape.role == Role::Body)
			.map(|shape| shape.frame)
			.collect::<Vec<_>>();
		bodies.sort_by(|a, b| a.min.x.total_cmp(&b.min.x));
		let side_by_side = bodies.len() >= 2
			&& bodies.windows(2).all(|pair| {
				let overlap = Xfrm::overlap(&pair[0], &pair[1]);
				overlap.y > 0.
					&& overlap.x < pair[0].width().min(pair[1].width()) / 2.
			});
		if !side_by_side {
			return vec![
				shapes.into_iter().flat_map(|shape| shape.blocks).collect(),
			];
		}
		let mut columns = vec![Vec::new(); bodies.len()];
		for shape in shapes {
			let column = match shape.role {
				Role::Body => bodies
					.iter()
					.position(|frame| *frame == shape.frame)
					.unwrap_or(0),
				_ => (0..bodies.len())
					.max_by(|a, b| {
						Xfrm::overlap(&shape.frame, &bodies[*a])
							.x
							.total_cmp(
								&Xfrm::overlap(&shape.frame, &bodies[*b]).x,
							)
							.then(b.cmp(a))
					})
					.unwrap_or(0),
			};
			columns[column].extend(shape.blocks);
		}
		columns
	}

	/// The shapes of a shape tree or group in reading order, top to bottom
	/// then left to right, a group's shapes in its place.
	fn shapes(
		&self,
		tree: &BsxElement,
		transform: &GroupTransform,
	) -> Vec<ShapeRead> {
		let mut placed = Vec::<(Rect, Vec<ShapeRead>)>::new();
		for child in tree.elements() {
			let shapes = self.shape(child, transform);
			if shapes.is_empty() {
				continue;
			}
			// a group sorts by its own frame, its shapes kept together
			let frame = match child.local_name() {
				"grpSp" => child
					.child("grpSpPr")
					.and_then(|properties| properties.child("xfrm"))
					.and_then(Xfrm::frame)
					.map(|frame| transform.apply(frame))
					.unwrap_or(shapes[0].frame),
				_ => shapes[0].frame,
			};
			placed.push((frame, shapes));
		}
		placed.sort_by(|(a, _), (b, _)| {
			a.min
				.y
				.total_cmp(&b.min.y)
				.then(a.min.x.total_cmp(&b.min.x))
		});
		placed.into_iter().flat_map(|(_, shapes)| shapes).collect()
	}

	/// One element of a shape tree as the shapes it reads as.
	fn shape(
		&self,
		shape: &BsxElement,
		transform: &GroupTransform,
	) -> Vec<ShapeRead> {
		let frame = || {
			self.frame(shape)
				.map(|frame| transform.apply(frame))
				.unwrap_or_default()
		};
		match shape.local_name() {
			"sp" => {
				let (role, body) = self.role(shape);
				let blocks = shape
					.child("txBody")
					.map(|text| self.text_body(shape, text, body))
					.unwrap_or_default();
				vec![ShapeRead {
					role,
					frame: frame(),
					blocks,
				}]
			}
			"pic" => {
				let frame = frame();
				let img = self.picture(shape, &frame);
				vec![ShapeRead {
					role: self.role(shape).0,
					frame,
					blocks: img.into_iter().collect(),
				}]
			}
			"graphicFrame" => vec![ShapeRead {
				role: self.role(shape).0,
				frame: frame(),
				blocks: self.graphic(shape),
			}],
			"grpSp" => {
				let nested = shape
					.child("grpSpPr")
					.and_then(|properties| properties.child("xfrm"))
					.map(|xfrm| transform.nest(xfrm))
					.unwrap_or(*transform);
				self.shapes(shape, &nested)
			}
			// the choice an Office reader shows, else its fallback
			"AlternateContent" => {
				let branch = |local: &str| {
					shape
						.child(local)
						.map(|branch| {
							branch
								.elements()
								.flat_map(|child| self.shape(child, transform))
								.collect::<Vec<_>>()
						})
						.unwrap_or_default()
				};
				let choice = branch("Choice");
				match choice.iter().any(|shape| !shape.blocks.is_empty()) {
					true => choice,
					false => branch("Fallback"),
				}
			}
			_ => Vec::new(),
		}
	}

	/// A shape's placeholder declaration, `p:ph` under its non-visual
	/// properties.
	fn placeholder(shape: &BsxElement) -> Option<&BsxElement> {
		shape
			.elements()
			.find(|element| element.local_name().starts_with("nv"))?
			.child("nvPr")?
			.child("ph")
	}

	/// A shape's own frame, from its shape, group or graphic frame
	/// properties.
	fn own_frame(shape: &BsxElement) -> Option<Rect> {
		["spPr", "grpSpPr"]
			.iter()
			.find_map(|local| shape.child(local))
			.and_then(|properties| properties.child("xfrm"))
			.or_else(|| shape.child("xfrm"))
			.and_then(Xfrm::frame)
	}

	/// The layout's or master's placeholder `shape` inherits from: by its
	/// `idx`, else its `type`.
	fn inherited(
		&self,
		shape: &BsxElement,
	) -> impl Iterator<Item = &Placeholder> {
		let declared = Self::placeholder(shape);
		let index = declared.and_then(|declared| declared.attribute("idx"));
		let kind = declared
			.map(|declared| declared.attribute("type").unwrap_or("obj"));
		self.placeholders.iter().filter(move |placeholder| {
			declared.is_some()
				&& match (index, &placeholder.index) {
					(Some(index), Some(other)) => index == other.as_str(),
					_ => {
						placeholder.kind.as_deref().unwrap_or("obj")
							== kind.unwrap_or("obj")
					}
				}
		})
	}

	/// A shape's frame, a placeholder without its own taking its layout's,
	/// else its master's.
	fn frame(&self, shape: &BsxElement) -> Option<Rect> {
		Self::own_frame(shape).or_else(|| {
			self.inherited(shape)
				.find_map(|placeholder| placeholder.frame)
		})
	}

	/// A shape's role by its placeholder type, and whether its paragraphs
	/// are a body's, bulleted unless a list style says otherwise.
	fn role(&self, shape: &BsxElement) -> (Role, bool) {
		let Some(declared) = Self::placeholder(shape) else {
			return (Role::Other, false);
		};
		match declared.attribute("type").unwrap_or("obj") {
			"ctrTitle" | "title" => (Role::Title, false),
			"subTitle" => (Role::Subtitle, false),
			"body" | "obj" => (Role::Body, true),
			"dt" | "ftr" | "sldNum" | "hdr" => (Role::Chrome, false),
			_ => (Role::Other, false),
		}
	}

	/// The bullet a paragraph at `level` of `shape` takes where its own
	/// properties say nothing: the shape's list style, its layout's and
	/// master's placeholders', a body's the master's body style, else a
	/// body bulleted and any other frame not.
	fn default_bullet(
		&self,
		shape: &BsxElement,
		level: u8,
		body: bool,
	) -> Bullet {
		let level_name = format!("lvl{}pPr", level + 1);
		let from = |style: &BsxElement| {
			style.child(&level_name).and_then(Self::bullet)
		};
		shape
			.child("txBody")
			.and_then(|text| text.child("lstStyle"))
			.and_then(from)
			.or_else(|| {
				self.inherited(shape).find_map(|placeholder| {
					placeholder.list_style.as_ref().and_then(from)
				})
			})
			.or_else(|| {
				body.then(|| self.body_style.as_ref().and_then(from))
					.flatten()
			})
			.unwrap_or(match body {
				true => Bullet::Unordered,
				false => Bullet::None,
			})
	}

	/// The bullet a paragraph's properties set, `None` where they set none.
	fn bullet(properties: &BsxElement) -> Option<Bullet> {
		properties
			.elements()
			.find_map(|child| match child.local_name() {
				"buNone" => Some(Bullet::None),
				"buAutoNum" => Some(Bullet::Ordered),
				"buChar" | "buBlip" => Some(Bullet::Unordered),
				_ => None,
			})
	}

	/// A text frame's paragraphs as blocks, each run of list items one list
	/// nested by level, a body's paragraphs bulleted unless they say
	/// otherwise, empty paragraphs dropped.
	fn text_body(
		&self,
		shape: &BsxElement,
		text: &BsxElement,
		body: bool,
	) -> Vec<ReadElement> {
		let base = Self::base_size(text);
		let mut out = Vec::new();
		let mut items = Vec::<ReadListItem>::new();
		let flush = |items: &mut Vec<ReadListItem>,
		             out: &mut Vec<ReadElement>| {
			if items.is_empty() {
				return;
			}
			let top = items.iter().map(|item| item.level).min().unwrap_or(0);
			out.push(ReadListItem::list(items, top));
			items.clear();
		};
		for paragraph in text.children_named("p") {
			let properties = paragraph.child("pPr");
			let level = properties
				.and_then(|properties| properties.attribute("lvl"))
				.and_then(|level| level.parse::<u8>().ok())
				.unwrap_or(0);
			let bullet = properties
				.and_then(Self::bullet)
				.unwrap_or_else(|| self.default_bullet(shape, level, body));
			let children = self.inline(paragraph, base);
			if children.is_empty() {
				continue;
			}
			let look = DirectLook {
				text_align: properties
					.and_then(|properties| properties.attribute("algn"))
					.and_then(|align| match align {
						"ctr" => Some(TextAlign::Center),
						"r" => Some(TextAlign::Right),
						_ => None,
					}),
				..default()
			};
			match bullet {
				Bullet::None => {
					flush(&mut items, &mut out);
					let mut element = ReadElement::new("p").with_look(look);
					element.children = children;
					out.push(element);
				}
				bullet => {
					let ordered = bullet == Bullet::Ordered;
					// a top level item of the other kind starts a new list
					if items.first().is_some_and(|first| {
						level <= first.level && first.ordered != ordered
					}) {
						flush(&mut items, &mut out);
					}
					let mut element = ReadElement::new("li").with_look(look);
					element.children = children;
					items.push(ReadListItem {
						level,
						ordered,
						element,
					});
				}
			}
		}
		flush(&mut items, &mut out);
		out
	}

	/// The size most of a text frame's text is set in, in hundredths of a
	/// point, `None` where most of it sets none, so only a run set otherwise
	/// reads with a size of its own.
	fn base_size(text: &BsxElement) -> Option<u32> {
		let mut counts = HashMap::<Option<u32>, usize>::default();
		for run in text.descendants_named("r") {
			let size = run
				.child("rPr")
				.and_then(|properties| properties.attribute("sz"))
				.and_then(|size| size.parse::<u32>().ok());
			*counts.entry(size).or_default() +=
				run.child("t").map(|text| text.text().len()).unwrap_or(0);
		}
		counts
			.into_iter()
			.max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))
			.and_then(|(size, _)| size)
	}

	/// A paragraph's runs, breaks and fields as inline content, equal
	/// wrappers of adjacent runs merged.
	fn inline(
		&self,
		paragraph: &BsxElement,
		base: Option<u32>,
	) -> Vec<ReadNode> {
		let mut segments = Vec::new();
		for child in paragraph.elements() {
			match child.local_name() {
				"r" | "fld" => {
					let text = child
						.child("t")
						.map(|text| text.text())
						.unwrap_or_default();
					if text.is_empty() {
						continue;
					}
					segments.push(ReadSegment {
						wrappers: self.run_wrappers(child.child("rPr"), base),
						node: ReadNode::Text(text),
					});
				}
				"br" => segments.push(ReadSegment {
					wrappers: Vec::new(),
					node: ReadNode::Element(ReadElement::new("br")),
				}),
				_ => {}
			}
		}
		ReadSegment::assemble(&segments)
	}

	/// The wrappers a run's properties make, outermost first: `a` for its
	/// link, a `span` carrying its colour, size or highlight, `strong`,
	/// `em`, `u`, `s`, `sub` or `sup`. A toggle turned off is the master's
	/// to decide, so it reads as nothing.
	fn run_wrappers(
		&self,
		properties: Option<&BsxElement>,
		base: Option<u32>,
	) -> Vec<ReadElement> {
		let Some(properties) = properties else {
			return Vec::new();
		};
		let mut wrappers = Vec::new();
		if let Some(href) = properties
			.child("hlinkClick")
			.and_then(|click| click.prefixed_attribute("id"))
			.and_then(|id| self.package.relationship(Some(&self.part), id))
		{
			wrappers.push(
				ReadElement::new("a")
					.with_attribute("href", href.target.to_string()),
			);
		}
		let look = DirectLook {
			color: properties.child("solidFill").and_then(Self::color),
			background: properties.child("highlight").and_then(Self::color),
			font_size_pt: properties
				.attribute("sz")
				.and_then(|size| size.parse::<u32>().ok())
				.filter(|size| Some(*size) != base)
				.map(|size| size as f32 / 100.),
			..default()
		};
		if !look.is_empty() {
			wrappers.push(ReadElement::new("span").with_look(look));
		}
		let on =
			|key: &str| matches!(properties.attribute(key), Some("1" | "true"));
		for (set, tag) in [
			(on("b"), "strong"),
			(on("i"), "em"),
			(
				properties
					.attribute("u")
					.is_some_and(|underline| underline != "none"),
				"u",
			),
			(
				properties
					.attribute("strike")
					.is_some_and(|strike| strike != "noStrike"),
				"s",
			),
		] {
			if set {
				wrappers.push(ReadElement::new(tag));
			}
		}
		match properties
			.attribute("baseline")
			.and_then(|baseline| baseline.parse::<i32>().ok())
		{
			Some(baseline) if baseline > 0 => {
				wrappers.push(ReadElement::new("sup"))
			}
			Some(baseline) if baseline < 0 => {
				wrappers.push(ReadElement::new("sub"))
			}
			_ => {}
		}
		wrappers
	}

	/// A DrawingML colour, a fill's or a highlight's: its scheme slot,
	/// tinted by its luminance modifiers, else its value.
	fn color(fill: &BsxElement) -> Option<OfficeColor> {
		let color = fill.elements().next()?;
		let modifier = |local: &str| {
			color
				.child(local)
				.and_then(|modifier| modifier.attribute("val"))
				.and_then(|value| value.parse::<f32>().ok())
				.map(|value| value / 100_000.)
		};
		match color.local_name() {
			"schemeClr" => {
				let slot = ThemeSlot::parse(color.attribute("val")?)?;
				// `lumMod` and `lumOff` as SpreadsheetML's tint, which they
				// express exactly when they sum to one
				let tint = match (modifier("lumMod"), modifier("lumOff")) {
					(_, Some(offset)) if offset > 0. => offset,
					(Some(scale), _) => scale - 1.,
					_ => match (modifier("tint"), modifier("shade")) {
						(Some(tint), _) => 1. - tint,
						(None, Some(shade)) => shade - 1.,
						(None, None) => 0.,
					},
				};
				Some(OfficeColor::Slot { slot, tint })
			}
			"srgbClr" => OfficeColor::parse_hex(color.attribute("val")?)
				.map(OfficeColor::Rgb),
			"sysClr" => OfficeColor::parse_hex(color.attribute("lastClr")?)
				.map(OfficeColor::Rgb),
			_ => None,
		}
	}

	/// A picture shape's `img`, its bytes as a `data:` URL, sized by its
	/// frame.
	fn picture(
		&self,
		picture: &BsxElement,
		frame: &Rect,
	) -> Option<ReadElement> {
		let blip = picture.child("blipFill")?.child("blip")?;
		let alt = picture
			.child("nvPicPr")
			.and_then(|properties| properties.child("cNvPr"))
			.and_then(|properties| properties.attribute("descr"))
			.unwrap_or_default();
		let id = blip
			.prefixed_attribute("embed")
			.or_else(|| blip.prefixed_attribute("link"))?;
		let relationship = self.package.relationship(Some(&self.part), id)?;
		match relationship.external {
			true => Some(OfficePicture::read(
				relationship.target.to_string(),
				Xfrm::emu(frame),
				alt,
			)),
			false => {
				let path = relationship.target.as_str();
				Some(OfficePicture::read_embedded(
					self.package.part(path)?,
					self.package.content_type(path)?,
					Xfrm::emu(frame),
					alt,
				))
			}
		}
	}

	/// A graphic frame's table, or its SmartArt's text as paragraphs; a
	/// chart reads as nothing.
	fn graphic(&self, frame: &BsxElement) -> Vec<ReadElement> {
		let Some(data) = frame
			.child("graphic")
			.and_then(|graphic| graphic.child("graphicData"))
		else {
			return Vec::new();
		};
		if let Some(table) = data.child("tbl") {
			return vec![self.table(table)];
		}
		let Some(path) = data
			.child("relIds")
			.and_then(|ids| ids.prefixed_attribute("dm"))
			.and_then(|id| self.package.relationship(Some(&self.part), id))
			.map(|relationship| relationship.target.to_string())
		else {
			return Vec::new();
		};
		let nodes = self.package.xml(&path).ok().flatten().unwrap_or_default();
		let Some(root) = BsxNode::document_element(&nodes) else {
			return Vec::new();
		};
		root.descendants_named("p")
			.into_iter()
			.filter_map(|paragraph| {
				let children = self.inline(paragraph, None);
				(!children.is_empty()).then(|| {
					let mut element = ReadElement::new("p");
					element.children = children;
					element
				})
			})
			.collect()
	}

	/// A table: its first row a `thead` of `th`s when the table says so,
	/// every other row in a `tbody`, a merge as spans on its first cell and
	/// the cells it covers absent.
	fn table(&self, table: &BsxElement) -> ReadElement {
		let header = table
			.child("tblPr")
			.and_then(|properties| properties.attribute("firstRow"))
			.is_some_and(|first| matches!(first, "1" | "true"));
		let mut head = ReadElement::new("thead");
		let mut body = ReadElement::new("tbody");
		for (index, row) in table.children_named("tr").enumerate() {
			let heading = header && index == 0;
			let tag = if heading { "th" } else { "td" };
			let mut tr = ReadElement::new("tr");
			for cell in row.children_named("tc") {
				let covered = |key: &str| {
					matches!(cell.attribute(key), Some("1" | "true"))
				};
				if covered("hMerge") || covered("vMerge") {
					continue;
				}
				let look = DirectLook {
					background: cell
						.child("tcPr")
						.and_then(|properties| properties.child("solidFill"))
						.and_then(Self::color),
					..default()
				};
				let blocks = cell
					.child("txBody")
					.map(|text| self.text_body(cell, text, false))
					.unwrap_or_default();
				let mut element = ReadElement::new(tag)
					.with_look(look)
					.with_cell_content(blocks);
				for (key, span) in
					[("gridSpan", "colspan"), ("rowSpan", "rowspan")]
				{
					if let Some(value) = cell
						.attribute(key)
						.and_then(|value| value.parse::<u32>().ok())
						.filter(|value| *value > 1)
					{
						element =
							element.with_attribute(span, value.to_string());
					}
				}
				tr = tr.with_child(element);
			}
			match heading {
				true => head = head.with_child(tr),
				false => body = body.with_child(tr),
			}
		}
		let mut element = ReadElement::new("table");
		if !head.children.is_empty() {
			element = element.with_child(head);
		}
		if !body.children.is_empty() {
			element = element.with_child(body);
		}
		element
	}

	/// The slide's speaker notes, its notes slide's body placeholder's
	/// paragraphs.
	fn notes(&self) -> Result<Vec<ReadElement>> {
		let Some(path) = self
			.package
			.related(Some(&self.part), OpcRelationship::NOTES_SLIDE)
		else {
			return Ok(Vec::new());
		};
		let nodes = self.package.xml(path)?.unwrap_or_default();
		let notes = SlideRead {
			package: self.package,
			part: path.to_string(),
			placeholders: Vec::new(),
			body_style: None,
		};
		BsxNode::document_element(&nodes)
			.and_then(|root| root.child("cSld"))
			.and_then(|common| common.child("spTree"))
			.into_iter()
			.flat_map(|tree| tree.descendants_named("sp"))
			.filter(|shape| {
				Self::placeholder(shape)
					.and_then(|declared| declared.attribute("type"))
					== Some("body")
			})
			.filter_map(|shape| {
				shape
					.child("txBody")
					.map(|text| notes.text_body(shape, text, false))
			})
			.flatten()
			.collect::<Vec<_>>()
			.xok()
	}
}

impl Installer for DeckParser {
	fn install(self, app: &mut App) { DocParsers::register_on(app, self); }
}

impl DocParser for DeckParser {
	fn media_types(&self) -> Vec<MediaType> { vec![MediaType::Pptx] }

	fn parse(
		&mut self,
		bytes: &MediaBytes,
		parsers: &DocParsers,
	) -> Result<ParsedDocument, ParseError> {
		let package = OpcPackage::open(bytes, MediaType::Pptx)?;
		self.read(&package)?.into_document(&package, parsers)
	}
}

#[cfg(test)]
pub(crate) mod test {
	use super::super::office_picture::test::png;
	use super::super::office_test::*;
	use super::*;

	/// One slide of a [`DeckFixture`].
	#[derive(Default)]
	struct SlideFixture {
		shapes: String,
		hidden: bool,
		notes: Option<String>,
		/// Each relationship's id, type, target and whether it is external.
		relationships: Vec<(String, String, String, bool)>,
	}

	/// A synthetic slide deck built over [`OpcPackage`], the one fixture the
	/// reader's and writer's tests share: a presentation, a master with a
	/// bulleted body style, a Title and Content layout whose title and body
	/// placeholders carry the frames a slide inherits, and each slide's
	/// shapes, notes and relationships.
	#[derive(Default)]
	pub(crate) struct DeckFixture {
		slides: Vec<SlideFixture>,
		/// Each picture part's path and bytes.
		media: Vec<(String, Vec<u8>)>,
		/// Each SmartArt data part's path and its inner XML.
		diagrams: Vec<(String, String)>,
		properties: Option<String>,
	}

	impl DeckFixture {
		/// The prefixes a fixture's parts bind.
		const NAMESPACES: &str = "xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\" \
			xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" \
			xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" \
			xmlns:dgm=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\" \
			xmlns:mc=\"http://schemas.openxmlformats.org/markup-compatibility/2006\"";

		/// Adds a slide of `shapes`, a shape tree's inner PresentationML.
		pub fn with_slide(mut self, shapes: &str) -> Self {
			self.slides.push(SlideFixture {
				shapes: shapes.into(),
				..default()
			});
			self
		}

		/// Hides the last slide.
		pub fn hidden(mut self) -> Self {
			self.slides.last_mut().unwrap().hidden = true;
			self
		}

		/// Gives the last slide notes of `paragraphs`, each a notes body's
		/// `a:p`.
		pub fn with_notes(mut self, paragraphs: &str) -> Self {
			self.slides.last_mut().unwrap().notes = Some(paragraphs.into());
			self
		}

		/// Relates the last slide to the hyperlink `target` as `id`.
		pub fn with_link(mut self, id: &str, target: &str) -> Self {
			self.slides.last_mut().unwrap().relationships.push((
				id.into(),
				OpcRelationship::HYPERLINK.into(),
				target.into(),
				true,
			));
			self
		}

		/// Relates the last slide to a picture of `bytes` as `id`.
		pub fn with_image(mut self, id: &str, bytes: Vec<u8>) -> Self {
			let path = format!("ppt/media/image{}.png", self.media.len() + 1);
			self.media.push((path.clone(), bytes));
			self.slides.last_mut().unwrap().relationships.push((
				id.into(),
				OpcRelationship::IMAGE.into(),
				path,
				false,
			));
			self
		}

		/// Relates the last slide to a SmartArt data part of `inner` as `id`.
		pub fn with_diagram(mut self, id: &str, inner: &str) -> Self {
			let path =
				format!("ppt/diagrams/data{}.xml", self.diagrams.len() + 1);
			self.diagrams.push((path.clone(), inner.into()));
			self.slides.last_mut().unwrap().relationships.push((
				id.into(),
				OpcRelationship::DIAGRAM_DATA.into(),
				path,
				false,
			));
			self
		}

		/// Sets the core properties' inner elements, the `dc:` prefix bound.
		pub fn with_properties(mut self, xml: &str) -> Self {
			self.properties = Some(xml.into());
			self
		}

		/// The deck's bytes.
		pub fn bytes(self) -> MediaBytes {
			let ns = Self::NAMESPACES;
			let mut package = OpcPackage::default();
			let (presentation, master, layout, theme) = (
				"ppt/presentation.xml",
				"ppt/slideMasters/slideMaster1.xml",
				"ppt/slideLayouts/slideLayout1.xml",
				"ppt/theme/theme1.xml",
			);
			let placeholder =
				|kind: &str, index: &str, frame: (u32, u32, u32, u32)| {
					format!(
						"<p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"\"/><p:cNvSpPr/><p:nvPr><p:ph {kind} {index}/></p:nvPr></p:nvSpPr>\
					 <p:spPr><a:xfrm><a:off x=\"{}\" y=\"{}\"/><a:ext cx=\"{}\" cy=\"{}\"/></a:xfrm></p:spPr>\
					 <p:txBody><a:bodyPr/><a:lstStyle/><a:p/></p:txBody></p:sp>",
						frame.0, frame.1, frame.2, frame.3
					)
				};
			let placeholders = format!(
				"{}{}",
				placeholder(
					"type=\"title\"",
					"",
					(457200, 274638, 11277600, 1143000)
				),
				placeholder(
					"",
					"idx=\"1\"",
					(457200, 1600200, 11277600, 4525963)
				),
			);
			package
				.insert(theme, "application/vnd.openxmlformats-officedocument.theme+xml", OfficeTheme::default().theme_part())
				.insert(
					master,
					OpcPackage::XML_TYPE,
					format!(
						"<p:sldMaster {ns}><p:cSld><p:spTree>{placeholders}</p:spTree></p:cSld>\
						 <p:txStyles><p:bodyStyle><a:lvl1pPr><a:buChar char=\"•\"/></a:lvl1pPr>\
						 <a:lvl2pPr><a:buChar char=\"•\"/></a:lvl2pPr></p:bodyStyle></p:txStyles></p:sldMaster>"
					),
				)
				.insert(
					layout,
					OpcPackage::XML_TYPE,
					format!(
						"<p:sldLayout {ns}><p:cSld name=\"Title and Content\"><p:spTree>{placeholders}</p:spTree></p:cSld></p:sldLayout>"
					),
				);
			package.relate(Some(master), OpcRelationship::THEME, theme);
			package.relate(Some(layout), OpcRelationship::SLIDE_MASTER, master);
			for (path, bytes) in &self.media {
				package.insert(path.as_str(), "image/png", bytes.clone());
			}
			for (path, inner) in &self.diagrams {
				package.insert(
					path.as_str(),
					OpcPackage::XML_TYPE,
					format!(
						"<dgm:dataModel {ns}><dgm:ptLst>{inner}</dgm:ptLst></dgm:dataModel>"
					),
				);
			}
			let mut listed = String::new();
			for (index, slide) in self.slides.iter().enumerate() {
				let path = format!("ppt/slides/slide{}.xml", index + 1);
				let show = if slide.hidden { " show=\"0\"" } else { "" };
				package.insert(
					path.as_str(),
					OpcPackage::XML_TYPE,
					format!(
						"<p:sld {ns}{show}><p:cSld><p:spTree>{}</p:spTree></p:cSld></p:sld>",
						slide.shapes
					),
				);
				// the slide's own first, so they keep the ids its shapes name
				for (id, kind, target, external) in &slide.relationships {
					let made = match external {
						true => {
							package.relate_external(Some(&path), kind, target)
						}
						false => package.relate(Some(&path), kind, target),
					};
					assert_eq!(
						made.as_str(),
						id,
						"fixture ids count from rId1"
					);
				}
				package.relate(
					Some(&path),
					OpcRelationship::SLIDE_LAYOUT,
					layout,
				);
				if let Some(notes) = &slide.notes {
					let notes_path =
						format!("ppt/notesSlides/notesSlide{}.xml", index + 1);
					package.insert(
						notes_path.as_str(),
						OpcPackage::XML_TYPE,
						format!(
							"<p:notes {ns}><p:cSld><p:spTree>\
							 <p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"\"/><p:cNvSpPr/><p:nvPr><p:ph type=\"sldImg\"/></p:nvPr></p:nvSpPr><p:spPr/></p:sp>\
							 <p:sp><p:nvSpPr><p:cNvPr id=\"3\" name=\"\"/><p:cNvSpPr/><p:nvPr><p:ph type=\"body\" idx=\"1\"/></p:nvPr></p:nvSpPr><p:spPr/>\
							 <p:txBody><a:bodyPr/><a:lstStyle/>{notes}</p:txBody></p:sp>\
							 </p:spTree></p:cSld></p:notes>"
						),
					);
					package.relate(
						Some(&path),
						OpcRelationship::NOTES_SLIDE,
						&notes_path,
					);
				}
				let id = package.relate(
					Some(presentation),
					OpcRelationship::SLIDE,
					&path,
				);
				listed.push_str(&format!(
					"<p:sldId id=\"{}\" r:id=\"{id}\"/>",
					256 + index
				));
			}
			// a deleted slide's leftover, related but never listed
			package.insert(
				"ppt/slides/slide99.xml",
				OpcPackage::XML_TYPE,
				format!(
					"<p:sld {ns}><p:cSld><p:spTree>{}</p:spTree></p:cSld></p:sld>",
					text_box(0, 0, "<a:p><a:r><a:t>Leftover</a:t></a:r></a:p>")
				),
			);
			package.relate(
				Some(presentation),
				OpcRelationship::SLIDE,
				"ppt/slides/slide99.xml",
			);
			package.insert(
				presentation,
				OpcPackage::XML_TYPE,
				format!(
					"<p:presentation {ns}><p:sldIdLst>{listed}</p:sldIdLst>\
					 <p:sldSz cx=\"12192000\" cy=\"6858000\"/></p:presentation>"
				),
			);
			package.relate(
				None,
				OpcRelationship::OFFICE_DOCUMENT,
				presentation,
			);
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
			MediaBytes::new(MediaType::Pptx, package.to_bytes().unwrap())
		}
	}

	/// A placeholder shape of `ph`'s attributes holding `paragraphs`, its
	/// frame inherited from the layout.
	pub(crate) fn placeholder(ph: &str, paragraphs: &str) -> String {
		format!(
			"<p:sp><p:nvSpPr><p:cNvPr id=\"10\" name=\"\"/><p:cNvSpPr/><p:nvPr><p:ph {ph}/></p:nvPr></p:nvSpPr>\
			 <p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/>{paragraphs}</p:txBody></p:sp>"
		)
	}

	/// A placeholder shape of `ph`'s attributes at `x`, `y` (EMU) holding
	/// `paragraphs`.
	pub(crate) fn placed(ph: &str, x: u32, y: u32, paragraphs: &str) -> String {
		format!(
			"<p:sp><p:nvSpPr><p:cNvPr id=\"11\" name=\"\"/><p:cNvSpPr/><p:nvPr><p:ph {ph}/></p:nvPr></p:nvSpPr>\
			 <p:spPr><a:xfrm><a:off x=\"{x}\" y=\"{y}\"/><a:ext cx=\"5000000\" cy=\"4000000\"/></a:xfrm></p:spPr>\
			 <p:txBody><a:bodyPr/><a:lstStyle/>{paragraphs}</p:txBody></p:sp>"
		)
	}

	/// A text box at `x`, `y` (EMU) holding `paragraphs`.
	pub(crate) fn text_box(x: u32, y: u32, paragraphs: &str) -> String {
		format!(
			"<p:sp><p:nvSpPr><p:cNvPr id=\"12\" name=\"\"/><p:cNvSpPr txBox=\"1\"/><p:nvPr/></p:nvSpPr>\
			 <p:spPr><a:xfrm><a:off x=\"{x}\" y=\"{y}\"/><a:ext cx=\"3000000\" cy=\"1000000\"/></a:xfrm></p:spPr>\
			 <p:txBody><a:bodyPr/><a:lstStyle/>{paragraphs}</p:txBody></p:sp>"
		)
	}

	/// A paragraph of one run of `text`, its paragraph properties `ppr` and
	/// run properties' attributes `rpr`.
	pub(crate) fn paragraph(ppr: &str, rpr: &str, text: &str) -> String {
		format!(
			"<a:p>{ppr}<a:r><a:rPr lang=\"en-US\" {rpr}/><a:t>{text}</a:t></a:r></a:p>"
		)
	}

	/// A picture of the relationship `id` at `x`, `y` (EMU), `cx` by `cy`.
	pub(crate) fn picture(
		id: &str,
		x: u32,
		y: u32,
		cx: u32,
		cy: u32,
	) -> String {
		format!(
			"<p:pic><p:nvPicPr><p:cNvPr id=\"13\" name=\"\" descr=\"a dot\"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr>\
			 <p:blipFill><a:blip r:embed=\"{id}\"/></p:blipFill>\
			 <p:spPr><a:xfrm><a:off x=\"{x}\" y=\"{y}\"/><a:ext cx=\"{cx}\" cy=\"{cy}\"/></a:xfrm></p:spPr></p:pic>"
		)
	}

	/// A table at `x`, `y` (EMU) of `rows`, each a `a:tr`'s inner `a:tc`s.
	pub(crate) fn table(x: u32, y: u32, header: bool, rows: &[&str]) -> String {
		let rows = rows
			.iter()
			.map(|row| format!("<a:tr h=\"370840\">{row}</a:tr>"))
			.collect::<String>();
		format!(
			"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id=\"14\" name=\"\"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>\
			 <p:xfrm><a:off x=\"{x}\" y=\"{y}\"/><a:ext cx=\"6000000\" cy=\"1000000\"/></p:xfrm>\
			 <a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/table\"><a:tbl>\
			 <a:tblPr firstRow=\"{}\"/><a:tblGrid><a:gridCol w=\"2000000\"/><a:gridCol w=\"2000000\"/><a:gridCol w=\"2000000\"/></a:tblGrid>{rows}\
			 </a:tbl></a:graphicData></a:graphic></p:graphicFrame>",
			match header {
				true => "1",
				false => "0",
			}
		)
	}

	/// A table cell of `text` with the attributes `attributes`.
	pub(crate) fn cell(attributes: &str, text: &str) -> String {
		format!(
			"<a:tc {attributes}><a:txBody><a:bodyPr/><a:lstStyle/>{}</a:txBody><a:tcPr/></a:tc>",
			match text.is_empty() {
				true => "<a:p/>".to_string(),
				false => paragraph("", "", text),
			}
		)
	}

	/// The covering deck: a title slide, a two column content slide with a
	/// picture and notes, a plain slide of a text box, a content slide of a
	/// table and a link, and a hidden slide.
	pub(crate) fn covering() -> MediaBytes {
		DeckFixture::default()
			.with_slide(&format!(
				"{}{}",
				placeholder("type=\"ctrTitle\"", &paragraph("", "", "Field notes")),
				placeholder("type=\"subTitle\" idx=\"1\"", &format!(
					"{}{}",
					paragraph("", "", "A talk"),
					paragraph("", "", "November")
				)),
			))
			.with_notes(&paragraph("", "", "Welcome everyone."))
			.with_slide(&format!(
				"{}{}{}{}",
				// written before the title it sits below, read after it
				placed("idx=\"2\"", 6400800, 1600200, &paragraph("", "", "Right column")),
				placed("idx=\"1\"", 457200, 1600200, &format!(
					"{}{}{}",
					paragraph("", "b=\"1\"", "Left"),
					paragraph("<a:pPr lvl=\"1\"/>", "i=\"1\"", "nested"),
					paragraph("<a:pPr><a:buNone/></a:pPr>", "", "plain")
				)),
				placeholder("type=\"title\"", &paragraph("", "", "Two columns")),
				picture("rId1", 6400800, 3600000, 952500, 476250),
			))
			.with_image("rId1", png())
			.with_notes(&format!(
				"{}{}",
				paragraph("", "", "First point."),
				paragraph("", "", "Second point.")
			))
			.with_slide(&text_box(457200, 457200, &format!(
				"{}{}",
				paragraph("", "", "A plain slide"),
				paragraph("<a:pPr><a:buAutoNum type=\"arabicPeriod\"/></a:pPr>", "", "counted")
			)))
			.with_slide(&format!(
				"{}{}{}",
				placeholder("type=\"title\"", &paragraph("", "", "A table")),
				placeholder("idx=\"1\"", &format!(
					"<a:p><a:pPr><a:buNone/></a:pPr><a:r><a:rPr lang=\"en-US\"><a:hlinkClick r:id=\"rId1\"/></a:rPr><a:t>beet</a:t></a:r>\
					 <a:r><a:rPr lang=\"en-US\"/><a:t> is </a:t></a:r>\
					 <a:r><a:rPr lang=\"en-US\" sz=\"2800\"><a:solidFill><a:srgbClr val=\"FF0000\"/></a:solidFill></a:rPr><a:t>red</a:t></a:r>\
					 <a:r><a:rPr lang=\"en-US\" sz=\"1800\"/><a:t> and more text in the frame's own size</a:t></a:r></a:p>"
				)),
				table(457200, 3600000, true, &[
					&format!("{}{}", cell("gridSpan=\"2\"", "Channel"), cell("hMerge=\"1\"", "")),
					&format!("{}{}", cell("rowSpan=\"2\"", "Stall"), cell("", "a|b")),
					&format!("{}{}", cell("vMerge=\"1\"", ""), cell("", "c")),
				]),
			))
			.with_link("rId1", "https://beet.example/")
			.with_slide(&placeholder("type=\"title\"", &paragraph("", "", "Hidden")))
			.hidden()
			.with_properties("<dc:title>Field notes</dc:title><dc:creator>Ada</dc:creator>")
			.bytes()
	}

	/// The covering deck reads as card articles in presentation order, each
	/// laid out by its placeholders, its notes after an `hr`.
	#[beet_core::test]
	fn reads_a_deck_as_cards() {
		let (mut world, root) = parse(&covering()).unwrap();
		let html = html(&mut world, root);
		html.clone()
			.xpect_contains("<article data-card><div class=\"card-center\"><h1>Field notes</h1><p>A talk</p><p>November</p></div><hr /><p>Welcome everyone.</p></article>")
			// the body bulleted by the master, its nested level and its plain paragraph
			.xpect_contains("<div class=\"card-columns\"><div class=\"card-column\"><ul><li><strong>Left</strong><ul><li><em>nested</em></li></ul></li></ul><p>plain</p></div>")
			// the picture joins the column it sits under
			.xpect_contains("<div class=\"card-column\"><ul><li>Right column</li></ul><img src=\"data:image/png;base64,")
			.xpect_contains("alt=\"a dot\" width=\"100\" height=\"50\" /></div></div></div><hr /><p>First point.</p><p>Second point.</p></article>")
			.xpect_contains("<article data-card><p>A plain slide</p><ol><li>counted</li></ol></article>")
			.xpect_contains("<p><a href=\"https://beet.example/\">beet</a> is <span")
			.xpect_contains("<table><thead><tr><th colspan=\"2\">Channel</th></tr></thead><tbody><tr><td rowspan=\"2\">Stall</td><td>a|b</td></tr><tr><td>c</td></tr></tbody></table>")
			.xpect_contains("<article data-card hidden><div class=\"card-center\"><h1>Hidden</h1></div></article>")
			.xnot()
			.xpect_contains("Leftover");
		html.matches("<article").count().xpect_eq(5);
		world
			.entity(root)
			.get::<DocMeta>()
			.and_then(|meta| meta.title.clone())
			.xpect_eq(Some("Field notes".to_string()));
	}

	/// A group's shapes read in its place, and SmartArt reads as its text.
	#[beet_core::test]
	fn reads_groups_and_smartart() {
		let bytes = DeckFixture::default()
			.with_slide(&format!(
				"{}<p:grpSp><p:nvGrpSpPr><p:cNvPr id=\"5\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>\
				 <p:grpSpPr><a:xfrm><a:off x=\"0\" y=\"1000\"/><a:ext cx=\"100\" cy=\"100\"/><a:chOff x=\"0\" y=\"0\"/><a:chExt cx=\"100\" cy=\"100\"/></a:xfrm></p:grpSpPr>\
				 {}{}</p:grpSp>\
				 <p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id=\"6\" name=\"\"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>\
				 <p:xfrm><a:off x=\"0\" y=\"5000\"/><a:ext cx=\"100\" cy=\"100\"/></p:xfrm><a:graphic>\
				 <a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\"><dgm:relIds r:dm=\"rId1\"/></a:graphicData></a:graphic></p:graphicFrame>",
				text_box(0, 9000, &paragraph("", "", "last")),
				text_box(0, 50, &paragraph("", "", "group second")),
				text_box(0, 0, &paragraph("", "", "group first")),
			))
			.with_diagram(
				"rId1",
				"<dgm:pt modelId=\"1\"><dgm:t><a:bodyPr/><a:p><a:r><a:t>Plan</a:t></a:r></a:p></dgm:t></dgm:pt>\
				 <dgm:pt modelId=\"2\"><dgm:t><a:bodyPr/><a:p><a:r><a:t>Build</a:t></a:r></a:p></dgm:t></dgm:pt>",
			)
			.bytes();
		let (mut world, root) = parse(&bytes).unwrap();
		html(&mut world, root).xpect_contains(
			"<article data-card><p>group first</p><p>group second</p><p>Plan</p><p>Build</p><p>last</p></article>",
		);
	}
}
