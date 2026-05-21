//! Tree-sitter based syntax highlighting for code blocks.
//!
//! [`SyntaxHighlighting`] holds a tree-sitter [`Parser`] plus a per-language
//! [`Query`] and a colour palette mapping highlight capture names to
//! [`Color`]s. Given a language identifier and a source slice, [`Self::highlight`]
//! returns a sequence of [`HighlightSpan`]s covering every byte of the input
//! exactly once.
//!
//! Capture-name dispatch follows the same dot-separated longest-match rule as
//! tree-sitter-highlight: `string.escape` wins over `string` when both are
//! present in the palette.
//!
//! Only enabled with the `syntax_highlighting` feature.

use crate::prelude::*;
use crate::style::syntax::tokens as tokens_mod;
use alloc::collections::BTreeMap;
use beet_core::prelude::*;
use streaming_iterator::StreamingIterator;
use tree_sitter::Language;
use tree_sitter::Parser;
use tree_sitter::Query;
use tree_sitter::QueryCursor;

/// Top-level config holding parser, per-language queries, and the
/// capture-name → colour palette.
pub struct SyntaxHighlighting {
	parser: Parser,
	languages: HashMap<String, LanguageEntry>,
	palette: HashMap<String, Color>,
}

impl core::fmt::Debug for SyntaxHighlighting {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("SyntaxHighlighting")
			.field("languages", &self.languages.keys().collect::<Vec<_>>())
			.field("palette_len", &self.palette.len())
			.finish()
	}
}

impl Clone for SyntaxHighlighting {
	fn clone(&self) -> Self {
		let mut out = Self::empty();
		out.palette = self.palette.clone();
		for (name, entry) in &self.languages {
			out.languages.insert(name.clone(), LanguageEntry {
				language: entry.language.clone(),
				query_source: entry.query_source.clone(),
				aliases: entry.aliases.clone(),
			});
		}
		out
	}
}

impl Default for SyntaxHighlighting {
	fn default() -> Self { Self::with_defaults() }
}

struct LanguageEntry {
	language: Language,
	query_source: String,
	aliases: Vec<String>,
}

/// A contiguous slice of highlighted source text.
#[derive(Debug, Clone, PartialEq)]
pub struct HighlightSpan {
	/// Owned slice of the original source for this span.
	pub text: String,
	/// Recognised capture name like `keyword`, or `None` for unstyled
	/// regions between captures.
	pub capture: Option<String>,
	/// Resolved colour from the palette, when the capture matched a value.
	pub color: Option<Color>,
}

impl SyntaxHighlighting {
	/// A highlighter with no languages or palette entries.
	pub fn empty() -> Self {
		Self {
			parser: Parser::new(),
			languages: HashMap::default(),
			palette: HashMap::default(),
		}
	}

	/// A highlighter pre-registered with `rust`, `javascript`, `json`,
	/// `html` (plus common aliases), and the default colour palette.
	pub fn with_defaults() -> Self {
		let mut this = Self::empty();
		this.set_default_palette();
		this.add_rust();
		this.add_javascript();
		this.add_json();
		this.add_html();
		this
	}

	/// Register the rust grammar.
	pub fn add_rust(&mut self) -> &mut Self {
		self.add_language(
			"rust",
			tree_sitter_rust::LANGUAGE.into(),
			tree_sitter_rust::HIGHLIGHTS_QUERY,
			&["rs"],
		);
		self
	}

	/// Register the javascript grammar.
	pub fn add_javascript(&mut self) -> &mut Self {
		self.add_language(
			"javascript",
			tree_sitter_javascript::LANGUAGE.into(),
			tree_sitter_javascript::HIGHLIGHT_QUERY,
			&["js", "jsx", "mjs"],
		);
		self
	}

	/// Register the json grammar.
	pub fn add_json(&mut self) -> &mut Self {
		self.add_language(
			"json",
			tree_sitter_json::LANGUAGE.into(),
			tree_sitter_json::HIGHLIGHTS_QUERY,
			&[],
		);
		self
	}

	/// Register the html grammar.
	pub fn add_html(&mut self) -> &mut Self {
		self.add_language(
			"html",
			tree_sitter_html::LANGUAGE.into(),
			tree_sitter_html::HIGHLIGHTS_QUERY,
			&["htm"],
		);
		self
	}

	/// Register a tree-sitter [`Language`] under `name` with a highlight query.
	/// Optional `aliases` map alternate identifiers (eg `rs`) to the same entry.
	///
	/// Panics if the query fails to compile against the language.
	pub fn add_language(
		&mut self,
		name: impl Into<String>,
		language: Language,
		query: impl Into<String>,
		aliases: &[&str],
	) -> &mut Self {
		let name = name.into();
		let query_source = query.into();
		// validate up-front so a bad grammar fails registration, not highlight
		Query::new(&language, &query_source).expect("query compiles");
		let aliases = aliases.iter().map(|s| s.to_string()).collect();
		self.languages.insert(name.clone(), LanguageEntry {
			language,
			query_source,
			aliases,
		});
		self
	}

	/// Reset the palette to the built-in default scheme.
	pub fn set_default_palette(&mut self) -> &mut Self {
		self.palette.clear();
		self.palette
			.extend(default_palette().into_iter().map(|(k, v)| (k.into(), v)));
		self
	}

	/// Override the colour assigned to a single capture name.
	pub fn set_color(
		&mut self,
		capture: impl Into<String>,
		color: impl Into<Color>,
	) -> &mut Self {
		self.palette.insert(capture.into(), color.into());
		self
	}

	/// Look up the colour for a capture name with dot-separated longest-match
	/// fallback (`string.escape` first, then `string`).
	pub fn color_for(&self, capture: &str) -> Option<Color> {
		let mut key = capture;
		loop {
			if let Some(c) = self.palette.get(key) {
				return Some(*c);
			}
			match key.rfind('.') {
				Some(pos) => key = &key[..pos],
				None => return None,
			}
		}
	}

	/// Resolve a code-block info string (eg `rust` or `rs`) to a registered
	/// language entry.
	fn resolve_language(&self, name: &str) -> Option<&LanguageEntry> {
		let lower = name.to_ascii_lowercase();
		if let Some(entry) = self.languages.get(&lower) {
			return Some(entry);
		}
		self.languages
			.values()
			.find(|entry| entry.aliases.iter().any(|a| a == &lower))
	}

	/// Tokenize `source` for `lang`, returning a complete cover of the input
	/// where each byte appears in exactly one [`HighlightSpan`].
	///
	/// Returns a single unstyled span if the language is unknown or parsing
	/// produced no tree, so callers can always render the result.
	pub fn highlight(
		&mut self,
		lang: &str,
		source: &str,
	) -> Vec<HighlightSpan> {
		let Some(entry) = self.resolve_language(lang) else {
			return vec![HighlightSpan {
				text: source.to_string(),
				capture: None,
				color: None,
			}];
		};
		let language = entry.language.clone();
		let query_source = entry.query_source.clone();

		if self.parser.set_language(&language).is_err() {
			return vec![HighlightSpan {
				text: source.to_string(),
				capture: None,
				color: None,
			}];
		}
		let Some(tree) = self.parser.parse(source.as_bytes(), None) else {
			return vec![HighlightSpan {
				text: source.to_string(),
				capture: None,
				color: None,
			}];
		};

		// rebuild query so we can borrow capture_names() with a longer lifetime
		// (Query stored on entry would force re-borrowing self).
		let query =
			Query::new(&language, &query_source).expect("query compiles");
		let capture_names: Vec<&str> = query.capture_names().to_vec();

		// 1. collect captures sorted by start byte (BTreeMap keeps order)
		let mut events: BTreeMap<usize, (usize, u32)> = BTreeMap::new();
		let mut cursor = QueryCursor::new();
		let mut matches =
			cursor.captures(&query, tree.root_node(), source.as_bytes());
		while let Some((m, _)) = matches.next() {
			for cap in m.captures {
				let node = cap.node;
				let start = node.start_byte();
				let end = node.end_byte();
				if end <= start {
					continue;
				}
				// keep the longest non-overlapping run for any starting byte
				let entry = events.entry(start).or_insert((end, cap.index));
				if end > entry.0 {
					*entry = (end, cap.index);
				}
			}
		}

		// 2. flatten to non-overlapping spans by skipping ranges contained
		// inside the previously emitted span.
		let mut spans = Vec::new();
		let mut cursor_byte = 0usize;
		for (start, (end, capture_index)) in events {
			if start < cursor_byte {
				continue;
			}
			if start > cursor_byte {
				spans.push(HighlightSpan {
					text: source[cursor_byte..start].to_string(),
					capture: None,
					color: None,
				});
			}
			let name = capture_names
				.get(capture_index as usize)
				.copied()
				.unwrap_or("");
			let color = self.color_for(name);
			spans.push(HighlightSpan {
				text: source[start..end].to_string(),
				capture: Some(name.to_string()),
				color,
			});
			cursor_byte = end;
		}
		if cursor_byte < source.len() {
			spans.push(HighlightSpan {
				text: source[cursor_byte..].to_string(),
				capture: None,
				color: None,
			});
		}
		spans
	}
}

/// The built-in colour palette, keyed by tree-sitter capture name.
///
/// Designed to be readable on both light and dark terminals. Override
/// individual entries with [`SyntaxHighlighting::set_color`].
pub fn default_palette() -> Vec<(&'static str, Color)> {
	vec![
		("attribute", Color::srgb(0.85, 0.65, 0.30)),
		("boolean", Color::srgb(0.95, 0.55, 0.20)),
		("comment", Color::srgba(0.55, 0.60, 0.55, 0.85)),
		(
			"comment.documentation",
			Color::srgba(0.55, 0.70, 0.55, 0.95),
		),
		("constant", Color::srgb(0.95, 0.55, 0.20)),
		("constant.builtin", Color::srgb(0.95, 0.55, 0.20)),
		("constructor", Color::srgb(0.40, 0.75, 0.95)),
		("embedded", Color::srgb(0.75, 0.55, 0.95)),
		("error", Color::srgb(0.95, 0.30, 0.30)),
		("escape", Color::srgb(0.95, 0.55, 0.20)),
		("function", Color::srgb(0.40, 0.75, 0.95)),
		("function.builtin", Color::srgb(0.40, 0.85, 0.85)),
		("keyword", Color::srgb(0.95, 0.45, 0.75)),
		("number", Color::srgb(0.95, 0.55, 0.20)),
		("operator", Color::srgb(0.90, 0.50, 0.70)),
		("property", Color::srgb(0.40, 0.75, 0.95)),
		("punctuation", Color::srgba(0.85, 0.85, 0.85, 0.85)),
		("punctuation.bracket", Color::srgba(0.85, 0.85, 0.85, 0.85)),
		(
			"punctuation.delimiter",
			Color::srgba(0.85, 0.85, 0.85, 0.85),
		),
		("punctuation.special", Color::srgb(0.95, 0.45, 0.75)),
		("string", Color::srgb(0.60, 0.85, 0.55)),
		("string.escape", Color::srgb(0.95, 0.85, 0.45)),
		("string.regexp", Color::srgb(0.95, 0.55, 0.20)),
		("string.special", Color::srgb(0.95, 0.55, 0.20)),
		("tag", Color::srgb(0.40, 0.75, 0.95)),
		("type", Color::srgb(0.40, 0.85, 0.85)),
		("type.builtin", Color::srgb(0.40, 0.85, 0.85)),
		("variable", Color::srgb(0.85, 0.85, 0.85)),
		("variable.builtin", Color::srgb(0.95, 0.55, 0.20)),
		("variable.member", Color::srgb(0.40, 0.75, 0.95)),
		("variable.parameter", Color::srgb(0.85, 0.65, 0.30)),
	]
}

/// The list of capture names this module recognises, matching
/// [`tokens_mod::recognised_names`].
pub fn recognised_names() -> &'static [&'static str] {
	tokens_mod::recognised_names()
}

/// Walk descendants of `root`, find each `<pre><code class="language-X">`
/// whose only child is a text node, and replace the text node with one
/// span element per highlight run.
///
/// Each emitted span has:
/// - tag `span`
/// - attribute `class="hl-<capture>"` for captured runs (omitted for plain runs)
/// - a single text child carrying the original slice of source
pub fn apply_syntax_highlighting(
	world: &mut World,
	root: Entity,
	highlighter: &mut SyntaxHighlighting,
) {
	let blocks = find_code_blocks(world, root);
	for (code_entity, lang, text_entity, source) in blocks {
		let spans = highlighter.highlight(&lang, &source);
		// despawn the original single text child
		world.entity_mut(text_entity).despawn();
		for span in spans {
			spawn_highlight_span(world, code_entity, span);
		}
	}
}

/// Walk descendants of `root` and collect every fenced code block that is
/// eligible for syntax highlighting.
///
/// Returns tuples of (code_entity, language, text_entity, source_text).
fn find_code_blocks(
	world: &mut World,
	root: Entity,
) -> Vec<(Entity, String, Entity, String)> {
	let mut out = Vec::new();
	let mut stack = vec![root];
	while let Some(current) = stack.pop() {
		let Ok(entity_ref) = world.get_entity(current) else {
			continue;
		};
		let is_code = entity_ref
			.get::<Element>()
			.map(|el| el.tag() == "code")
			.unwrap_or(false);
		let children: Vec<Entity> = entity_ref
			.get::<Children>()
			.map(|c| c.iter().collect())
			.unwrap_or_default();
		if is_code {
			if let Some((lang, text_entity, text)) =
				resolve_code_block(world, current, &children)
			{
				out.push((current, lang, text_entity, text));
				continue; // do not recurse into a block we will replace
			}
		}
		for child in children {
			stack.push(child);
		}
	}
	out
}

/// Resolve a `<code class="language-X">` entity to (language, text_entity, text)
/// if its only child is a text node.
fn resolve_code_block(
	world: &World,
	code: Entity,
	children: &[Entity],
) -> Option<(String, Entity, String)> {
	let entity_ref = world.get_entity(code).ok()?;
	let attributes = entity_ref.get::<Attributes>()?;
	let mut lang = None;
	for attr_entity in attributes.iter() {
		let attr_ref = world.get_entity(attr_entity).ok()?;
		if attr_ref.get::<Attribute>()?.as_str() == "class" {
			let value = attr_ref.get::<Value>()?;
			let class = value.as_str().ok()?;
			for c in class.split_whitespace() {
				// accept both `language-X` (HTML convention) and bare `X`
				let name = c.strip_prefix("language-").unwrap_or(c);
				if !name.is_empty() {
					lang = Some(name.to_string());
					break;
				}
			}
		}
	}
	let lang = lang?;
	if children.len() != 1 {
		return None;
	}
	let text_entity = children[0];
	let text = world
		.get_entity(text_entity)
		.ok()?
		.get::<Value>()?
		.as_str()
		.ok()?
		.to_string();
	Some((lang, text_entity, text))
}

/// Spawn a single highlight span as a child of `parent`. When `span.capture`
/// is set, an `Attribute("class") = "hl-<capture>"` entity is also spawned.
fn spawn_highlight_span(
	world: &mut World,
	parent: Entity,
	span: HighlightSpan,
) {
	let element = world.spawn((Element::new("span"), ChildOf(parent))).id();
	if let Some(capture) = &span.capture {
		let class = format!("hl-{}", capture);
		world.spawn((
			Attribute::new("class"),
			Value::str(class),
			AttributeOf::new(element),
		));
	}
	world.spawn((Value::str(span.text), ChildOf(element)));
}


#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	fn highlights_simple_rust() {
		let mut hl = SyntaxHighlighting::with_defaults();
		let spans = hl.highlight("rust", "fn main() {}");
		// must cover entire source
		spans
			.iter()
			.map(|s| s.text.as_str())
			.collect::<String>()
			.xpect_eq("fn main() {}");
		// fn keyword captured
		spans
			.iter()
			.any(|s| s.capture.as_deref() == Some("keyword"))
			.xpect_true();
	}

	#[beet_core::test]
	fn highlights_unknown_lang_passthrough() {
		let mut hl = SyntaxHighlighting::with_defaults();
		let spans = hl.highlight("nonesuch", "abc");
		spans.len().xpect_eq(1);
		spans[0].text.as_str().xpect_eq("abc");
		spans[0].capture.is_none().xpect_true();
	}

	#[beet_core::test]
	fn longest_match_wins() {
		let mut hl = SyntaxHighlighting::empty();
		hl.set_color("string", Color::srgb(1.0, 0.0, 0.0));
		hl.set_color("string.escape", Color::srgb(0.0, 1.0, 0.0));
		hl.color_for("string.escape")
			.unwrap()
			.to_srgba()
			.green
			.xpect_close(1.0);
		// fallback for unknown subkind
		hl.color_for("string.regexp")
			.unwrap()
			.to_srgba()
			.red
			.xpect_close(1.0);
	}

	#[beet_core::test]
	fn aliases_resolve() {
		let mut hl = SyntaxHighlighting::with_defaults();
		hl.highlight("rs", "fn main() {}")
			.iter()
			.any(|s| s.capture.as_deref() == Some("keyword"))
			.xpect_true();
	}

	#[beet_core::test]
	fn spans_cover_source() {
		let mut hl = SyntaxHighlighting::with_defaults();
		let src = "let x = 42;\n// comment\n";
		hl.highlight("rust", src)
			.iter()
			.map(|s| s.text.as_str())
			.collect::<String>()
			.xpect_eq(src);
	}
}
