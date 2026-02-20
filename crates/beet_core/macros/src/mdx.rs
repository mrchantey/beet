//! Proc macro implementation for the `mdx!` macro.
//!
//! Parses a token stream containing markdown text interspersed with
//! `{}` bundle expressions. Brace groups that are NOT escaped (ie
//! `{{escaped}}`) are treated as bundle expressions. Everything else
//! is collected as markdown text and passed to the `markdown()`
//! function at runtime.
//!
//! The crate path is resolved automatically via
//! [`pkg_ext::internal_or_beet`], so callers invoke `mdx!` directly
//! without providing a `$crate` prefix.
//!
//! # Input Format
//!
//! ```text
//! mdx!(# Heading text {bundle_expr} more text)
//! mdx!("string with {interpolation}")
//! mdx!(r#"raw string with {interpolation}"#)
//! ```
use beet_core_shared::pkg_ext;
use proc_macro2::TokenStream;
use proc_macro2::TokenTree;
use quote::quote;

/// A segment of parsed MDX content.
enum Segment {
	/// Markdown text to be passed to `markdown()`.
	Markdown(String),
	/// A raw bundle expression from inside `{}`.
	Bundle(TokenStream),
}

/// Check if a token stream starts with a string literal and return
/// it if so. Returns `None` if the content is raw tokens.
fn try_extract_string_literal(
	tokens: &[TokenTree],
) -> Option<(String, proc_macro2::Span)> {
	if tokens.len() != 1 {
		return None;
	}
	match &tokens[0] {
		TokenTree::Literal(lit) => {
			let repr = lit.to_string();
			let span = lit.span();
			// Check for raw string r#"..."# or regular string "..."
			if repr.starts_with("r#\"")
				|| repr.starts_with("r\"")
				|| repr.starts_with('"')
			{
				// Use syn to properly parse the string literal
				let parsed: syn::LitStr = syn::parse_str(&repr).ok()?;
				Some((parsed.value(), span))
			} else {
				None
			}
		}
		_ => None,
	}
}

/// Parse a string literal's content, splitting on unescaped `{}`
/// interpolation boundaries.
fn parse_string_content(
	content: &str,
	caller_span: proc_macro2::Span,
) -> Vec<Segment> {
	let mut segments = Vec::new();
	let mut text_buf = String::new();
	let chars: Vec<char> = content.chars().collect();
	let mut idx = 0;

	while idx < chars.len() {
		if chars[idx] == '{' {
			if idx + 1 < chars.len() && chars[idx + 1] == '{' {
				// Escaped {{ → literal {
				text_buf.push('{');
				idx += 2;
				continue;
			}
			// Tentatively scan ahead for matching `}` and collect
			// the expression content before deciding whether this
			// is an interpolation or literal braces.
			let scan_start = idx;
			idx += 1; // skip the `{`
			let mut depth = 1u32;
			let mut expr_str = String::new();
			while idx < chars.len() && depth > 0 {
				if chars[idx] == '{' {
					depth += 1;
					expr_str.push('{');
				} else if chars[idx] == '}' {
					depth -= 1;
					if depth > 0 {
						expr_str.push('}');
					}
				} else {
					expr_str.push(chars[idx]);
				}
				idx += 1;
			}
			let expr_str = expr_str.trim().to_string();
			if expr_str.is_empty() {
				// Empty `{}` — treat as literal braces
				text_buf.push_str("{}");
			} else if let Ok(ts) = expr_str.parse::<TokenStream>() {
				// Remap spans to the caller's string literal so
				// identifiers resolve in the caller's scope.
				let ts = remap_spans(ts, caller_span);
				// Valid expression — flush accumulated text first,
				// then add the bundle segment
				if !text_buf.is_empty() {
					segments
						.push(Segment::Markdown(std::mem::take(&mut text_buf)));
				}
				segments.push(Segment::Bundle(ts));
			} else {
				// Not valid Rust — treat as literal text by
				// reconstructing from the char slice
				let literal: String = chars[scan_start..idx].iter().collect();
				text_buf.push_str(&literal);
			}
		} else if chars[idx] == '}' {
			if idx + 1 < chars.len() && chars[idx + 1] == '}' {
				// Escaped }} → literal }
				text_buf.push('}');
				idx += 2;
				continue;
			}
			// Stray `}` — just include it
			text_buf.push('}');
			idx += 1;
		} else {
			text_buf.push(chars[idx]);
			idx += 1;
		}
	}

	if !text_buf.is_empty() {
		segments.push(Segment::Markdown(text_buf));
	}

	segments
}

/// Parse raw tokens (non-string-literal mode), splitting by brace
/// groups. Adjacent non-brace tokens are collected as markdown text
/// by reconstructing their string representation.
fn parse_raw_tokens(tokens: Vec<TokenTree>) -> Vec<Segment> {
	let mut segments = Vec::new();
	let mut text_buf = String::new();
	let mut last_was_punct = false;
	let mut last_was_hash = false;

	for tt in tokens {
		match tt {
			TokenTree::Group(group)
				if group.delimiter() == proc_macro2::Delimiter::Brace =>
			{
				// Check for escaped `{{...}}` by peeking at inner
				// content — if the inner content is itself a single
				// brace group, treat the outer as escaped.
				let inner: Vec<TokenTree> =
					group.stream().into_iter().collect();
				if inner.len() == 1 {
					if let TokenTree::Group(ref inner_group) = inner[0] {
						if inner_group.delimiter()
							== proc_macro2::Delimiter::Brace
						{
							// Escaped {{content}} → literal braces + content
							text_buf.push('{');
							text_buf
								.push_str(&inner_group.stream().to_string());
							text_buf.push('}');
							last_was_punct = false;
							last_was_hash = false;
							continue;
						}
					}
				}

				// Unescaped {expr} → flush text and add bundle
				if !text_buf.is_empty() {
					segments
						.push(Segment::Markdown(std::mem::take(&mut text_buf)));
				}
				segments.push(Segment::Bundle(group.stream()));
				last_was_punct = false;
				last_was_hash = false;
			}
			TokenTree::Punct(punct) => {
				let ch = punct.as_char();
				// Don't add space before punctuation that is part
				// of markdown syntax
				if !text_buf.is_empty()
					&& !last_was_punct
					&& !last_was_hash
					&& !matches!(
						ch,
						'#' | '*'
							| '_' | '~' | '`' | '-'
							| '>' | '.' | '!' | '['
							| ']' | '(' | ')' | ':'
							| ','
					) {
					// Only add space before certain punctuation
				}

				// Handle spacing: no space before most punctuation,
				// but reconstruct markdown naturally
				if matches!(ch, '#') && text_buf.is_empty()
					|| text_buf.ends_with('\n')
				{
					// Hash at start of line — heading marker
				} else if matches!(ch, '#') && last_was_hash {
					// Consecutive hashes
				} else if !text_buf.is_empty()
					&& !text_buf.ends_with(' ')
					&& !text_buf.ends_with('\n')
					&& !last_was_punct
					&& matches!(ch, '#')
				{
					text_buf.push(' ');
				}

				text_buf.push(ch);
				last_was_punct = true;
				last_was_hash = ch == '#';
			}
			TokenTree::Ident(ident) => {
				if !text_buf.is_empty()
					&& !text_buf.ends_with(' ')
					&& !text_buf.ends_with('\n')
					&& !last_was_hash
				{
					text_buf.push(' ');
				}
				text_buf.push_str(&ident.to_string());
				last_was_punct = false;
				last_was_hash = false;
			}
			TokenTree::Literal(lit) => {
				if !text_buf.is_empty()
					&& !text_buf.ends_with(' ')
					&& !text_buf.ends_with('\n')
				{
					text_buf.push(' ');
				}
				text_buf.push_str(&lit.to_string());
				last_was_punct = false;
				last_was_hash = false;
			}
			TokenTree::Group(group) => {
				// Non-brace groups (parens, brackets) — include as
				// text
				let (open, close) = match group.delimiter() {
					proc_macro2::Delimiter::Parenthesis => ("(", ")"),
					proc_macro2::Delimiter::Bracket => ("[", "]"),
					proc_macro2::Delimiter::None => ("", ""),
					proc_macro2::Delimiter::Brace => {
						unreachable!("handled above")
					}
				};
				text_buf.push_str(open);
				text_buf.push_str(&group.stream().to_string());
				text_buf.push_str(close);
				last_was_punct = false;
				last_was_hash = false;
			}
		}
	}

	if !text_buf.is_empty() {
		segments.push(Segment::Markdown(text_buf));
	}

	segments
}

/// Generate the output token stream from parsed segments.
fn generate_output(segments: Vec<Segment>) -> TokenStream {
	let crate_path = pkg_ext::internal_or_beet("beet_stack");

	// Filter out empty markdown segments
	let segments: Vec<Segment> = segments
		.into_iter()
		.filter(|seg| match seg {
			Segment::Markdown(text) => !text.trim().is_empty(),
			Segment::Bundle(_) => true,
		})
		.collect();

	if segments.is_empty() {
		return quote! { () };
	}

	let segment_tokens: Vec<TokenStream> = segments
		.into_iter()
		.map(|seg| match seg {
			Segment::Markdown(text) => {
				quote! { #crate_path::prelude::markdown(#text) }
			}
			Segment::Bundle(expr) => {
				quote! { #expr }
			}
		})
		.collect();

	if segment_tokens.len() == 1 {
		let single = &segment_tokens[0];
		quote! { #single }
	} else {
		quote! {
			::bevy::prelude::children![
				#(#segment_tokens),*
			]
		}
	}
}

/// Recursively remap all spans in a token stream to the given span,
/// ensuring generated identifiers resolve in the caller's scope.
fn remap_spans(ts: TokenStream, span: proc_macro2::Span) -> TokenStream {
	ts.into_iter()
		.map(|mut tt| {
			tt.set_span(span);
			if let TokenTree::Group(group) = tt {
				let inner = remap_spans(group.stream(), span);
				let mut new_group =
					proc_macro2::Group::new(group.delimiter(), inner);
				new_group.set_span(span);
				TokenTree::Group(new_group)
			} else {
				tt
			}
		})
		.collect()
}

/// Entry point for the `mdx` proc macro.
///
/// Content tokens are parsed directly — no crate path prefix needed.
pub fn impl_mdx(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let input2: TokenStream = input.into();
	let content_tokens: Vec<TokenTree> = input2.into_iter().collect();

	let segments = match try_extract_string_literal(&content_tokens) {
		Some((string_content, span)) => {
			parse_string_content(&string_content, span)
		}
		None => parse_raw_tokens(content_tokens),
	};

	let output = generate_output(segments);
	output.into()
}
