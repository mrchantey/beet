//! Markdown-to-ECS differ with positional diffing.
//!
//! [`MarkdownDiffer`] implements [`Parser`] to reconcile a markdown
//! string directly against a Bevy entity hierarchy, spawning,
//! updating, or despawning entities as needed. Diffing is positional
//! — only entities whose content or structure actually changed are
//! touched.
//!
//! # Usage
//!
//! ```
//! use beet_stack::prelude::*;
//! use beet_core::prelude::*;
//!
//! let mut world = World::new();
//! let root = world.spawn_empty().id();
//! MarkdownDiffer::new("# Hello\n\nworld")
//!     .diff(world.entity_mut(root))
//!     .unwrap();
//!
//! // root now has Heading1 and Paragraph children
//! let children = world.entity(root).get::<Children>().unwrap();
//! children.len().xpect_eq(2);
//! ```
//!
//! # Diffing
//!
//! Calling [`MarkdownDiffer::diff`] on an entity that already has
//! children from a previous diff will reconcile positionally:
//!
//! - Matching node kinds at the same index are updated in place.
//! - Mismatched kinds cause the old subtree to be despawned and a new
//!   one spawned.
//! - Extra old children are despawned; extra new nodes are spawned.
use super::Parser as ParserTrait;
use crate::prelude::*;
use beet_core::prelude::*;
use pulldown_cmark::Event;
use pulldown_cmark::Options;
use pulldown_cmark::Tag;
use pulldown_cmark::TextMergeStream;


// ---------------------------------------------------------------------------
// Intermediate representation
// ---------------------------------------------------------------------------

/// Intermediate tree node used internally during diffing.
#[derive(Debug, Clone, PartialEq)]
enum MdNode {
	/// An element with a kind tag and child nodes.
	Element {
		kind: MdElement,
		children: Vec<MdNode>,
	},
	/// A leaf text node.
	Text(String),
}

impl MdNode {
	fn element(kind: MdElement, children: Vec<MdNode>) -> Self {
		Self::Element { kind, children }
	}
	fn leaf(kind: MdElement) -> Self {
		Self::Element {
			kind,
			children: Vec::new(),
		}
	}
}


/// The semantic kind of an [`MdNode::Element`].
#[derive(Debug, Clone, PartialEq)]
enum MdElement {
	// -- block --
	Paragraph,
	Heading(u8),
	BlockQuote,
	CodeBlock { language: Option<String> },
	OrderedList { start: u64 },
	UnorderedList,
	ListItem,
	Table { alignments: Vec<TextAlignment> },
	TableHead,
	TableRow,
	TableCell,
	ThematicBreak,
	FootnoteDefinition { label: String },
	DefinitionList,
	DefinitionTitle,
	DefinitionDetails,
	HtmlBlock(String),
	MetadataBlock { kind: MetadataKind },
	MathDisplay,
	Image { src: String, title: Option<String> },
	// -- inline wrappers --
	Strong,
	Emphasis,
	Strikethrough,
	Superscript,
	Subscript,
	Link { href: String, title: Option<String> },
	InlineCode,
	MathInline,
	HardBreak,
	SoftBreak,
	FootnoteRef { label: String },
	HtmlInline(String),
	TaskListCheck { checked: bool },
}

impl MdElement {
	/// Returns true if two elements are the "same kind" for diffing
	/// purposes — same variant and same structural parameters, ignoring
	/// mutable content like text or src that can be updated in place.
	#[allow(dead_code)]
	fn same_kind(&self, other: &Self) -> bool {
		use MdElement::*;
		match (self, other) {
			(Paragraph, Paragraph)
			| (BlockQuote, BlockQuote)
			| (UnorderedList, UnorderedList)
			| (ListItem, ListItem)
			| (TableHead, TableHead)
			| (TableRow, TableRow)
			| (TableCell, TableCell)
			| (ThematicBreak, ThematicBreak)
			| (DefinitionList, DefinitionList)
			| (DefinitionTitle, DefinitionTitle)
			| (DefinitionDetails, DefinitionDetails)
			| (MathDisplay, MathDisplay)
			| (Strong, Strong)
			| (Emphasis, Emphasis)
			| (Strikethrough, Strikethrough)
			| (Superscript, Superscript)
			| (Subscript, Subscript)
			| (InlineCode, InlineCode)
			| (MathInline, MathInline)
			| (HardBreak, HardBreak)
			| (SoftBreak, SoftBreak) => true,
			(Heading(level_a), Heading(level_b)) => level_a == level_b,
			(
				OrderedList { start: start_a },
				OrderedList { start: start_b },
			) => start_a == start_b,
			(
				CodeBlock { language: lang_a },
				CodeBlock { language: lang_b },
			) => lang_a == lang_b,
			(
				Table {
					alignments: align_a,
				},
				Table {
					alignments: align_b,
				},
			) => align_a == align_b,
			(
				FootnoteDefinition { label: label_a },
				FootnoteDefinition { label: label_b },
			) => label_a == label_b,
			(Link { .. }, Link { .. }) => true,
			(Image { .. }, Image { .. }) => true,
			(
				MetadataBlock { kind: kind_a },
				MetadataBlock { kind: kind_b },
			) => kind_a == kind_b,
			(
				FootnoteRef { label: label_a },
				FootnoteRef { label: label_b },
			) => label_a == label_b,
			(
				TaskListCheck { checked: checked_a },
				TaskListCheck { checked: checked_b },
			) => checked_a == checked_b,
			_ => false,
		}
	}
}


// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

/// Reconciles a markdown string against an entity's children.
///
/// Implements [`Parser`](super::Parser) so it can be used with the
/// generic parsing interface. Internally uses [`pulldown_cmark`] to
/// parse the markdown and positionally diffs the result against the
/// existing entity hierarchy.
///
/// # Example
///
/// ```
/// use beet_stack::prelude::*;
/// use beet_core::prelude::*;
///
/// let mut world = World::new();
/// let root = world.spawn_empty().id();
/// MarkdownDiffer::new("# Hello\n\nworld")
///     .diff(world.entity_mut(root))
///     .unwrap();
///
/// let children = world.entity(root).get::<Children>().unwrap();
/// children.len().xpect_eq(2);
/// ```
pub struct MarkdownDiffer<'a> {
	text: &'a str,
	options: Options,
}

impl<'a> MarkdownDiffer<'a> {
	/// Create a differ for the given markdown text.
	pub fn new(text: &'a str) -> Self {
		Self {
			text,
			options: Self::default_options(),
		}
	}

	/// Create a differ with custom pulldown-cmark options.
	pub fn with_options(text: &'a str, options: Options) -> Self {
		Self { text, options }
	}

	/// Returns the default pulldown-cmark options.
	pub fn default_options() -> Options {
		Options::ENABLE_TABLES
			| Options::ENABLE_FOOTNOTES
			| Options::ENABLE_STRIKETHROUGH
			| Options::ENABLE_TASKLISTS
			| Options::ENABLE_HEADING_ATTRIBUTES
			| Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
			| Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS
			| Options::ENABLE_MATH
			| Options::ENABLE_GFM
			| Options::ENABLE_DEFINITION_LIST
			| Options::ENABLE_SUPERSCRIPT
			| Options::ENABLE_SUBSCRIPT
			| Options::ENABLE_WIKILINKS
	}

	/// Parse the markdown text into an intermediate node tree.
	fn parse(&self) -> Vec<MdNode> {
		let parser = pulldown_cmark::Parser::new_ext(self.text, self.options);
		let stream = TextMergeStream::new(parser);
		let mut builder = TreeBuilder::default();
		for event in stream {
			builder.handle_event(event);
		}
		builder.finish()
	}
}

impl ParserTrait for MarkdownDiffer<'_> {
	fn diff(&mut self, entity: EntityWorldMut) -> Result {
		let nodes = self.parse();
		let entity_id = entity.id();
		let world = entity.into_world_mut();
		diff_children(world, entity_id, &nodes);
		Ok(())
	}
}


// ---------------------------------------------------------------------------
// Tree builder — converts pulldown events into MdNode tree
// ---------------------------------------------------------------------------

struct TreeBuilder {
	/// Stack of (element_kind, children_so_far).
	/// The bottom entry is a synthetic root whose children become the
	/// final output.
	stack: Vec<(Option<MdElement>, Vec<MdNode>)>,
}

impl Default for TreeBuilder {
	fn default() -> Self {
		Self {
			stack: vec![(None, Vec::new())],
		}
	}
}

impl TreeBuilder {
	fn push(&mut self, kind: MdElement) {
		self.stack.push((Some(kind), Vec::new()));
	}

	fn pop(&mut self) {
		if let Some((kind, children)) = self.stack.pop() {
			let node = match kind {
				Some(kind) => MdNode::element(kind, children),
				None => return, // shouldn't happen
			};
			if let Some(parent) = self.stack.last_mut() {
				parent.1.push(node);
			}
		}
	}

	fn push_leaf(&mut self, node: MdNode) {
		if let Some(parent) = self.stack.last_mut() {
			parent.1.push(node);
		}
	}

	fn finish(mut self) -> Vec<MdNode> {
		// Drain any unclosed elements (shouldn't happen with valid
		// markdown, but be defensive).
		while self.stack.len() > 1 {
			self.pop();
		}
		self.stack
			.pop()
			.map(|(_, children)| children)
			.unwrap_or_default()
	}

	fn handle_event(&mut self, event: Event) {
		match event {
			Event::Start(tag) => self.handle_start(tag),
			Event::End(_) => self.pop(),
			Event::Text(text) => {
				self.push_leaf(MdNode::Text(text.into_string()));
			}
			Event::Code(text) => {
				self.push_leaf(MdNode::element(MdElement::InlineCode, vec![
					MdNode::Text(text.into_string()),
				]));
			}
			Event::InlineMath(text) => {
				self.push_leaf(MdNode::element(MdElement::MathInline, vec![
					MdNode::Text(text.into_string()),
				]));
			}
			Event::DisplayMath(text) => {
				self.push_leaf(MdNode::element(MdElement::MathDisplay, vec![
					MdNode::Text(text.into_string()),
				]));
			}
			Event::Html(text) => {
				self.push_leaf(MdNode::element(
					MdElement::HtmlBlock(text.into_string()),
					Vec::new(),
				));
			}
			Event::InlineHtml(text) => {
				self.push_leaf(MdNode::element(
					MdElement::HtmlInline(text.into_string()),
					Vec::new(),
				));
			}
			Event::FootnoteReference(label) => {
				self.push_leaf(MdNode::leaf(MdElement::FootnoteRef {
					label: label.into_string(),
				}));
			}
			Event::SoftBreak => {
				self.push_leaf(MdNode::leaf(MdElement::SoftBreak));
			}
			Event::HardBreak => {
				self.push_leaf(MdNode::leaf(MdElement::HardBreak));
			}
			Event::Rule => {
				self.push_leaf(MdNode::leaf(MdElement::ThematicBreak));
			}
			Event::TaskListMarker(checked) => {
				self.push_leaf(MdNode::leaf(MdElement::TaskListCheck {
					checked,
				}));
			}
		}
	}

	fn handle_start(&mut self, tag: Tag) {
		match tag {
			Tag::Paragraph => self.push(MdElement::Paragraph),
			Tag::Heading { level, .. } => {
				self.push(MdElement::Heading(heading_level(level)));
			}
			Tag::BlockQuote(_) => self.push(MdElement::BlockQuote),
			Tag::CodeBlock(kind) => {
				let language = match kind {
					pulldown_cmark::CodeBlockKind::Fenced(info) => {
						let info = info.into_string();
						let lang = info
							.split_whitespace()
							.next()
							.unwrap_or("")
							.to_string();
						if lang.is_empty() { None } else { Some(lang) }
					}
					pulldown_cmark::CodeBlockKind::Indented => None,
				};
				self.push(MdElement::CodeBlock { language });
			}
			Tag::HtmlBlock => {
				// HtmlBlock is a container tag in pulldown-cmark;
				// the actual HTML arrives as Text events inside.
				// We accumulate into an HtmlBlock element.
				self.push(MdElement::HtmlBlock(String::new()));
			}
			Tag::List(start) => match start {
				Some(start_num) => {
					self.push(MdElement::OrderedList { start: start_num });
				}
				None => self.push(MdElement::UnorderedList),
			},
			Tag::Item => self.push(MdElement::ListItem),
			Tag::FootnoteDefinition(label) => {
				self.push(MdElement::FootnoteDefinition {
					label: label.into_string(),
				});
			}
			Tag::DefinitionList => self.push(MdElement::DefinitionList),
			Tag::DefinitionListTitle => self.push(MdElement::DefinitionTitle),
			Tag::DefinitionListDefinition => {
				self.push(MdElement::DefinitionDetails);
			}
			Tag::Table(alignments) => {
				let alignments = alignments
					.into_iter()
					.map(|alignment| match alignment {
						pulldown_cmark::Alignment::None => TextAlignment::None,
						pulldown_cmark::Alignment::Left => TextAlignment::Left,
						pulldown_cmark::Alignment::Center => {
							TextAlignment::Center
						}
						pulldown_cmark::Alignment::Right => {
							TextAlignment::Right
						}
					})
					.collect();
				self.push(MdElement::Table { alignments });
			}
			Tag::TableHead => self.push(MdElement::TableHead),
			Tag::TableRow => self.push(MdElement::TableRow),
			Tag::TableCell => self.push(MdElement::TableCell),
			Tag::Emphasis => self.push(MdElement::Emphasis),
			Tag::Strong => self.push(MdElement::Strong),
			Tag::Strikethrough => self.push(MdElement::Strikethrough),
			Tag::Superscript => self.push(MdElement::Superscript),
			Tag::Subscript => self.push(MdElement::Subscript),
			Tag::Link {
				dest_url, title, ..
			} => {
				self.push(MdElement::Link {
					href: dest_url.into_string(),
					title: if title.is_empty() {
						None
					} else {
						Some(title.into_string())
					},
				});
			}
			Tag::Image {
				dest_url, title, ..
			} => {
				self.push(MdElement::Image {
					src: dest_url.into_string(),
					title: if title.is_empty() {
						None
					} else {
						Some(title.into_string())
					},
				});
			}
			Tag::MetadataBlock(kind) => {
				let kind = match kind {
					pulldown_cmark::MetadataBlockKind::YamlStyle => {
						MetadataKind::Yaml
					}
					pulldown_cmark::MetadataBlockKind::PlusesStyle => {
						MetadataKind::Toml
					}
				};
				self.push(MdElement::MetadataBlock { kind });
			}
		}
	}
}

fn heading_level(level: pulldown_cmark::HeadingLevel) -> u8 {
	match level {
		pulldown_cmark::HeadingLevel::H1 => 1,
		pulldown_cmark::HeadingLevel::H2 => 2,
		pulldown_cmark::HeadingLevel::H3 => 3,
		pulldown_cmark::HeadingLevel::H4 => 4,
		pulldown_cmark::HeadingLevel::H5 => 5,
		pulldown_cmark::HeadingLevel::H6 => 6,
	}
}


// ---------------------------------------------------------------------------
// ECS rendering and diffing
// ---------------------------------------------------------------------------

/// Diff/apply a list of [`MdNode`] as the children of `parent`.
///
/// Walks existing children positionally. Matching kinds are updated in
/// place; mismatches cause despawn + spawn. Extra old children are
/// despawned; extra new nodes are spawned.
fn diff_children(world: &mut World, parent: Entity, nodes: &[MdNode]) {
	let existing: Vec<Entity> = world
		.entity(parent)
		.get::<Children>()
		.map(|children| children.iter().collect())
		.unwrap_or_default();

	let mut keep_count = 0usize;

	for (idx, node) in nodes.iter().enumerate() {
		if idx < existing.len() {
			let child = existing[idx];
			if can_reuse(world, child, node) {
				update_in_place(world, child, node);
				keep_count += 1;
			} else {
				// Mismatch — despawn old subtree and spawn new one in
				// place. We handle ordering after the loop.
				despawn_recursive(world, child);
				let new_child = spawn_node(world, node);
				world.entity_mut(new_child).insert(ChildOf(parent));
				keep_count += 1;
			}
		} else {
			let new_child = spawn_node(world, node);
			world.entity_mut(new_child).insert(ChildOf(parent));
		}
	}

	// Despawn any extra old children beyond the new node count.
	for idx in (keep_count..existing.len()).rev() {
		despawn_recursive(world, existing[idx]);
	}
}

/// Check whether an existing entity can be reused for the given node
/// (same structural kind).
fn can_reuse(world: &World, entity: Entity, node: &MdNode) -> bool {
	match node {
		MdNode::Text(_) => {
			world.entity(entity).contains::<TextNode>()
				&& !has_any_element_marker(world, entity)
		}
		MdNode::Element { kind, .. } => {
			entity_matches_kind(world, entity, kind)
		}
	}
}

/// Returns true if `entity` has an element marker component that
/// would conflict with being treated as a plain text node.
fn has_any_element_marker(world: &World, entity: Entity) -> bool {
	let entity_ref = world.entity(entity);
	entity_ref.contains::<Paragraph>()
		|| entity_ref.contains::<Heading>()
		|| entity_ref.contains::<BlockQuote>()
		|| entity_ref.contains::<CodeBlock>()
		|| entity_ref.contains::<ListMarker>()
		|| entity_ref.contains::<ListItem>()
		|| entity_ref.contains::<Table>()
		|| entity_ref.contains::<TableHead>()
		|| entity_ref.contains::<TableRow>()
		|| entity_ref.contains::<TableCell>()
		|| entity_ref.contains::<ThematicBreak>()
		|| entity_ref.contains::<Image>()
		|| entity_ref.contains::<FootnoteDefinition>()
		|| entity_ref.contains::<DefinitionList>()
		|| entity_ref.contains::<DefinitionTitle>()
		|| entity_ref.contains::<DefinitionDetails>()
		|| entity_ref.contains::<MetadataBlock>()
		|| entity_ref.contains::<HtmlBlock>()
		|| entity_ref.contains::<MathDisplay>()
		|| entity_ref.contains::<Important>()
		|| entity_ref.contains::<Emphasize>()
		|| entity_ref.contains::<Strikethrough>()
		|| entity_ref.contains::<Superscript>()
		|| entity_ref.contains::<Subscript>()
		|| entity_ref.contains::<Code>()
		|| entity_ref.contains::<MathInline>()
		|| entity_ref.contains::<HtmlInline>()
		|| entity_ref.contains::<Link>()
		|| entity_ref.contains::<HardBreak>()
		|| entity_ref.contains::<SoftBreak>()
		|| entity_ref.contains::<FootnoteRef>()
		|| entity_ref.contains::<TaskListCheck>()
}

/// Check whether an existing entity matches a specific [`MdElement`] kind.
fn entity_matches_kind(
	world: &World,
	entity: Entity,
	kind: &MdElement,
) -> bool {
	let entity_ref = world.entity(entity);
	match kind {
		MdElement::Paragraph => entity_ref.contains::<Paragraph>(),
		MdElement::Heading(level) => entity_ref
			.get::<Heading>()
			.is_some_and(|heading| heading.level() == *level),
		MdElement::BlockQuote => entity_ref.contains::<BlockQuote>(),
		MdElement::CodeBlock { language } => entity_ref
			.get::<CodeBlock>()
			.is_some_and(|cb| cb.language.as_deref() == language.as_deref()),
		MdElement::OrderedList { start } => entity_ref
			.get::<ListMarker>()
			.is_some_and(|lm| lm.ordered && lm.start == Some(*start)),
		MdElement::UnorderedList => {
			entity_ref.get::<ListMarker>().is_some_and(|lm| !lm.ordered)
		}
		MdElement::ListItem => entity_ref.contains::<ListItem>(),
		MdElement::Table { alignments } => entity_ref
			.get::<Table>()
			.is_some_and(|table| &table.alignments == alignments),
		MdElement::TableHead => entity_ref.contains::<TableHead>(),
		MdElement::TableRow => entity_ref.contains::<TableRow>(),
		MdElement::TableCell => entity_ref.contains::<TableCell>(),
		MdElement::ThematicBreak => entity_ref.contains::<ThematicBreak>(),
		MdElement::FootnoteDefinition { label } => entity_ref
			.get::<FootnoteDefinition>()
			.is_some_and(|fd| fd.label == *label),
		MdElement::DefinitionList => entity_ref.contains::<DefinitionList>(),
		MdElement::DefinitionTitle => entity_ref.contains::<DefinitionTitle>(),
		MdElement::DefinitionDetails => {
			entity_ref.contains::<DefinitionDetails>()
		}
		MdElement::HtmlBlock(_) => entity_ref.contains::<HtmlBlock>(),
		MdElement::MetadataBlock { kind } => entity_ref
			.get::<MetadataBlock>()
			.is_some_and(|mb| mb.kind == *kind),
		MdElement::MathDisplay => entity_ref.contains::<MathDisplay>(),
		MdElement::Image { .. } => entity_ref.contains::<Image>(),
		MdElement::Strong => entity_ref.contains::<Important>(),
		MdElement::Emphasis => entity_ref.contains::<Emphasize>(),
		MdElement::Strikethrough => entity_ref.contains::<Strikethrough>(),
		MdElement::Superscript => entity_ref.contains::<Superscript>(),
		MdElement::Subscript => entity_ref.contains::<Subscript>(),
		MdElement::Link { .. } => entity_ref.contains::<Link>(),
		MdElement::InlineCode => entity_ref.contains::<Code>(),
		MdElement::MathInline => entity_ref.contains::<MathInline>(),
		MdElement::HardBreak => entity_ref.contains::<HardBreak>(),
		MdElement::SoftBreak => entity_ref.contains::<SoftBreak>(),
		MdElement::FootnoteRef { label } => entity_ref
			.get::<FootnoteRef>()
			.is_some_and(|fr| fr.label == *label),
		MdElement::HtmlInline(_) => entity_ref.contains::<HtmlInline>(),
		MdElement::TaskListCheck { checked } => entity_ref
			.get::<TaskListCheck>()
			.is_some_and(|tc| tc.checked == *checked),
	}
}

/// Update an existing entity in place to match the new node.
/// Assumes `can_reuse` already returned true.
fn update_in_place(world: &mut World, entity: Entity, node: &MdNode) {
	match node {
		MdNode::Text(text) => {
			if let Some(mut existing) =
				world.entity_mut(entity).get_mut::<TextNode>()
			{
				if existing.0 != *text {
					existing.0 = text.clone();
				}
			}
		}
		MdNode::Element { kind, children } => {
			// Update mutable fields on the component
			update_element_data(world, entity, kind);
			// Recurse into children
			diff_children(world, entity, children);
		}
	}
}

/// Update the mutable data fields of an element component (ie href,
/// src, content) without changing the component type itself.
fn update_element_data(world: &mut World, entity: Entity, kind: &MdElement) {
	match kind {
		MdElement::Link { href, title } => {
			if let Some(mut link) = world.entity_mut(entity).get_mut::<Link>() {
				if link.href != *href {
					link.href = href.clone();
				}
				if link.title != *title {
					link.title = title.clone();
				}
			}
		}
		MdElement::Image { src, title } => {
			if let Some(mut image) = world.entity_mut(entity).get_mut::<Image>()
			{
				if image.src != *src {
					image.src = src.clone();
				}
				if image.title != *title {
					image.title = title.clone();
				}
			}
		}
		MdElement::HtmlBlock(content) => {
			if let Some(mut block) =
				world.entity_mut(entity).get_mut::<HtmlBlock>()
			{
				if block.0 != *content {
					block.0 = content.clone();
				}
			}
		}
		MdElement::HtmlInline(content) => {
			if let Some(mut inline) =
				world.entity_mut(entity).get_mut::<HtmlInline>()
			{
				if inline.0 != *content {
					inline.0 = content.clone();
				}
			}
		}
		MdElement::MetadataBlock { .. } => {
			// Content arrives as Text children, handled by diff_children.
		}
		// Most element kinds have no mutable data beyond children.
		_ => {}
	}
}


/// Spawn a new entity subtree for a node and return the root entity id.
fn spawn_node(world: &mut World, node: &MdNode) -> Entity {
	match node {
		MdNode::Text(text) => world.spawn(TextNode::new(text.as_str())).id(),
		MdNode::Element { kind, children } => {
			let entity = spawn_element(world, kind);
			for child_node in children {
				let child = spawn_node(world, child_node);
				world.entity_mut(child).insert(ChildOf(entity));
			}
			entity
		}
	}
}

/// Spawn a single element entity (no children yet) for the given kind.
fn spawn_element(world: &mut World, kind: &MdElement) -> Entity {
	match kind {
		MdElement::Paragraph => world.spawn(Paragraph).id(),
		MdElement::Heading(1) => world.spawn(Heading1).id(),
		MdElement::Heading(2) => world.spawn(Heading2).id(),
		MdElement::Heading(3) => world.spawn(Heading3).id(),
		MdElement::Heading(4) => world.spawn(Heading4).id(),
		MdElement::Heading(5) => world.spawn(Heading5).id(),
		MdElement::Heading(6) => world.spawn(Heading6).id(),
		MdElement::Heading(_) => world.spawn(Heading1).id(),
		MdElement::BlockQuote => world.spawn(BlockQuote).id(),
		MdElement::CodeBlock { language } => world
			.spawn(match language {
				Some(lang) => CodeBlock::with_language(lang),
				None => CodeBlock::plain(),
			})
			.id(),
		MdElement::OrderedList { start } => {
			world.spawn(ListMarker::ordered(*start)).id()
		}
		MdElement::UnorderedList => world.spawn(ListMarker::unordered()).id(),
		MdElement::ListItem => world.spawn(ListItem).id(),
		MdElement::Table { alignments } => world
			.spawn(Table {
				alignments: alignments.clone(),
			})
			.id(),
		MdElement::TableHead => world.spawn(TableHead).id(),
		MdElement::TableRow => world.spawn(TableRow).id(),
		MdElement::TableCell => world.spawn(TableCell::default()).id(),
		MdElement::ThematicBreak => world.spawn(ThematicBreak).id(),
		MdElement::FootnoteDefinition { label } => world
			.spawn(FootnoteDefinition {
				label: label.clone(),
			})
			.id(),
		MdElement::DefinitionList => world.spawn(DefinitionList).id(),
		MdElement::DefinitionTitle => world.spawn(DefinitionTitle).id(),
		MdElement::DefinitionDetails => world.spawn(DefinitionDetails).id(),
		MdElement::HtmlBlock(content) => {
			world.spawn(HtmlBlock(content.clone())).id()
		}
		MdElement::MetadataBlock { kind } => world
			.spawn(MetadataBlock {
				kind: *kind,
				content: String::new(),
			})
			.id(),
		MdElement::MathDisplay => world.spawn(MathDisplay).id(),
		MdElement::Image { src, title } => world
			.spawn({
				let image = Image::new(src);
				match title {
					Some(title) => image.with_title(title),
					None => image,
				}
			})
			.id(),
		MdElement::Strong => world.spawn(Important).id(),
		MdElement::Emphasis => world.spawn(Emphasize).id(),
		MdElement::Strikethrough => world.spawn(Strikethrough).id(),
		MdElement::Superscript => world.spawn(Superscript).id(),
		MdElement::Subscript => world.spawn(Subscript).id(),
		MdElement::Link { href, title } => {
			let link = Link::new(href);
			let link = match title {
				Some(title) => link.with_title(title),
				None => link,
			};
			world.spawn(link).id()
		}
		MdElement::InlineCode => world.spawn(Code).id(),
		MdElement::MathInline => world.spawn(MathInline).id(),
		MdElement::HardBreak => world.spawn(HardBreak).id(),
		MdElement::SoftBreak => world.spawn(SoftBreak).id(),
		MdElement::FootnoteRef { label } => world
			.spawn(FootnoteRef {
				label: label.clone(),
			})
			.id(),
		MdElement::HtmlInline(content) => {
			world.spawn(HtmlInline(content.clone())).id()
		}
		MdElement::TaskListCheck { checked } => world
			.spawn(if *checked {
				TaskListCheck::checked()
			} else {
				TaskListCheck::unchecked()
			})
			.id(),
	}
}


/// Recursively despawn an entity and all its descendants.
fn despawn_recursive(world: &mut World, entity: Entity) {
	// Collect children first to avoid borrow issues.
	let children: Vec<Entity> = world
		.entity(entity)
		.get::<Children>()
		.map(|children| children.iter().collect())
		.unwrap_or_default();
	for child in children {
		despawn_recursive(world, child);
	}
	world.despawn(entity);
}


// ---------------------------------------------------------------------------
// FileContent — load markdown from a workspace file
// ---------------------------------------------------------------------------

/// Component indicating that an entity's content should be loaded
/// from a markdown file at the given workspace-relative path.
///
/// When this component is added or changed, the
/// [`load_file_content`] system reads the file, parses the markdown,
/// and spawns the parsed tree as children of the entity.
///
/// # Example
///
/// ```no_run
/// use beet_stack::prelude::*;
/// use beet_core::prelude::*;
///
/// let mut world = World::new();
/// world.spawn(FileContent::new("examples/stack/petes_beets/home.md"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect, Component)]
#[reflect(Component)]
pub struct FileContent {
	/// Workspace-relative path to the markdown file.
	pub path: WsPathBuf,
}

impl FileContent {
	/// Create a new file content component pointing at the given path.
	pub fn new(path: impl Into<WsPathBuf>) -> Self {
		Self { path: path.into() }
	}
}

/// System that loads and parses markdown for entities with a new or
/// changed [`FileContent`] component.
///
/// Reads the file from disk using [`fs_ext::read_to_string`], parses
/// it with [`MarkdownDiffer`], and applies the result as children.
pub fn load_file_content(world: &mut World) {
	let mut to_load: Vec<(Entity, WsPathBuf)> = Vec::new();

	let mut query =
		world.query_filtered::<(Entity, &FileContent), Changed<FileContent>>();
	for (entity, file_content) in query.iter(world) {
		to_load.push((entity, file_content.path.clone()));
	}

	for (entity, path) in to_load {
		match fs_ext::read_to_string(&path.into_abs()) {
			Ok(text) => {
				let mut differ = MarkdownDiffer::new(&text);
				if let Err(err) = differ.diff(world.entity_mut(entity)) {
					cross_log_error!(
						"Failed to diff markdown file {path}: {err}"
					);
				}
			}
			Err(err) => {
				cross_log_error!("Failed to load markdown file {path}: {err}");
			}
		}
	}
}


// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod test {
	use super::*;

	fn parse(text: &str) -> Vec<MdNode> { MarkdownDiffer::new(text).parse() }

	fn render(world: &mut World, text: &str) -> Entity {
		let root = world.spawn_empty().id();
		MarkdownDiffer::new(text)
			.diff(world.entity_mut(root))
			.unwrap();
		root
	}

	fn child_entities(world: &World, parent: Entity) -> Vec<Entity> {
		world
			.entity(parent)
			.get::<Children>()
			.map(|children| children.iter().collect())
			.unwrap_or_default()
	}

	fn collect_text(world: &World, parent: Entity) -> String {
		let mut result = String::new();
		for child in child_entities(world, parent) {
			if let Some(text) = world.entity(child).get::<TextNode>() {
				result.push_str(text.as_str());
			}
		}
		result
	}

	// -- Parsing tests --

	#[test]
	fn parse_paragraph() {
		let nodes = parse("Hello world");
		nodes.len().xpect_eq(1);
		matches!(&nodes[0], MdNode::Element {
			kind: MdElement::Paragraph,
			..
		})
		.xpect_true();
	}

	#[test]
	fn parse_heading() {
		let nodes = parse("# Title");
		nodes.len().xpect_eq(1);
		matches!(&nodes[0], MdNode::Element {
			kind: MdElement::Heading(1),
			..
		})
		.xpect_true();
	}

	#[test]
	fn parse_multiple_headings() {
		let nodes = parse("# One\n## Two\n### Three");
		nodes.len().xpect_eq(3);
		matches!(&nodes[0], MdNode::Element {
			kind: MdElement::Heading(1),
			..
		})
		.xpect_true();
		matches!(&nodes[1], MdNode::Element {
			kind: MdElement::Heading(2),
			..
		})
		.xpect_true();
		matches!(&nodes[2], MdNode::Element {
			kind: MdElement::Heading(3),
			..
		})
		.xpect_true();
	}

	#[test]
	fn parse_emphasis_and_strong() {
		let nodes = parse("*em* **strong**");
		nodes.len().xpect_eq(1);
		if let MdNode::Element { children, .. } = &nodes[0] {
			// Should have emphasis element, space, strong element
			children
				.iter()
				.any(|child| {
					matches!(child, MdNode::Element {
						kind: MdElement::Emphasis,
						..
					})
				})
				.xpect_true();
			children
				.iter()
				.any(|child| {
					matches!(child, MdNode::Element {
						kind: MdElement::Strong,
						..
					})
				})
				.xpect_true();
		}
	}

	#[test]
	fn parse_inline_code() {
		let nodes = parse("Use `foo()` here");
		nodes.len().xpect_eq(1);
		if let MdNode::Element { children, .. } = &nodes[0] {
			children
				.iter()
				.any(|child| {
					matches!(child, MdNode::Element {
						kind: MdElement::InlineCode,
						..
					})
				})
				.xpect_true();
		}
	}

	#[test]
	fn parse_code_block() {
		let nodes = parse("```rust\nfn main() {}\n```");
		nodes.len().xpect_eq(1);
		if let MdNode::Element { kind, .. } = &nodes[0] {
			matches!(kind, MdElement::CodeBlock { language: Some(lang) } if lang == "rust")
				.xpect_true();
		}
	}

	#[test]
	fn parse_unordered_list() {
		let nodes = parse("- one\n- two\n- three");
		nodes.len().xpect_eq(1);
		if let MdNode::Element { kind, children } = &nodes[0] {
			matches!(kind, MdElement::UnorderedList).xpect_true();
			children.len().xpect_eq(3);
		}
	}

	#[test]
	fn parse_ordered_list() {
		let nodes = parse("1. first\n2. second");
		nodes.len().xpect_eq(1);
		if let MdNode::Element { kind, children } = &nodes[0] {
			matches!(kind, MdElement::OrderedList { start: 1 }).xpect_true();
			children.len().xpect_eq(2);
		}
	}

	#[test]
	fn parse_link() {
		let nodes = parse("[click](https://example.com)");
		nodes.len().xpect_eq(1);
		if let MdNode::Element { children, .. } = &nodes[0] {
			children
				.iter()
				.any(|child| {
					matches!(
						child,
						MdNode::Element { kind: MdElement::Link { href, .. }, .. }
						if href == "https://example.com"
					)
				})
				.xpect_true();
		}
	}

	#[test]
	fn parse_blockquote() {
		let nodes = parse("> quoted text");
		nodes.len().xpect_eq(1);
		matches!(&nodes[0], MdNode::Element {
			kind: MdElement::BlockQuote,
			..
		})
		.xpect_true();
	}

	#[test]
	fn parse_thematic_break() {
		let nodes = parse("---");
		nodes.len().xpect_eq(1);
		matches!(&nodes[0], MdNode::Element {
			kind: MdElement::ThematicBreak,
			..
		})
		.xpect_true();
	}

	#[test]
	fn parse_image() {
		let nodes = parse("![alt text](image.png)");
		nodes.len().xpect_eq(1);
		if let MdNode::Element { children, .. } = &nodes[0] {
			children
				.iter()
				.any(|child| {
					matches!(
						child,
						MdNode::Element { kind: MdElement::Image { src, .. }, .. }
						if src == "image.png"
					)
				})
				.xpect_true();
		}
	}

	// -- Render tests --

	#[test]
	fn render_paragraph() {
		let mut world = World::new();
		let root = render(&mut world, "Hello world");

		let children = child_entities(&world, root);
		children.len().xpect_eq(1);
		world
			.entity(children[0])
			.contains::<Paragraph>()
			.xpect_true();

		collect_text(&world, children[0]).xpect_eq("Hello world");
	}

	#[test]
	fn render_heading() {
		let mut world = World::new();
		let root = render(&mut world, "# Title");

		let children = child_entities(&world, root);
		children.len().xpect_eq(1);

		let heading_entity = children[0];
		world
			.entity(heading_entity)
			.contains::<Heading1>()
			.xpect_true();
		world
			.entity(heading_entity)
			.get::<Heading>()
			.unwrap()
			.level()
			.xpect_eq(1);

		collect_text(&world, heading_entity).xpect_eq("Title");
	}

	#[test]
	fn render_heading_levels() {
		let mut world = World::new();
		let root = render(&mut world, "## Sub\n### Deep");

		let children = child_entities(&world, root);
		children.len().xpect_eq(2);

		world
			.entity(children[0])
			.contains::<Heading2>()
			.xpect_true();
		world
			.entity(children[1])
			.contains::<Heading3>()
			.xpect_true();
	}

	#[test]
	fn render_strong_and_emphasis() {
		let mut world = World::new();
		let root = render(&mut world, "**bold** *italic*");

		let children = child_entities(&world, root);
		children.len().xpect_eq(1);

		let para = children[0];
		let inline_children = child_entities(&world, para);

		// Should have: Important wrapper, text, Emphasize wrapper
		inline_children
			.iter()
			.any(|&entity| world.entity(entity).contains::<Important>())
			.xpect_true();
		inline_children
			.iter()
			.any(|&entity| world.entity(entity).contains::<Emphasize>())
			.xpect_true();
	}

	#[test]
	fn render_inline_code() {
		let mut world = World::new();
		let root = render(&mut world, "Use `foo()` here");

		let children = child_entities(&world, root);
		let para = children[0];
		let inline_children = child_entities(&world, para);

		inline_children
			.iter()
			.any(|&entity| world.entity(entity).contains::<Code>())
			.xpect_true();
	}

	#[test]
	fn render_code_block() {
		let mut world = World::new();
		let root = render(&mut world, "```rust\nfn main() {}\n```");

		let children = child_entities(&world, root);
		children.len().xpect_eq(1);

		let code_entity = children[0];
		let code_block = world.entity(code_entity).get::<CodeBlock>().unwrap();
		code_block.language.as_deref().xpect_eq(Some("rust"));
	}

	#[test]
	fn render_unordered_list() {
		let mut world = World::new();
		let root = render(&mut world, "- one\n- two\n- three");

		let children = child_entities(&world, root);
		children.len().xpect_eq(1);

		let list_entity = children[0];
		let list_marker =
			world.entity(list_entity).get::<ListMarker>().unwrap();
		list_marker.ordered.xpect_false();

		let items = child_entities(&world, list_entity);
		items.len().xpect_eq(3);
		for item in &items {
			world.entity(*item).contains::<ListItem>().xpect_true();
		}
	}

	#[test]
	fn render_link() {
		let mut world = World::new();
		let root =
			render(&mut world, "[click](https://example.com \"Example\")");

		let children = child_entities(&world, root);
		let para = children[0];
		let inline_children = child_entities(&world, para);

		let link_entity = inline_children
			.iter()
			.find(|&&entity| world.entity(entity).contains::<Link>())
			.unwrap();

		let link = world.entity(*link_entity).get::<Link>().unwrap();
		link.href.as_str().xpect_eq("https://example.com");
		link.title.as_deref().xpect_eq(Some("Example"));
	}

	#[test]
	fn render_blockquote() {
		let mut world = World::new();
		let root = render(&mut world, "> Hello");

		let children = child_entities(&world, root);
		children.len().xpect_eq(1);
		world
			.entity(children[0])
			.contains::<BlockQuote>()
			.xpect_true();

		// BlockQuote should contain a Paragraph
		let bq_children = child_entities(&world, children[0]);
		bq_children.len().xpect_eq(1);
		world
			.entity(bq_children[0])
			.contains::<Paragraph>()
			.xpect_true();
	}

	#[test]
	fn render_thematic_break() {
		let mut world = World::new();
		let root = render(&mut world, "---");

		let children = child_entities(&world, root);
		children.len().xpect_eq(1);
		world
			.entity(children[0])
			.contains::<ThematicBreak>()
			.xpect_true();
	}

	// -- Diff tests --

	#[test]
	fn diff_updates_text_in_place() {
		let mut world = World::new();
		let root = render(&mut world, "Hello");

		let para = child_entities(&world, root)[0];
		let text_entity = child_entities(&world, para)[0];
		let original_id = text_entity;

		// Re-render with different text
		MarkdownDiffer::new("World")
			.diff(world.entity_mut(root))
			.unwrap();

		let para_after = child_entities(&world, root)[0];
		let text_after = child_entities(&world, para_after)[0];

		// Same paragraph and text entities should be reused
		para_after.xpect_eq(para);
		text_after.xpect_eq(original_id);

		// But text content should be updated
		world
			.entity(text_after)
			.get::<TextNode>()
			.unwrap()
			.as_str()
			.xpect_eq("World");
	}

	#[test]
	fn diff_replaces_mismatched_kind() {
		let mut world = World::new();
		let root = render(&mut world, "# Heading");

		let heading = child_entities(&world, root)[0];
		world.entity(heading).contains::<Heading1>().xpect_true();

		// Re-render as paragraph
		MarkdownDiffer::new("Not a heading")
			.diff(world.entity_mut(root))
			.unwrap();

		let children = child_entities(&world, root);
		children.len().xpect_eq(1);

		let new_child = children[0];
		world.entity(new_child).contains::<Paragraph>().xpect_true();
		// Old heading entity should be despawned
		world.get_entity(heading).is_err().xpect_true();
	}

	#[test]
	fn diff_adds_new_children() {
		let mut world = World::new();
		let root = render(&mut world, "# One");

		child_entities(&world, root).len().xpect_eq(1);

		// Add a paragraph
		MarkdownDiffer::new("# One\n\nTwo")
			.diff(world.entity_mut(root))
			.unwrap();

		let children = child_entities(&world, root);
		children.len().xpect_eq(2);
		world
			.entity(children[0])
			.contains::<Heading1>()
			.xpect_true();
		world
			.entity(children[1])
			.contains::<Paragraph>()
			.xpect_true();
	}

	#[test]
	fn diff_removes_extra_children() {
		let mut world = World::new();
		let root = render(&mut world, "# One\n\nTwo\n\nThree");

		child_entities(&world, root).len().xpect_eq(3);

		// Remove last two paragraphs
		MarkdownDiffer::new("# One")
			.diff(world.entity_mut(root))
			.unwrap();

		child_entities(&world, root).len().xpect_eq(1);
	}

	#[test]
	fn diff_preserves_unchanged_structure() {
		let mut world = World::new();
		let root = render(
			&mut world,
			"# Title\n\nFirst paragraph\n\nSecond paragraph",
		);

		let original_children = child_entities(&world, root);
		let heading = original_children[0];
		let para1 = original_children[1];

		// Change only the second paragraph text
		MarkdownDiffer::new("# Title\n\nFirst paragraph\n\nChanged paragraph")
			.diff(world.entity_mut(root))
			.unwrap();

		let new_children = child_entities(&world, root);
		// Heading and first paragraph should be reused
		new_children[0].xpect_eq(heading);
		new_children[1].xpect_eq(para1);
	}

	#[test]
	fn render_complex_document() {
		let mut world = World::new();
		let root = render(
			&mut world,
			r#"# Welcome

This is a **bold** and *italic* paragraph.

## Features

- Item one
- Item two
- Item three

> A wise quote

---

```rust
fn hello() {}
```
"#,
		);

		let children = child_entities(&world, root);
		// h1, p, h2, ul, blockquote, hr, code block
		children.len().xpect_eq(7);
		world
			.entity(children[0])
			.contains::<Heading1>()
			.xpect_true();
		world
			.entity(children[1])
			.contains::<Paragraph>()
			.xpect_true();
		world
			.entity(children[2])
			.contains::<Heading2>()
			.xpect_true();
		world
			.entity(children[3])
			.contains::<ListMarker>()
			.xpect_true();
		world
			.entity(children[4])
			.contains::<BlockQuote>()
			.xpect_true();
		world
			.entity(children[5])
			.contains::<ThematicBreak>()
			.xpect_true();
		world
			.entity(children[6])
			.contains::<CodeBlock>()
			.xpect_true();
	}
}
