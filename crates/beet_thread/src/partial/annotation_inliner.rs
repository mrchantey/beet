use crate::o11s::Annotation;
use crate::o11s::UrlCitation;
use beet_core::prelude::*;

use super::PostPartialKey;

#[derive(Debug, Clone, Default)]
struct AnnotationState {
	original_text: String,
	annotations: Vec<Annotation>,
}

// Inlines openresponses annotations into text as markdown links.
// Supports footnote style (default) where citations are appended
// at the bottom, or inline style where they replace the annotated span.
//
// Tracks per-item original text and accumulated annotations to avoid
// position drift from incremental insertion. Annotations always reference
// positions in the original text, so we re-render from scratch each time.
#[derive(Debug, Clone)]
pub struct AnnotationInliner {
	footnote_style: bool,
	/// Per-item state keyed by [`PartialItemKey`], storing original text
	/// and all annotations received so far.
	item_state: HashMap<PostPartialKey, AnnotationState>,
}

impl Default for AnnotationInliner {
	fn default() -> Self {
		Self {
			footnote_style: true,
			item_state: HashMap::default(),
		}
	}
}

impl AnnotationInliner {
	pub fn new() -> Self { Self::default() }

	pub fn with_footnote_style(mut self, footnote_style: bool) -> Self {
		self.footnote_style = footnote_style;
		self
	}

	/// Store the original (un-annotated) text for a given item key.
	/// Called when `TextDone` arrives or when full content is first known.
	pub fn set_original_text(&mut self, key: PostPartialKey, text: String) {
		self.item_state.entry(key).or_default().original_text = text;
	}

	/// Add an annotation for the given item key and re-render the full
	/// annotated text from the original. Returns the rendered text, or
	/// `None` if no original text has been stored for this key.
	pub fn push_annotation(
		&mut self,
		key: &PostPartialKey,
		annotation: Annotation,
	) -> Option<String> {
		self.item_state.get_mut(key)?.annotations.push(annotation);
		self.render(key)
	}

	/// Render the current annotated text for a key without adding a new
	/// annotation. Returns `None` if no original text is stored.
	pub fn render(&self, key: &PostPartialKey) -> Option<String> {
		let state = self.item_state.get(key)?;
		if state.annotations.is_empty() {
			return Some(state.original_text.clone());
		}
		Some(self.inline_annotations(&state.original_text, &state.annotations))
	}

	/// Stateless one-shot: inline all annotations into the given text.
	/// Annotations are sorted by start_index so replacements
	/// dont shift indices of earlier annotations.
	pub fn inline_annotations(
		&self,
		text: &str,
		annotations: &[Annotation],
	) -> String {
		if annotations.is_empty() {
			return text.to_string();
		}

		let mut citations: Vec<&UrlCitation> = annotations
			.iter()
			.map(|annotation| match annotation {
				Annotation::UrlCitation(citation) => citation,
			})
			.collect();

		// sort by start_index ascending for consistent footnote numbering
		citations.sort_by_key(|citation| citation.start_index);

		if self.footnote_style {
			self.inline_footnote_style(text, &citations)
		} else {
			self.inline_replace_style(text, &citations)
		}
	}

	// Appends footnote references in-text and a footnote section at the bottom.
	// eg: `some cited text[^1]` with `[^1]: [title](url)` at the bottom
	fn inline_footnote_style(
		&self,
		text: &str,
		citations: &[&UrlCitation],
	) -> String {
		let char_indices: Vec<(usize, char)> = text.char_indices().collect();
		let char_count = char_indices.len();

		// build a map of char_offset -> byte_offset for end positions
		let char_to_byte: Vec<usize> =
			char_indices.iter().map(|(byte_idx, _)| *byte_idx).collect();

		// collect insertion points (byte offset after annotated span)
		// and footnote entries, processing in reverse byte order
		// so insertions dont shift earlier positions
		let mut insertions: Vec<(usize, String)> = Vec::new();
		let mut footnotes: Vec<String> = Vec::new();

		for (index, citation) in citations.iter().enumerate() {
			let footnote_num = index + 1;
			let end_char = citation.end_index as usize;

			// byte offset right after the annotated span
			let insert_byte = if end_char < char_count {
				char_to_byte[end_char]
			} else {
				text.len()
			};

			insertions.push((insert_byte, format!("[^{footnote_num}]")));

			let title = if citation.title.is_empty() {
				&citation.url
			} else {
				&citation.title
			};
			footnotes.push(format!(
				"[^{footnote_num}]: [{title}]({})",
				citation.url
			));
		}

		// apply insertions in reverse byte order to preserve positions
		insertions.sort_by(|lhs, rhs| rhs.0.cmp(&lhs.0));
		let mut result = text.to_string();
		for (byte_offset, ref_text) in &insertions {
			result.insert_str(*byte_offset, ref_text);
		}

		// append footnotes section
		if !footnotes.is_empty() {
			result.push_str("\n\n");
			result.push_str(&footnotes.join("\n"));
		}

		result
	}

	// Replaces the annotated span with a markdown link inline.
	// eg: `[cited text](url "title")`
	fn inline_replace_style(
		&self,
		text: &str,
		citations: &[&UrlCitation],
	) -> String {
		let char_indices: Vec<(usize, char)> = text.char_indices().collect();
		let char_count = char_indices.len();

		let char_to_byte: Vec<usize> =
			char_indices.iter().map(|(byte_idx, _)| *byte_idx).collect();

		// process in reverse order so replacements dont shift positions
		let mut sorted: Vec<&&UrlCitation> = citations.iter().collect();
		sorted.sort_by(|lhs, rhs| rhs.start_index.cmp(&lhs.start_index));

		let mut result = text.to_string();

		for citation in sorted {
			let start_char = citation.start_index as usize;
			let end_char = citation.end_index as usize;

			let start_byte = if start_char < char_count {
				char_to_byte[start_char]
			} else {
				result.len()
			};
			let end_byte = if end_char < char_count {
				char_to_byte[end_char]
			} else {
				result.len()
			};

			let span_text = &result[start_byte..end_byte];
			let replacement = if citation.title.is_empty() {
				format!("[{span_text}]({})", citation.url)
			} else {
				format!(
					"[{span_text}]({} \"{}\")",
					citation.url, citation.title
				)
			};

			result.replace_range(start_byte..end_byte, &replacement);
		}

		result
	}
}


#[cfg(test)]
mod test {
	use super::*;

	fn make_citation(
		url: &str,
		title: &str,
		start: u32,
		end: u32,
	) -> Annotation {
		Annotation::UrlCitation(UrlCitation {
			url: url.to_string(),
			title: title.to_string(),
			start_index: start,
			end_index: end,
		})
	}

	fn make_key(id: &str, index: u32) -> PostPartialKey {
		PostPartialKey::Content {
			responses_id: id.to_string(),
			content_index: index,
		}
	}

	#[test]
	fn footnote_single_citation() {
		let inliner = AnnotationInliner::new();
		let text = "Check out this cool article for more details.";
		let annotations =
			vec![make_citation("https://example.com", "Example", 14, 30)];

		let result = inliner.inline_annotations(text, &annotations);
		assert!(result.contains("[^1]"));
		assert!(result.contains("[^1]: [Example](https://example.com)"));
	}

	#[test]
	fn footnote_multiple_citations() {
		let inliner = AnnotationInliner::new();
		let text = "First source and second source are both great.";
		let annotations = vec![
			make_citation("https://a.com", "Source A", 0, 12),
			make_citation("https://b.com", "Source B", 17, 30),
		];

		let result = inliner.inline_annotations(text, &annotations);
		assert!(result.contains("[^1]"));
		assert!(result.contains("[^2]"));
		assert!(result.contains("[^1]: [Source A](https://a.com)"));
		assert!(result.contains("[^2]: [Source B](https://b.com)"));
	}

	#[test]
	fn inline_replace_single() {
		let inliner = AnnotationInliner::new().with_footnote_style(false);
		let text = "Visit example for info.";
		let annotations =
			vec![make_citation("https://example.com", "Example", 6, 13)];

		let result = inliner.inline_annotations(text, &annotations);
		assert_eq!(
			result,
			"Visit [example](https://example.com \"Example\") for info."
		);
	}

	#[test]
	fn empty_annotations_passthrough() {
		let inliner = AnnotationInliner::new();
		let text = "No annotations here.";
		let result = inliner.inline_annotations(text, &[]);
		assert_eq!(result, text);
	}

	#[test]
	fn empty_title_uses_url() {
		let inliner = AnnotationInliner::new();
		let text = "Some text here.";
		let annotations = vec![make_citation("https://example.com", "", 5, 9)];

		let result = inliner.inline_annotations(text, &annotations);
		assert!(
			result.contains("[^1]: [https://example.com](https://example.com)")
		);
	}

	#[test]
	fn incremental_annotations_avoid_position_drift() {
		let mut inliner = AnnotationInliner::new();
		let key = make_key("resp1", 0);
		let text = "First source and second source are both great.";

		inliner.set_original_text(key.clone(), text.to_string());

		// first annotation arrives
		let result = inliner
			.push_annotation(
				&key,
				make_citation("https://a.com", "Source A", 0, 12),
			)
			.unwrap();
		assert!(result.contains("[^1]"));
		assert!(!result.contains("[^2]"));

		// second annotation arrives incrementally
		let result = inliner
			.push_annotation(
				&key,
				make_citation("https://b.com", "Source B", 17, 30),
			)
			.unwrap();

		// both annotations rendered correctly from original positions
		assert!(result.contains("[^1]"));
		assert!(result.contains("[^2]"));
		assert!(result.contains("[^1]: [Source A](https://a.com)"));
		assert!(result.contains("[^2]: [Source B](https://b.com)"));

		// verify it matches one-shot rendering
		let oneshot = inliner.inline_annotations(text, &[
			make_citation("https://a.com", "Source A", 0, 12),
			make_citation("https://b.com", "Source B", 17, 30),
		]);
		assert_eq!(result, oneshot);
	}

	#[test]
	fn render_without_annotations() {
		let mut inliner = AnnotationInliner::new();
		let key = make_key("resp1", 0);
		let text = "Hello world.";

		inliner.set_original_text(key.clone(), text.to_string());
		let result = inliner.render(&key).unwrap();
		assert_eq!(result, text);
	}

	#[test]
	fn render_missing_key_returns_none() {
		let inliner = AnnotationInliner::new();
		let key = make_key("missing", 0);
		assert!(inliner.render(&key).is_none());
	}

	#[test]
	fn push_annotation_missing_key_returns_none() {
		let mut inliner = AnnotationInliner::new();
		let key = make_key("missing", 0);
		let result = inliner
			.push_annotation(&key, make_citation("https://a.com", "A", 0, 5));
		assert!(result.is_none());
	}
}
