//! The words a reader reads, read straight off a tree.
use crate::prelude::*;
use beet_core::prelude::*;

/// The text a reader reads under an entity, the words every renderer shows:
/// text values and a node's own text, never a non-visual element's content
/// nor a [`SourceText`], with a checkbox as `[x] ` or `[ ] `, and a line
/// break, a paragraph's end and a cell's as a newline.
#[derive(SystemParam)]
pub struct ReaderText<'w, 's> {
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
	attributes: AttributeQuery<'w, 's>,
}

impl ReaderText<'_, '_> {
	/// The block elements a reader reads as paragraphs.
	pub const BLOCKS: &[&str] =
		&["p", "h1", "h2", "h3", "h4", "h5", "h6", "li", "pre"];

	/// Everything a reader reads under `entity`, itself included.
	pub fn text(&self, entity: Entity) -> String {
		let mut out = String::new();
		self.push_text(entity, false, true, &mut out);
		out
	}

	/// What a reader reads under `entity` outside any table nested in it, ie
	/// a table cell's own words.
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
	/// checked, by its `checked` attribute or its own value.
	pub fn checkbox(&self, entity: Entity) -> Option<bool> {
		let (Some(element), value, ..) = self.nodes.get(entity).ok()? else {
			return None;
		};
		let is_checkbox = element.tag() == "input"
			&& self
				.attributes
				.find(entity, "type")
				.is_some_and(|(_, value)| value.to_string() == "checkbox");
		is_checkbox.then(|| {
			matches!(value, Some(Value::Bool(true)))
				|| self.attributes.find(entity, "checked").is_some()
		})
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
			Some(tag) if Self::BLOCKS.contains(&tag) => out.push(entity),
			Some(tag) if tag == "table" || is_non_visual(tag) => {}
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
			Some(tag) if is_non_visual(tag) => return,
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
		let ends_line = element.is_some_and(|element| {
			Self::BLOCKS.contains(&element.tag())
				|| matches!(element.tag(), "td" | "th" | "tr" | "table")
		});
		if ends_line && !is_start && !out.is_empty() && !out.ends_with('\n') {
			out.push('\n');
		}
	}
}
