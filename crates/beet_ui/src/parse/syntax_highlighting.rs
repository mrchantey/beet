//! Tree-sitter based syntax highlighting for code blocks.
//!
//! [`SyntaxHighlighting`] holds a per-language [`Language`] grammar and
//! highlight query source. Given a language identifier and a source
//! slice, [`Self::highlight`] returns a sequence of [`HighlightSpan`]s
//! covering every byte of the input exactly once.
//!
//! Capture names are emitted verbatim, with no colour resolution: the
//! renderer is responsible for mapping `hl-<capture>` classes to styles.
//!
//! Only enabled with the `syntax_highlighting` feature.

use crate::prelude::*;
use crate::style::syntax::tokens as tokens_mod;
use alloc::collections::BTreeMap;
use beet_core::prelude::*;
use streaming_iterator::StreamingIterator;
use tree_sitter::Language;
use tree_sitter::Parser;
use tree_sitter::Query as TsQuery;
use tree_sitter::QueryCursor;

/// Registry of tree-sitter grammars used to tokenize fenced code blocks.
#[derive(Default, Clone)]
pub struct SyntaxHighlighting {
	languages: HashMap<SmolStr, LanguageEntry>,
}

impl core::fmt::Debug for SyntaxHighlighting {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("SyntaxHighlighting")
			.field("languages", &self.languages.keys().collect::<Vec<_>>())
			.finish()
	}
}

#[derive(Clone)]
struct LanguageEntry {
	language: Language,
	query_source: SmolStr,
	aliases: Vec<SmolStr>,
}

/// A contiguous slice of highlighted source text.
#[derive(Debug, Clone, PartialEq)]
pub struct HighlightSpan {
	/// Owned slice of the original source for this span.
	pub text: SmolStr,
	/// Recognised capture name like `keyword`, or `None` for unstyled
	/// regions between captures.
	pub capture: Option<SmolStr>,
}

impl SyntaxHighlighting {
	/// A highlighter pre-registered with `rust`, `javascript`, `json`,
	/// and `html` (plus common aliases).
	pub fn with_defaults() -> Self {
		let mut this = Self::default();
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
		)
	}

	/// Register the javascript grammar.
	pub fn add_javascript(&mut self) -> &mut Self {
		self.add_language(
			"javascript",
			tree_sitter_javascript::LANGUAGE.into(),
			tree_sitter_javascript::HIGHLIGHT_QUERY,
			&["js", "jsx", "mjs"],
		)
	}

	/// Register the json grammar.
	pub fn add_json(&mut self) -> &mut Self {
		self.add_language(
			"json",
			tree_sitter_json::LANGUAGE.into(),
			tree_sitter_json::HIGHLIGHTS_QUERY,
			&[],
		)
	}

	/// Register the html grammar.
	pub fn add_html(&mut self) -> &mut Self {
		self.add_language(
			"html",
			tree_sitter_html::LANGUAGE.into(),
			tree_sitter_html::HIGHLIGHTS_QUERY,
			&["htm"],
		)
	}

	/// Register a tree-sitter [`Language`] under `name` with a highlight query.
	/// Optional `aliases` map alternate identifiers (eg `rs`) to the same entry.
	///
	/// Panics if the query fails to compile against the language.
	pub fn add_language(
		&mut self,
		name: impl Into<SmolStr>,
		language: Language,
		query: impl Into<SmolStr>,
		aliases: &[&str],
	) -> &mut Self {
		let name = name.into();
		let query_source = query.into();
		// validate up-front so a bad grammar fails registration, not highlight
		TsQuery::new(&language, &query_source).expect("query compiles");
		let aliases = aliases.iter().map(|s| SmolStr::from(*s)).collect();
		self.languages.insert(name, LanguageEntry {
			language,
			query_source,
			aliases,
		});
		self
	}

	/// Resolve a code-block info string (eg `rust` or `rs`) to a registered
	/// language entry.
	fn resolve_language(&self, name: &str) -> Option<&LanguageEntry> {
		let lower = name.to_ascii_lowercase();
		if let Some(entry) = self.languages.get(lower.as_str()) {
			return Some(entry);
		}
		self.languages
			.values()
			.find(|entry| entry.aliases.iter().any(|a| a.as_str() == lower))
	}

	/// Tokenize `source` for `lang`, returning a complete cover of the input
	/// where each byte appears in exactly one [`HighlightSpan`].
	///
	/// Returns a single unstyled span if the language is unknown or parsing
	/// produced no tree, so callers can always render the result.
	pub fn highlight(&self, lang: &str, source: &str) -> Vec<HighlightSpan> {
		let Some(entry) = self.resolve_language(lang) else {
			return vec![HighlightSpan {
				text: source.into(),
				capture: None,
			}];
		};

		let mut parser = Parser::new();
		if parser.set_language(&entry.language).is_err() {
			return vec![HighlightSpan {
				text: source.into(),
				capture: None,
			}];
		}
		let Some(tree) = parser.parse(source.as_bytes(), None) else {
			return vec![HighlightSpan {
				text: source.into(),
				capture: None,
			}];
		};

		let query = TsQuery::new(&entry.language, &entry.query_source)
			.expect("query compiles");
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
					text: source[cursor_byte..start].into(),
					capture: None,
				});
			}
			let name = capture_names
				.get(capture_index as usize)
				.copied()
				.unwrap_or("");
			spans.push(HighlightSpan {
				text: source[start..end].into(),
				capture: Some(name.into()),
			});
			cursor_byte = end;
		}
		if cursor_byte < source.len() {
			spans.push(HighlightSpan {
				text: source[cursor_byte..].into(),
				capture: None,
			});
		}
		spans
	}

	/// Walk descendants of `root`, find each `<code class="language-X">`
	/// whose only child is a text node, and replace the text node with one
	/// span element per highlight run.
	///
	/// Each emitted span has:
	/// - tag `span`
	/// - attribute `class="hl-<capture>"` for captured runs (omitted for plain runs)
	/// - a single text child carrying the original slice of source
	pub fn apply(&self, world: &mut World, root: Entity) {
		let blocks = self.find_code_blocks(world, root);
		for (code_entity, lang, text_entity, source) in blocks {
			let spans = self.highlight(&lang, &source);
			world.entity_mut(text_entity).despawn();
			for span in spans {
				spawn_highlight_span(world, code_entity, span);
			}
		}
	}

	/// Collect every fenced code block descended from `root` that is
	/// eligible for syntax highlighting. Returns tuples of
	/// `(code_entity, language, text_entity, source_text)`.
	fn find_code_blocks(
		&self,
		world: &mut World,
		root: Entity,
	) -> Vec<(Entity, SmolStr, Entity, SmolStr)> {
		world.with_state::<(Query<&Children>, ElementQuery), _>(
			|(children, elements)| {
				children
					.iter_descendants_inclusive::<Children>(root)
					.filter_map(|entity| elements.get(entity).ok())
					.filter(|view| view.tag() == "code")
					.filter_map(|view| {
						let (text_entity, value) = view.inner_text?;
						let source = value.as_str().ok()?;
						let lang = view.iter_classes().find_map(|c| {
							let name =
								c.strip_prefix("language-").unwrap_or(&c);
							(!name.is_empty()).then(|| SmolStr::from(name))
						})?;
						Some((
							view.entity,
							lang,
							text_entity,
							SmolStr::from(source),
						))
					})
					.collect()
			},
		)
	}
}

/// The built-in colour palette, keyed by tree-sitter capture name.
///
/// Designed to be readable on both light and dark terminals. Renderers
/// can register these via a class map like `hl-<capture>`.
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

/// Spawn a single highlight span as a child of `parent`. When `span.capture`
/// is set, an `Attribute("class") = "hl-<capture>"` entity is also spawned.
fn spawn_highlight_span(
	world: &mut World,
	parent: Entity,
	span: HighlightSpan,
) {
	let element = world.spawn((Element::new("span"), ChildOf(parent))).id();
	if let Some(capture) = &span.capture {
		let class = format!("hl-{capture}");
		world.spawn((
			Attribute::new("class"),
			Value::str(class),
			AttributeOf::new(element),
		));
	}
	world.spawn((Value::Str(span.text), ChildOf(element)));
}


#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	fn highlights_simple_rust() {
		let hl = SyntaxHighlighting::with_defaults();
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
		let hl = SyntaxHighlighting::with_defaults();
		let spans = hl.highlight("nonesuch", "abc");
		spans.len().xpect_eq(1);
		spans[0].text.as_str().xpect_eq("abc");
		spans[0].capture.is_none().xpect_true();
	}

	#[beet_core::test]
	fn aliases_resolve() {
		let hl = SyntaxHighlighting::with_defaults();
		hl.highlight("rs", "fn main() {}")
			.iter()
			.any(|s| s.capture.as_deref() == Some("keyword"))
			.xpect_true();
	}

	#[beet_core::test]
	fn spans_cover_source() {
		let hl = SyntaxHighlighting::with_defaults();
		let src = "let x = 42;\n// comment\n";
		hl.highlight("rust", src)
			.iter()
			.map(|s| s.text.as_str())
			.collect::<String>()
			.xpect_eq(src);
	}
}
