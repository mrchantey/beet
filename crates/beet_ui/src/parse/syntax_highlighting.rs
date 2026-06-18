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
//! Registered as a [`Resource`] by [`StylePlugin`], and applied via the
//! [`apply_syntax_highlighting`] system in the [`PostParseTree`] schedule.
//!
//! Only enabled with the `syntax_highlighting` feature.

use crate::prelude::*;
use crate::style::syntax::tokens as tokens_mod;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use beet_core::prelude::*;
use streaming_iterator::StreamingIterator;
use tree_sitter::Language;
use tree_sitter::Parser;
use tree_sitter::Query as TsQuery;
use tree_sitter::QueryCursor;

/// Registry of tree-sitter grammars used to tokenize fenced code blocks.
///
/// [`Default`] returns a registry pre-populated with rust, javascript,
/// json, and html grammars; this is what [`StylePlugin`] inserts via
/// `init_resource`.
#[derive(Clone, Resource)]
pub struct SyntaxHighlighting {
	languages: HashMap<SmolStr, LanguageEntry>,
}

impl Default for SyntaxHighlighting {
	fn default() -> Self { Self::with_defaults() }
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
	/// Compiled once at registration and shared (`Arc`) across every
	/// [`highlight`](SyntaxHighlighting::highlight) call. Recompiling the large
	/// grammar queries (eg rust) per code block dominated markdown page render
	/// time; caching the compiled query keeps it off the hot path.
	query: Arc<TsQuery>,
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
		let mut this = Self {
			languages: HashMap::default(),
		};
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
		// compile up-front so a bad grammar fails registration, not highlight, and
		// the cached query is reused by every `highlight` call.
		let query =
			TsQuery::new(&language, &query_source).expect("query compiles");
		let aliases = aliases.iter().map(|s| SmolStr::from(*s)).collect();
		self.languages.insert(name, LanguageEntry {
			language,
			query: Arc::new(query),
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

		let query = entry.query.as_ref();
		let capture_names: Vec<&str> = query.capture_names().to_vec();

		// 1. collect captures sorted by start byte (BTreeMap keeps order)
		let mut events: BTreeMap<usize, (usize, u32)> = BTreeMap::new();
		let mut cursor = QueryCursor::new();
		let mut matches =
			cursor.captures(query, tree.root_node(), source.as_bytes());
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
}

/// Walk every `<code class="language-X">` element in the world whose only
/// child is a text node, and replace that text node with one span element
/// per highlight run.
///
/// Each emitted span has:
/// - tag `span`
/// - attribute `class="hl-<capture>"` for captured runs (omitted for plain runs)
/// - a single text child carrying the original slice of source
///
/// Idempotent: once a code element's text child has been replaced with
/// span elements, [`ElementView::inner_text`] no longer matches, so the
/// system skips it on subsequent runs.
pub fn apply_syntax_highlighting(
	mut commands: Commands,
	highlighter: Res<SyntaxHighlighting>,
	elements: ElementQuery,
) {
	for view in elements.iter() {
		if view.tag() != "code" {
			continue;
		}
		let Some((text_entity, value)) = view.inner_text else {
			continue;
		};
		let Ok(source) = value.as_str() else { continue };
		let Some(lang) = view.iter_classes().find_map(|c| {
			let name = c.strip_prefix("language-").unwrap_or(&c);
			(!name.is_empty()).then(|| SmolStr::from(name))
		}) else {
			continue;
		};
		commands.entity(text_entity).despawn();
		for span in highlighter.highlight(&lang, source) {
			spawn_highlight_span(&mut commands, view.entity, span);
		}
	}
}

/// The list of capture names this module recognises, matching
/// [`tokens_mod::recognised_names`].
pub fn recognised_names() -> &'static [&'static str] {
	tokens_mod::recognised_names()
}

/// Spawn a single highlight span as a child of `parent`. When `span.capture`
/// is set, an `Attribute("class") = "hl-<capture>"` entity is also spawned.
fn spawn_highlight_span(
	commands: &mut Commands,
	parent: Entity,
	span: HighlightSpan,
) {
	let element = commands.spawn((Element::new("span"), ChildOf(parent))).id();
	if let Some(capture) = &span.capture {
		let class = format!("hl-{capture}");
		commands.spawn((
			Attribute::new("class"),
			Value::str(class),
			AttributeOf::new(element),
		));
	}
	commands.spawn((Value::Str(span.text), ChildOf(element)));
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
