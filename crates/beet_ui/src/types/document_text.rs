//! [`DocumentText`], a document's words, paragraphs and checkboxes, read off
//! its tree the same way whichever format it came from.
use crate::prelude::*;
use beet_core::prelude::*;

/// A document's content read off its tree, the same way for every format
/// (markdown, HTML, Word, a workbook): its words as plain text
/// ([`text`](Self::text)), its paragraphs ([`blocks`](Self::blocks)) and
/// its checkboxes ([`checkbox`](Self::checkbox)). A look is the cascade's,
/// never read here.
///
/// Cell listings and prose checks read a document through this; it is not a
/// renderer, which writes the tree out in a format of its own.
#[derive(SystemParam)]
pub struct DocumentText<'w, 's> {
	nodes: Query<
		'w,
		's,
		(
			Option<&'static Element>,
			Option<&'static Value>,
			Option<&'static CData>,
			Option<&'static Children>,
		),
	>,
	elements: ElementQuery<'w, 's>,
}

impl DocumentText<'_, '_> {
	/// The elements that are paragraphs.
	pub const BLOCKS: &[&str] =
		&["p", "h1", "h2", "h3", "h4", "h5", "h6", "li", "pre"];

	/// Whether `tag` is one of the [`BLOCKS`](Self::BLOCKS).
	pub fn is_block(tag: &str) -> bool { Self::BLOCKS.contains(&tag) }

	/// Whether `tag` sets its words bold, `strong` or `b`.
	pub fn is_bold(tag: &str) -> bool { matches!(tag, "strong" | "b") }

	/// Whether `tag` sets its words italic, `em` or `i`.
	pub fn is_italic(tag: &str) -> bool { matches!(tag, "em" | "i") }

	/// A heading's level, 1 for `h1` to 6 for `h6`, `None` for any other tag.
	pub fn heading_level(tag: &str) -> Option<u8> {
		tag.strip_prefix('h')
			.and_then(|level| level.parse::<u8>().ok())
			.filter(|level| (1..=6).contains(level))
	}

	/// The plain text under `entity`, itself included: every text node,
	/// nothing inside a textless element
	/// ([`RenderTreeQuery::is_textless`]), a checkbox as `[x] ` or `[ ] `, and
	/// a line break and the end of a paragraph, row or cell as a newline.
	pub fn text(&self, entity: Entity) -> String {
		let mut out = String::new();
		self.push_text(entity, false, true, &mut out);
		out
	}

	/// The [`text`](Self::text) under `entity` outside any table nested in
	/// it, ie a table cell's own words.
	pub fn own_text(&self, entity: Entity) -> String {
		let mut out = String::new();
		self.push_text(entity, true, true, &mut out);
		out
	}

	/// The paragraphs under `entity` in order: every [`BLOCKS`](Self::BLOCKS)
	/// element, not looking inside one, nor inside a nested table.
	pub fn blocks(&self, entity: Entity) -> Vec<Entity> {
		let mut out = Vec::new();
		for child in self.children(entity) {
			self.push_blocks(child, &mut out);
		}
		out
	}

	/// Whether `entity` is a checkbox `<input>`, and if so whether it is
	/// checked, as [`ElementView::checkbox`] reads it.
	pub fn checkbox(&self, entity: Entity) -> Option<bool> {
		self.elements.get(entity).ok()?.checkbox()
	}

	fn children(&self, entity: Entity) -> Vec<Entity> {
		self.nodes
			.get(entity)
			.ok()
			.and_then(|(.., children)| children)
			.map(|children| children.to_vec())
			.unwrap_or_default()
	}

	fn push_blocks(&self, entity: Entity, out: &mut Vec<Entity>) {
		let Ok((element, ..)) = self.nodes.get(entity) else {
			return;
		};
		match element.map(Element::tag) {
			Some(tag) if Self::is_block(tag) => out.push(entity),
			Some(tag)
				if tag == "table" || RenderTreeQuery::is_textless(tag) => {}
			_ => {
				for child in self.children(entity) {
					self.push_blocks(child, out);
				}
			}
		}
	}

	fn push_text(
		&self,
		entity: Entity,
		skip_tables: bool,
		is_start: bool,
		out: &mut String,
	) {
		let Ok((element, value, cdata, _)) = self.nodes.get(entity) else {
			return;
		};
		if let Some(checked) = self.checkbox(entity) {
			out.push_str(if checked { "[x] " } else { "[ ] " });
			return;
		}
		match element.map(Element::tag) {
			Some(tag) if RenderTreeQuery::is_textless(tag) => return,
			Some("br") => out.push('\n'),
			Some("table") if skip_tables && !is_start => return,
			// an element's own value is binding state, never its words
			Some(_) => {}
			None => {
				if let Some(Value::Str(text)) = value {
					// a label bringing its own space follows its box once
					let after_box =
						out.ends_with("[x] ") || out.ends_with("[ ] ");
					match after_box {
						true => {
							out.push_str(text.strip_prefix(' ').unwrap_or(text))
						}
						false => out.push_str(text),
					}
				}
			}
		}
		if let Some(cdata) = cdata {
			out.push_str(cdata);
		}
		for child in self.children(entity) {
			self.push_text(child, skip_tables, false, out);
		}
		// a block ends its line
		let ends_line = element.map(Element::tag).is_some_and(|tag| {
			Self::is_block(tag) || TableGrid::is_table_tag(tag)
		});
		if ends_line && !is_start && !out.is_empty() && !out.ends_with('\n') {
			out.push('\n');
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A box reads as its value says when it holds a bool, an unticked value
	/// overriding an authored `checked`, and as its attribute otherwise.
	#[beet_core::test]
	fn reads_a_checkbox_by_its_state() {
		let mut world = World::new();
		let root = world
			.spawn(rsx! {
				<p>
					<input type="checkbox" checked/>
					<input type="checkbox" checked {Value::Bool(false)}/>
					<input type="checkbox" {Value::Bool(true)}/>
					<input type="checkbox"/>
				</p>
			})
			.id();
		world.with_state::<(DocumentText, Query<&Children>), _>(
			|(text, children)| {
				children
					.iter_descendants(root)
					.filter_map(|entity| text.checkbox(entity))
					.collect::<Vec<_>>()
					.xpect_eq(vec![true, false, true, false]);
			},
		);
	}
}
