//! [`DocumentTreeEdit`], the edits that may span several nodes of a
//! document's tree.
use crate::prelude::*;
use beet_core::prelude::*;

/// A cell an edit addresses: a table cell, or with the `ooxml` feature a
/// workbook cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellAddress {
	/// A table cell, `t<table>r<row>c<cell>`.
	Table(TableCellAddress),
	/// A workbook cell, `<sheet>!<A1>`.
	#[cfg(feature = "ooxml")]
	Sheet(SheetCellAddress),
}

impl CellAddress {
	/// Parses either form, a table cell or a workbook cell.
	pub fn parse(text: &str) -> Result<Self> {
		if let Ok(address) = TableCellAddress::parse(text) {
			return Self::Table(address).xok();
		}
		#[cfg(feature = "ooxml")]
		if let Ok(address) = SheetCellAddress::parse(text) {
			return Self::Sheet(address).xok();
		}
		bevybail!(
			"`{text}` is not a cell, expected `t<n>r<n>c<n>` or `<sheet>!<A1>`"
		)
	}
}

impl core::fmt::Display for CellAddress {
	fn fmt(
		&self,
		formatter: &mut core::fmt::Formatter<'_>,
	) -> core::fmt::Result {
		match self {
			Self::Table(address) => address.fmt(formatter),
			#[cfg(feature = "ooxml")]
			Self::Sheet(address) => address.fmt(formatter),
		}
	}
}

/// Edits that may span several nodes of a document's tree: a cell's
/// paragraphs, a phrase split across runs, a box found by the label beside
/// it. A form control's own [`Value`] is the form widgets' to edit; these are
/// for filling a document as a person fills a form, the same edits over a
/// markdown table, an HTML page or a Word file.
///
/// Each method is one edit: it plans from what it reads and queues what it
/// writes, so a caller runs each in its own `world.with_state` and the next
/// sees its result, ie
/// `world.with_state::<DocumentTreeEdit, _>(|mut edit| edit.check_box(root, "Surveys"))`.
/// A cell is addressed by its [`CellAddress`], a checkbox by the label in its
/// paragraph, a paragraph by a text it carries; matching reads a no-break
/// space as a space, since a form's placeholders carry them.
#[derive(SystemParam)]
pub struct DocumentTreeEdit<'w, 's> {
	commands: Commands<'w, 's>,
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
	table_cells: TableCells<'w, 's>,
	#[cfg(feature = "ooxml")]
	sheet_cells: SheetCells<'w, 's>,
}

impl DocumentTreeEdit<'_, '_> {
	/// The most replacements in one paragraph, since a replacement may
	/// itself carry the text it replaced.
	const MOST: usize = 50;

	/// Replaces the cell's paragraphs with one per line of `text`: the first
	/// paragraph keeps its place and the look of its first words, its other
	/// words go, every other paragraph goes, and the rest of the lines follow
	/// it as new paragraphs. A cell holding no paragraph, ie a markdown
	/// table's or a workbook's, takes the text as its own words. A locked
	/// cell is refused, as is a formula into a workbook cell. Answers the cell
	/// written, a merged cell's range's first.
	pub fn set_text(
		&mut self,
		root: Entity,
		cell: &CellAddress,
		text: &str,
	) -> Result<Entity> {
		let entity = self.cell(root, cell)?;
		#[cfg(feature = "ooxml")]
		if let CellAddress::Sheet(address) = cell
			&& text.trim().starts_with('=')
		{
			bevybail!("{address}: a formula is not a value");
		}
		let lines = text.split('\n').collect::<Vec<_>>();
		let blocks = self.text.blocks(entity);
		let Some((first, rest)) = blocks.split_first() else {
			self.set_words(entity, text);
			return entity.xok();
		};
		self.set_words(*first, lines[0]);
		for block in rest {
			self.commands.entity(*block).try_despawn();
		}
		self.insert_after(*first, &lines[1..]);
		entity.xok()
	}

	/// Adds one paragraph per line of `text` after the cell's own, in the
	/// tag of its first, for a box whose prompt shares the cell with its
	/// answer. Answers the cell written.
	pub fn append_text(
		&mut self,
		root: Entity,
		cell: &CellAddress,
		text: &str,
	) -> Result<Entity> {
		let entity = self.cell(root, cell)?;
		let lines = text.split('\n').collect::<Vec<_>>();
		match self.text.blocks(entity).last().copied() {
			Some(last) => self.insert_after(last, &lines),
			// a cell of words alone takes each line after a break
			None => {
				for line in lines {
					self.commands.spawn((Element::new("br"), ChildOf(entity)));
					self.commands.spawn((Value::str(line), ChildOf(entity)));
				}
			}
		}
		entity.xok()
	}

	/// Checks every checkbox whose paragraph carries `label`, its state its
	/// value, answering how many.
	pub fn check_box(&mut self, root: Entity, label: &str) -> Result<usize> {
		let needle = Self::needle(label)?;
		let boxes = self
			.descendants(root)
			.into_iter()
			.filter(|entity| self.text.checkbox(*entity).is_some())
			.filter(|entity| {
				let paragraph = self.paragraph(*entity).unwrap_or(root);
				Self::plain(&self.text.text(paragraph)).contains(&needle)
			})
			.collect::<Vec<_>>();
		for entity in &boxes {
			self.commands.entity(*entity).insert(Value::Bool(true));
		}
		boxes.len().xok()
	}

	/// Removes every paragraph carrying `text`, ie a red instruction
	/// sentence, answering how many. A cell's last paragraph is emptied
	/// rather than removed, since a cell holds one.
	pub fn remove_paragraphs(
		&mut self,
		root: Entity,
		text: &str,
	) -> Result<usize> {
		let needle = Self::needle(text)?;
		let matched = self
			.paragraphs(root)
			.into_iter()
			.filter(|block| self.leaf(*block))
			.filter(|block| {
				Self::plain(&self.text.text(*block)).contains(&needle)
			})
			.collect::<Vec<_>>();
		// a cell whose every paragraph goes keeps its last, emptied
		let emptied = matched
			.iter()
			.filter_map(|block| {
				let cell = self.cell_of(*block)?;
				let blocks = self.text.blocks(cell);
				(blocks.iter().all(|other| matched.contains(other))
					&& blocks.last() == Some(block))
				.then_some(*block)
			})
			.collect::<Vec<_>>();
		for block in &matched {
			match emptied.contains(block) {
				true => self.empty(*block),
				false => self.commands.entity(*block).try_despawn(),
			}
		}
		matched.len().xok()
	}

	/// Replaces `old` with `new` inside the words that carry it, touching
	/// only the pieces of text the match covers, so a highlighted placeholder
	/// stays highlighted. Answers how many it replaced.
	pub fn replace_text(
		&mut self,
		root: Entity,
		old: &str,
		new: &str,
	) -> Result<usize> {
		let needle = Self::needle(old)?;
		let paragraphs = self
			.paragraphs(root)
			.into_iter()
			.filter(|block| self.leaf(*block))
			.map(|block| self.text_nodes(block))
			.collect::<Vec<_>>();
		let mut replaced = 0;
		for nodes in paragraphs {
			let mut texts = nodes
				.iter()
				.map(|node| {
					self.nodes
						.get(*node)
						.ok()
						.and_then(|(_, value, _)| value)
						.map(|value| Self::plain(&value.to_string()))
						.unwrap_or_default()
				})
				.collect::<Vec<_>>();
			let mut touched = vec![false; texts.len()];
			for _ in 0..Self::MOST {
				let Some(at) = texts.concat().find(&needle) else {
					break;
				};
				let end = at + needle.len();
				let mut position = 0;
				let mut placed = false;
				for (text, touched) in texts.iter_mut().zip(touched.iter_mut())
				{
					let (start, stop) = (position, position + text.len());
					position = stop;
					if stop <= at || start >= end {
						continue;
					}
					let before =
						&text[..at.saturating_sub(start).min(text.len())];
					let after = &text[(end - start).min(text.len())..];
					*text = match placed {
						true => format!("{before}{after}"),
						false => format!("{before}{new}{after}"),
					};
					placed = true;
					*touched = true;
				}
				replaced += 1;
			}
			for ((node, text), _) in nodes
				.iter()
				.zip(texts)
				.zip(touched)
				.filter(|(_, touched)| *touched)
			{
				self.commands.entity(*node).insert(Value::Str(text.into()));
			}
		}
		replaced.xok()
	}

	/// The cell at `address` under `root`, refused when absent or locked.
	fn cell(&self, root: Entity, address: &CellAddress) -> Result<Entity> {
		let absent = || bevyhow!("the document has no cell {address}");
		match address {
			CellAddress::Table(table) => {
				self.table_cells.table_cell(root, *table).ok_or_else(absent)
			}
			#[cfg(feature = "ooxml")]
			CellAddress::Sheet(sheet) => {
				let cell = self
					.sheet_cells
					.sheet_cell(root, sheet)
					.ok_or_else(absent)?;
				if self.sheet_cells.is_locked(cell) {
					bevybail!(
						"{address} is locked; only an unlocked cell takes a value"
					);
				}
				cell.xok()
			}
		}
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
				Some(tag) if RenderTreeQuery::is_textless(tag) => {}
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
				Some(tag) if RenderTreeQuery::is_textless(tag) => {}
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
				Some(tag)
					if RenderTreeQuery::is_textless(tag) || tag == "table" => {}
				_ if self.is_text(child) => out.push(child),
				_ => out.extend(self.own_text_nodes(child)),
			}
		}
		out
	}

	/// Sets the words of `block`: its first text node keeps its place and
	/// its look and takes `text`, every other piece of content around it
	/// goes, and a block holding no text takes a new text node.
	fn set_words(&mut self, block: Entity, text: &str) {
		let Some(first) = self.own_text_nodes(block).into_iter().next() else {
			// nothing to keep: every piece of content goes
			self.empty(block);
			self.commands.spawn((Value::str(text), ChildOf(block)));
			return;
		};
		let chain = core::iter::once(first)
			.chain(self.parents.iter_ancestors(first))
			.take_while(|entity| *entity != block)
			.collect::<Vec<_>>();
		// at every level the chain passes, the content beside it goes
		let mut removed = Vec::new();
		for link in &chain {
			let parent = self.parents.get(*link).map(ChildOf::parent).ok();
			for sibling in parent
				.map(|parent| self.children(parent))
				.unwrap_or_default()
			{
				if sibling != *link && self.is_content(sibling) {
					removed.push(sibling);
				}
			}
		}
		for entity in removed {
			self.commands.entity(entity).try_despawn();
		}
		self.commands.entity(first).insert(Value::str(text));
	}

	/// Empties `block` of every piece of content it holds.
	fn empty(&mut self, block: Entity) {
		let content = self
			.children(block)
			.into_iter()
			.filter(|child| self.is_content(*child))
			.collect::<Vec<_>>();
		for entity in content {
			self.commands.entity(entity).try_despawn();
		}
	}

	/// Inserts a paragraph per line after `model`, in its tag, each holding
	/// its line as a text node.
	fn insert_after(&mut self, model: Entity, lines: &[&str]) {
		let tag = self.tag(model).unwrap_or("p").to_string();
		let Ok(parent) = self.parents.get(model).map(ChildOf::parent) else {
			return;
		};
		let mut index = self
			.children(parent)
			.iter()
			.position(|child| *child == model)
			.unwrap_or_default();
		for line in lines {
			index += 1;
			let paragraph = self
				.commands
				.spawn((Element::new(tag.as_str()), children![Value::str(
					*line
				)]))
				.id();
			self.commands.entity(parent).insert_child(index, paragraph);
		}
	}
}

#[cfg(test)]
#[cfg(feature = "markdown_parser")]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// The edits a Word form takes, applied to a markdown one: its table's
	/// cells, its task list's box, its instruction and its placeholder.
	#[beet_core::test]
	fn fills_a_markdown_form() {
		let mut world = ParsePlugin.into_world();
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
		world
			.with_state::<DocumentTreeEdit, _>(|mut edit| {
				edit.set_text(root, &cell("t1r1c2"), "Acme Stalls")
			})
			.unwrap();
		world
			.with_state::<DocumentTreeEdit, _>(|mut edit| {
				edit.append_text(root, &cell("t1r2c1"), "rents fitted stalls")
			})
			.unwrap();
		world
			.with_state::<DocumentTreeEdit, _>(|mut edit| {
				edit.check_box(root, "Surveys")
			})
			.unwrap()
			.xpect_eq(1);
		world
			.with_state::<DocumentTreeEdit, _>(|mut edit| {
				edit.remove_paragraphs(root, "Please delete this sentence")
			})
			.unwrap()
			.xpect_eq(1);
		world
			.with_state::<DocumentTreeEdit, _>(|mut edit| {
				edit.replace_text(root, "(Insert name)", "Acme")
			})
			.unwrap()
			.xpect_eq(1);
		MarkdownRenderer::new()
			.render(&mut RenderContext::new(
				&mut world,
				root,
				&RequestParts::default(),
			))
			.unwrap()
			.to_string()
			.xpect_eq(
				"| Name | Acme Stalls |\n|---|---|\n| Describe it:<br>rents fitted stalls |  |\n\n\
				 - [x] Surveys\n- [ ] Focus groups\n\n\
				 The business is <mark>Acme</mark>.\n",
			);
	}
}
