//! The words a reader reads, read straight off a tree.
use crate::prelude::*;
use beet_core::prelude::*;

/// The text a reader reads under an entity, the words every renderer shows:
/// text values and a node's own text, never a textless element's content
/// ([`RenderTreeQuery::is_textless`]), with a checkbox as `[x] ` or `[ ] `,
/// and a line break, a paragraph's end and a cell's as a newline. Also the
/// look of its words, [`ReaderText::look`].
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
	elements: ElementQuery<'w, 's>,
	parents: Query<'w, 's, &'static ChildOf>,
}

impl ReaderText<'_, '_> {
	/// The block elements a reader reads as paragraphs.
	pub const BLOCKS: &[&str] =
		&["p", "h1", "h2", "h3", "h4", "h5", "h6", "li", "pre"];

	/// Whether `tag` is one of the [`BLOCKS`](Self::BLOCKS).
	pub fn is_block(tag: &str) -> bool { Self::BLOCKS.contains(&tag) }

	/// A heading's level, 1 for `h1` to 6 for `h6`, `None` for any other tag.
	pub fn heading_level(tag: &str) -> Option<u8> {
		tag.strip_prefix('h')
			.and_then(|level| level.parse::<u8>().ok())
			.filter(|level| (1..=6).contains(level))
	}

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
	/// checked, as [`ElementView::checkbox`] reads it.
	pub fn checkbox(&self, entity: Entity) -> Option<bool> {
		self.elements.get(entity).ok()?.checkbox()
	}

	/// The look of the words at `entity`, read from it and its ancestors up
	/// to the paragraph around it, the nearest [`BLOCKS`](Self::BLOCKS)
	/// element: the same facts whichever format the tree was read from.
	pub fn look(&self, entity: Entity) -> TextLook {
		let mut look = TextLook::default();
		for view in core::iter::once(entity)
			.chain(self.parents.iter_ancestors(entity))
			.filter_map(|entity| self.elements.get(entity).ok())
		{
			look.apply(&view);
			if Self::is_block(view.tag()) {
				break;
			}
		}
		look
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
		let ends_line = element
			.map(Element::tag)
			.is_some_and(|tag| Self::is_block(tag) || Cells::is_table_tag(tag));
		if ends_line && !is_start && !out.is_empty() && !out.ends_with('\n') {
			out.push('\n');
		}
	}
}

/// The look of some words, as [`ReaderText::look`] reads it from the
/// elements around them.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TextLook {
	/// Inside a `<strong>` or `<b>`.
	pub bold: bool,
	/// Inside an `<em>` or `<i>`.
	pub italic: bool,
	/// The innermost `<mark>`'s background, `yellow` when its style sets
	/// none.
	pub highlight: Option<SmolStr>,
	/// The innermost `color` a `style` declares.
	pub colour: Option<SmolStr>,
}

impl TextLook {
	/// Whether `tag` sets its words bold, `strong` or `b`.
	pub fn is_bold(tag: &str) -> bool { matches!(tag, "strong" | "b") }

	/// Whether `tag` sets its words italic, `em` or `i`.
	pub fn is_italic(tag: &str) -> bool { matches!(tag, "em" | "i") }

	/// Adds what `view` says, an inner element's highlight and colour
	/// already read winning over an outer one's.
	fn apply(&mut self, view: &ElementView) {
		let tag = view.tag();
		self.bold |= Self::is_bold(tag);
		self.italic |= Self::is_italic(tag);
		if tag == "mark" && self.highlight.is_none() {
			self.highlight = view
				.style_property("background-color")
				.or_else(|| view.style_property("background"))
				.unwrap_or_else(|| "yellow".into())
				.xsome();
		}
		if self.colour.is_none() {
			self.colour = view.style_property("color");
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
		world.with_state::<(ReaderText, Query<&Children>), _>(
			|(text, children)| {
				children
					.iter_descendants(root)
					.filter_map(|entity| text.checkbox(entity))
					.collect::<Vec<_>>()
					.xpect_eq(vec![true, false, true, false]);
			},
		);
	}

	/// The look of each text under a parsed document, by its words.
	#[cfg(feature = "markdown_parser")]
	fn looks(bytes: MediaBytes) -> Vec<(String, TextLook)> {
		let mut world = ParsePlugin.into_world();
		let root = world.spawn_empty().id();
		DocParsers::parse(ParseContext::new(
			&mut world.entity_mut(root),
			&bytes,
		))
		.unwrap();
		world.with_state::<(ReaderText, Query<&Children>, Query<&Value>), _>(
			|(text, children, values)| {
				children
					.iter_descendants(root)
					.filter_map(|entity| {
						let words = values.get(entity).ok()?.as_str().ok()?;
						(words.to_string(), text.look(entity)).xsome()
					})
					.collect()
			},
		)
	}

	/// The look of `words` in `looks`.
	#[cfg(feature = "markdown_parser")]
	fn look_of(looks: &[(String, TextLook)], words: &str) -> TextLook {
		looks
			.iter()
			.find(|(text, _)| text == words)
			.map(|(_, look)| look.clone())
			.unwrap()
	}

	/// A look is the same fact whichever format said it: markdown's
	/// emphasis, an HTML `<mark>` and a styled colour, the innermost
	/// winning and nothing read from beyond the paragraph.
	#[cfg(feature = "markdown_parser")]
	#[beet_core::test]
	fn reads_a_look_from_any_format() {
		let markdown = looks(MediaBytes::new_markdown("**bold** and *it*"));
		look_of(&markdown, "bold").xpect_eq(TextLook {
			bold: true,
			..default()
		});
		look_of(&markdown, "it").xpect_eq(TextLook {
			italic: true,
			..default()
		});
		look_of(&markdown, " and ").xpect_eq(TextLook::default());

		let html = looks(MediaBytes::new_html(
			"<div style=\"color: red\"><p><mark>hi</mark>\
			 <span style=\"color: #00F\"><mark style=\"background: green\"><b>there</b></mark></span></p></div>",
		));
		look_of(&html, "hi").xpect_eq(TextLook {
			highlight: Some("yellow".into()),
			..default()
		});
		look_of(&html, "there").xpect_eq(TextLook {
			bold: true,
			highlight: Some("green".into()),
			colour: Some("#00F".into()),
			..default()
		});
	}
}
