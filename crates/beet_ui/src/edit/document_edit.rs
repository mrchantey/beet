use crate::prelude::*;
use beet_core::prelude::*;

/// A cell an edit addresses: a table cell or a workbook cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellAddress {
	/// A table cell, `t<table>r<row>c<cell>`.
	Table(TableCellAddress),
	/// A workbook cell, `<sheet>!<A1>`.
	Sheet(SheetCellAddress),
}

impl CellAddress {
	/// Parses either form, a table cell or a workbook cell.
	pub fn parse(text: &str) -> Result<Self> {
		TableCellAddress::parse(text)
			.map(Self::Table)
			.or_else(|_| SheetCellAddress::parse(text).map(Self::Sheet))
			.map_err(|_| {
				bevyhow!(
					"`{text}` is not a cell, expected `t<n>r<n>c<n>` or `<sheet>!<A1>`"
				)
			})
	}
}

impl core::fmt::Display for CellAddress {
	fn fmt(
		&self,
		formatter: &mut core::fmt::Formatter<'_>,
	) -> core::fmt::Result {
		match self {
			Self::Table(address) => address.fmt(formatter),
			Self::Sheet(address) => address.fmt(formatter),
		}
	}
}

/// Replaces a cell's paragraphs with one per line of `text`: the first
/// paragraph keeps its place and the look of its first words, its other
/// words go, every other paragraph goes, and the rest of the lines follow it
/// as new paragraphs. A cell holding no paragraph, ie a markdown table's or
/// a workbook's, takes the text as its own words. A locked cell is refused,
/// as is a formula into a workbook cell. Answers the cell written, a merged
/// cell's range's first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetText {
	/// The cell.
	pub cell: CellAddress,
	/// The text, a line a paragraph.
	pub text: String,
}

/// Adds one paragraph per line of `text` after a cell's own, in the tag of
/// its first, for a box whose prompt shares the cell with its answer.
/// Answers the cell written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppendText {
	/// The cell.
	pub cell: CellAddress,
	/// The text, a line a paragraph.
	pub text: String,
}

/// Checks every checkbox whose paragraph carries `label`, answering how many.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckBox {
	/// The label beside the box.
	pub label: String,
}

/// Removes every paragraph carrying `text`, ie a red instruction sentence,
/// answering how many. A cell's last paragraph is emptied rather than
/// removed, since a cell holds one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveParagraphs {
	/// The text the paragraphs carry.
	pub text: String,
}

/// Replaces `old` with `new` inside the words that carry it, touching only
/// the pieces of text the match covers, so a highlighted placeholder stays
/// highlighted. Answers how many it replaced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaceText {
	/// The text as the document carries it.
	pub old: String,
	/// The replacement.
	pub new: String,
}

impl SetText {
	/// Applies the edit to the document under `root`.
	pub fn apply_to(&self, world: &mut World, root: Entity) -> Result<Entity> {
		let cell = DocumentEdit::cell(world, root, &self.cell)?;
		if let CellAddress::Sheet(address) = &self.cell
			&& self.text.trim().starts_with('=')
		{
			bevybail!("{address}: a formula is not a value");
		}
		let lines = self.text.split('\n').collect::<Vec<_>>();
		let blocks =
			world.with_state::<ReaderText, _>(|text| text.blocks(cell));
		let Some((first, rest)) = blocks.split_first() else {
			DocumentEdit::set_words(world, cell, &self.text);
			return cell.xok();
		};
		DocumentEdit::set_words(world, *first, lines[0]);
		for block in rest {
			world.entity_mut(*block).despawn();
		}
		DocumentEdit::insert_after(world, *first, &lines[1..]);
		cell.xok()
	}
}

impl AppendText {
	/// Applies the edit to the document under `root`.
	pub fn apply_to(&self, world: &mut World, root: Entity) -> Result<Entity> {
		let cell = DocumentEdit::cell(world, root, &self.cell)?;
		let lines = self.text.split('\n').collect::<Vec<_>>();
		let blocks =
			world.with_state::<ReaderText, _>(|text| text.blocks(cell));
		match blocks.last() {
			Some(last) => DocumentEdit::insert_after(world, *last, &lines),
			// a cell of words alone takes each line after a break
			None => {
				for line in lines {
					world.spawn((Element::new("br"), ChildOf(cell)));
					world.spawn((Value::str(line), ChildOf(cell)));
				}
			}
		}
		cell.xok()
	}
}

impl CheckBox {
	/// Applies the edit to the document under `root`.
	pub fn apply_to(&self, world: &mut World, root: Entity) -> Result<usize> {
		let needle = DocumentEdit::needle(&self.label)?;
		let boxes = world.with_state::<DocumentEdit, _>(|edit| {
			edit.descendants(root)
				.into_iter()
				.filter(|entity| edit.text.checkbox(*entity).is_some())
				.filter(|entity| {
					let paragraph = edit.paragraph(*entity).unwrap_or(root);
					DocumentEdit::plain(&edit.text.text(paragraph))
						.contains(&needle)
				})
				.map(|entity| (entity, edit.has_attribute(entity, "checked")))
				.collect::<Vec<_>>()
		});
		for (entity, checked) in &boxes {
			world.entity_mut(*entity).insert(Value::Bool(true));
			if !checked {
				world.spawn((
					AttributeOf::new(*entity),
					Attribute::new("checked"),
					Value::Null,
				));
			}
		}
		boxes.len().xok()
	}
}

impl RemoveParagraphs {
	/// Applies the edit to the document under `root`.
	pub fn apply_to(&self, world: &mut World, root: Entity) -> Result<usize> {
		let needle = DocumentEdit::needle(&self.text)?;
		let (matched, emptied) = world.with_state::<DocumentEdit, _>(|edit| {
			let matched = edit
				.paragraphs(root)
				.into_iter()
				.filter(|block| {
					DocumentEdit::plain(&edit.text.text(*block))
						.contains(&needle)
				})
				.collect::<Vec<_>>();
			// a cell whose every paragraph goes keeps its last, emptied
			let emptied = matched
				.iter()
				.filter_map(|block| {
					let cell = edit.cell_of(*block)?;
					let blocks = edit.text.blocks(cell);
					(blocks.iter().all(|other| matched.contains(other))
						&& blocks.last() == Some(block))
					.then_some(*block)
				})
				.collect::<Vec<_>>();
			(matched, emptied)
		});
		for block in &matched {
			match emptied.contains(block) {
				true => DocumentEdit::empty(world, *block),
				false => world.entity_mut(*block).despawn(),
			}
		}
		matched.len().xok()
	}
}

impl ReplaceText {
	/// The most replacements in one paragraph, since a replacement may
	/// itself carry the text it replaced.
	const MOST: usize = 50;

	/// Applies the edit to the document under `root`.
	pub fn apply_to(&self, world: &mut World, root: Entity) -> Result<usize> {
		let needle = DocumentEdit::needle(&self.old)?;
		let paragraphs = world.with_state::<DocumentEdit, _>(|edit| {
			edit.paragraphs(root)
				.into_iter()
				.filter(|block| edit.leaf(*block))
				.map(|block| edit.text_nodes(block))
				.collect::<Vec<_>>()
		});
		let mut replaced = 0;
		for nodes in paragraphs {
			for _ in 0..Self::MOST {
				let texts = nodes
					.iter()
					.map(|node| {
						world
							.entity(*node)
							.get::<Value>()
							.map(|value| {
								DocumentEdit::plain(&value.to_string())
							})
							.unwrap_or_default()
					})
					.collect::<Vec<_>>();
				let Some(at) = texts.concat().find(&needle) else {
					break;
				};
				let end = at + needle.len();
				let mut position = 0;
				let mut placed = false;
				for (node, text) in nodes.iter().zip(&texts) {
					let (start, stop) = (position, position + text.len());
					position = stop;
					if stop <= at || start >= end {
						continue;
					}
					let before =
						&text[..at.saturating_sub(start).min(text.len())];
					let after = &text[(end - start).min(text.len())..];
					let written = match placed {
						true => format!("{before}{after}"),
						false => format!("{before}{}{after}", self.new),
					};
					world.entity_mut(*node).insert(Value::Str(written.into()));
					placed = true;
				}
				replaced += 1;
			}
		}
		replaced.xok()
	}
}

/// Every edit is a command on a document's root, its answer dropped.
macro_rules! entity_command {
	($($edit:ty),*) => {$(
		impl EntityCommand for $edit {
			type Out = Result;
			fn apply(self, mut entity: EntityWorldMut) -> Result {
				let root = entity.id();
				entity.world_scope(|world| self.apply_to(world, root).map(drop))
			}
		}
	)*};
}
entity_command!(SetText, AppendText, CheckBox, RemoveParagraphs, ReplaceText);

/// The reads an edit plans with: paragraphs, text nodes and the cell around.
#[derive(SystemParam)]
struct DocumentEdit<'w, 's> {
	text: ReaderText<'w, 's>,
	nodes: Query<
		'w,
		's,
		(
			Option<&'static Element>,
			Option<&'static Value>,
			Option<&'static Children>,
		),
	>,
	parents: Query<'w, 's, &'static ChildOf>,
	attributes: AttributeQuery<'w, 's>,
}

impl DocumentEdit<'_, '_> {
	/// The cell at `address` under `root`, refused when absent or locked.
	fn cell(
		world: &mut World,
		root: Entity,
		address: &CellAddress,
	) -> Result<Entity> {
		world.with_state::<TableCells, _>(|cells| {
			let cell = match address {
				CellAddress::Table(table) => cells.table_cell(root, *table),
				CellAddress::Sheet(sheet) => cells.sheet_cell(root, sheet),
			}
			.ok_or_else(|| bevyhow!("the document has no cell {address}"))?;
			if cells.is_locked(cell) {
				bevybail!(
					"{address} is locked; only an unlocked cell takes a value"
				);
			}
			cell.xok()
		})
	}

	/// Text as matching reads it, a no-break space as a space.
	fn plain(text: &str) -> String { text.replace('\u{a0}', " ") }

	/// Refuses an empty match, which every paragraph carries.
	fn needle(text: &str) -> Result<String> {
		match Self::plain(text) {
			needle if needle.trim().is_empty() => {
				bevybail!("an empty text matches every paragraph")
			}
			needle => needle.xok(),
		}
	}

	fn tag(&self, entity: Entity) -> Option<&str> {
		self.nodes
			.get(entity)
			.ok()
			.and_then(|(element, ..)| element)
			.map(Element::tag)
	}

	fn children(&self, entity: Entity) -> Vec<Entity> {
		self.nodes
			.get(entity)
			.ok()
			.and_then(|(.., children)| children)
			.map(|children| children.to_vec())
			.unwrap_or_default()
	}

	fn descendants(&self, entity: Entity) -> Vec<Entity> {
		let mut out = Vec::new();
		for child in self.children(entity) {
			out.push(child);
			out.extend(self.descendants(child));
		}
		out
	}

	fn has_attribute(&self, entity: Entity, key: &str) -> bool {
		self.attributes.find(entity, key).is_some()
	}

	/// Whether `entity` is a text node a reader reads.
	fn is_text(&self, entity: Entity) -> bool {
		matches!(self.nodes.get(entity), Ok((None, Some(Value::Str(_)), _)))
	}

	/// Whether `entity` holds anything a reader reads: an element, or
	/// text, itself or below.
	fn is_content(&self, entity: Entity) -> bool {
		self.tag(entity).is_some()
			|| self.is_text(entity)
			|| self
				.children(entity)
				.into_iter()
				.any(|child| self.is_content(child))
	}

	/// The paragraph around `entity`, the nearest [`ReaderText::BLOCKS`]
	/// ancestor.
	fn paragraph(&self, entity: Entity) -> Option<Entity> {
		self.parents.iter_ancestors(entity).find(|ancestor| {
			self.tag(*ancestor)
				.is_some_and(|tag| ReaderText::BLOCKS.contains(&tag))
		})
	}

	/// The table cell around `entity`.
	fn cell_of(&self, entity: Entity) -> Option<Entity> {
		self.parents
			.iter_ancestors(entity)
			.find(|ancestor| matches!(self.tag(*ancestor), Some("td" | "th")))
	}

	/// Every paragraph under `root` at any depth, tables included, a
	/// paragraph inside another left to the outer.
	fn paragraphs(&self, root: Entity) -> Vec<Entity> {
		let mut out = Vec::new();
		self.push_paragraphs(root, &mut out);
		out
	}

	fn push_paragraphs(&self, entity: Entity, out: &mut Vec<Entity>) {
		for child in self.children(entity) {
			match self.tag(child) {
				Some(tag) if ReaderText::BLOCKS.contains(&tag) => {
					out.push(child)
				}
				Some(tag) if is_non_visual(tag) => {}
				_ => self.push_paragraphs(child, out),
			}
		}
	}

	/// Whether a paragraph holds no paragraph of its own.
	fn leaf(&self, block: Entity) -> bool { self.paragraphs(block).is_empty() }

	/// The text nodes a reader reads under `entity`, in order.
	fn text_nodes(&self, entity: Entity) -> Vec<Entity> {
		let mut out = Vec::new();
		for child in self.children(entity) {
			match self.tag(child) {
				Some(tag) if is_non_visual(tag) => {}
				_ if self.is_text(child) => out.push(child),
				_ => out.extend(self.text_nodes(child)),
			}
		}
		out
	}

	/// The text nodes a reader reads under `entity` outside any table nested
	/// in it, in order.
	fn own_text_nodes(&self, entity: Entity) -> Vec<Entity> {
		let mut out = Vec::new();
		for child in self.children(entity) {
			match self.tag(child) {
				Some(tag) if is_non_visual(tag) || tag == "table" => {}
				_ if self.is_text(child) => out.push(child),
				_ => out.extend(self.own_text_nodes(child)),
			}
		}
		out
	}

	/// Sets the words of `block`: its first text node keeps its place and
	/// its look and takes `text`, every other piece of content around it
	/// goes, and a block holding no text takes a new text node.
	fn set_words(world: &mut World, block: Entity, text: &str) {
		let plan = world.with_state::<DocumentEdit, _>(|edit| {
			let Some(first) = edit.own_text_nodes(block).into_iter().next()
			else {
				// nothing to keep: every piece of content goes
				return (
					None,
					edit.children(block)
						.into_iter()
						.filter(|child| edit.is_content(*child))
						.collect::<Vec<_>>(),
				);
			};
			let chain = core::iter::once(first)
				.chain(edit.parents.iter_ancestors(first))
				.take_while(|entity| *entity != block)
				.collect::<Vec<_>>();
			// at every level the chain passes, the content beside it goes
			let mut removed = Vec::new();
			for link in &chain {
				let parent = edit.parents.get(*link).map(ChildOf::parent).ok();
				for sibling in parent
					.map(|parent| edit.children(parent))
					.unwrap_or_default()
				{
					if sibling != *link && edit.is_content(sibling) {
						removed.push(sibling);
					}
				}
			}
			(Some(first), removed)
		});
		let (first, removed) = plan;
		for entity in removed {
			if let Ok(entity) = world.get_entity_mut(entity) {
				entity.despawn();
			}
		}
		match first {
			Some(first) => {
				world.entity_mut(first).insert(Value::str(text));
			}
			None => {
				world.spawn((Value::str(text), ChildOf(block)));
			}
		}
	}

	/// Empties `block` of every piece of content it holds.
	fn empty(world: &mut World, block: Entity) {
		let content = world.with_state::<DocumentEdit, _>(|edit| {
			edit.children(block)
				.into_iter()
				.filter(|child| edit.is_content(*child))
				.collect::<Vec<_>>()
		});
		for entity in content {
			world.entity_mut(entity).despawn();
		}
	}

	/// Inserts a paragraph per line after `model`, in its tag, each holding
	/// its line as a text node.
	fn insert_after(world: &mut World, model: Entity, lines: &[&str]) {
		let tag = world
			.entity(model)
			.get::<Element>()
			.map(|element| element.tag().to_string())
			.unwrap_or_else(|| "p".into());
		let Some(parent) =
			world.entity(model).get::<ChildOf>().map(ChildOf::parent)
		else {
			return;
		};
		let mut index = world
			.entity(parent)
			.get::<Children>()
			.and_then(|children| {
				children.iter().position(|child| child == model)
			})
			.unwrap_or_default();
		for line in lines {
			index += 1;
			let paragraph = world
				.spawn((Element::new(tag.as_str()), children![Value::str(
					*line
				)]))
				.id();
			world.entity_mut(parent).insert_child(index, paragraph);
		}
	}
}

#[cfg(test)]
#[cfg(feature = "markdown_parser")]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// The edits a Word form takes, applied to a markdown one: its table's
	/// cells, its task list's box, its instruction and its placeholder.
	#[beet_core::test]
	fn fills_a_markdown_form() {
		let mut world = World::new();
		let root = world.spawn_empty().id();
		MediaParser::new()
			.parse(ParseContext::new(
				&mut world.entity_mut(root),
				&MediaBytes::new_markdown(
					"| Name |  |\n|---|---|\n| Describe it: |  |\n\n\
					 - [ ] Surveys\n- [ ] Focus groups\n\n\
					 (Please delete this sentence once completed)\n\n\
					 The business is <mark>(Insert name)</mark>.\n",
				),
			))
			.unwrap();
		let cell = |text| CellAddress::parse(text).unwrap();
		SetText {
			cell: cell("t1r1c2"),
			text: "Acme Stalls".into(),
		}
		.apply_to(&mut world, root)
		.unwrap();
		AppendText {
			cell: cell("t1r2c1"),
			text: "rents fitted stalls".into(),
		}
		.apply_to(&mut world, root)
		.unwrap();
		CheckBox {
			label: "Surveys".into(),
		}
		.apply_to(&mut world, root)
		.unwrap()
		.xpect_eq(1);
		RemoveParagraphs {
			text: "Please delete this sentence".into(),
		}
		.apply_to(&mut world, root)
		.unwrap()
		.xpect_eq(1);
		ReplaceText {
			old: "(Insert name)".into(),
			new: "Acme".into(),
		}
		.apply_to(&mut world, root)
		.unwrap()
		.xpect_eq(1);
		MarkdownRenderer::new()
			.render(&mut RenderContext::new(root, &mut world))
			.unwrap()
			.to_string()
			.xpect_eq(
				"| Name | Acme Stalls |\n|---|---|\n| Describe it:<br>rents fitted stalls |  |\n\n\
				 - [x] Surveys\n- [ ] Focus groups\n\n\
				 The business is <mark>Acme</mark>.\n",
			);
	}
}
