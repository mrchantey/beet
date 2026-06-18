//! A recoverable, panic-free, source-text-free parser from a `TokenStream` to
//! the [`ast`](super::ast).
//!
//! Recoverable: on malformed markup it collects [`Diagnostic`]s and still returns
//! a best-effort tree, so the macro emits every error at once and stays
//! IDE-friendly. Source-text-free: parsing depends only on token *structure*,
//! never [`Span::source_text`], so the result is identical whether the tokens
//! came from source or from a macro expansion. That is what lets a nested `rsx!`
//! expand safely inside `#[template]` output.
//!
use super::ast::*;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use proc_macro2::TokenTree;
use proc_macro2_diagnostics::Diagnostic;
use proc_macro2_diagnostics::Level;
use syn::Ident;
use syn::LitStr;
use syn::Token;
use syn::ext::IdentExt;
use syn::parse::ParseStream;
use syn::parse::Parser as _;
use syn::spanned::Spanned;
use syn::token::Brace;

/// Parse a markup token stream into the node tree plus any diagnostics. Always
/// returns a (possibly partial) tree; it never panics and never errors out.
pub fn parse_rsx(input: TokenStream) -> (Vec<RsxNode>, Vec<Diagnostic>) {
	let mut parser = RsxParser::default();
	// the closure borrows the parser for the duration of the sub-parse, then
	// releases it so the collected diagnostics can be read back out.
	let nodes = (|input: ParseStream| Ok(parser.nodes(input)))
		.parse2(input)
		.unwrap_or_default();
	(nodes, parser.errors)
}

/// Parser state: the accumulated diagnostics. The tree is returned by value.
#[derive(Default)]
struct RsxParser {
	errors: Vec<Diagnostic>,
}

impl RsxParser {
	/// Push a spanned error diagnostic.
	fn error(&mut self, span: Span, message: &str) {
		self.errors.push(Diagnostic::spanned(
			span,
			Level::Error,
			message.to_string(),
		));
	}

	/// Convert a `syn::Result` to an `Option`, recording any error.
	fn ok<T>(&mut self, result: syn::Result<T>) -> Option<T> {
		match result {
			Ok(value) => Some(value),
			Err(err) => {
				self.errors.push(err.into());
				None
			}
		}
	}

	/// Parse a sequence of sibling nodes until the input is exhausted.
	fn nodes(&mut self, input: ParseStream) -> Vec<RsxNode> {
		let mut nodes = Vec::new();
		while !input.is_empty() {
			let before = input.cursor();
			if let Some(node) = self.node(input) {
				nodes.push(node);
			}
			// a node that neither advanced nor errored would loop forever; skip a
			// token to guarantee progress.
			if input.cursor() == before {
				let _ = input.parse::<TokenTree>();
			}
		}
		nodes
	}

	/// Parse a single node, dispatching on the leading tokens.
	fn node(&mut self, input: ParseStream) -> Option<RsxNode> {
		if input.peek(Token![<]) {
			if input.peek2(Token![!]) {
				// `<!DOCTYPE ..>` (keyword follows) vs `<!-- .. -->`.
				if input.peek3(Ident::peek_any) {
					self.doctype(input).map(RsxNode::Doctype)
				} else {
					self.comment(input).map(RsxNode::Comment)
				}
			} else if input.peek2(Token![>]) {
				self.fragment(input).map(RsxNode::Fragment)
			} else {
				self.element(input).map(RsxNode::Element)
			}
		} else if input.peek(Brace) {
			self.ok(input.parse::<syn::Block>()).map(RsxNode::Block)
		} else if input.peek(LitStr) {
			self.ok(input.parse::<LitStr>()).map(RsxNode::Text)
		} else {
			self.error(
				input.span(),
				"unexpected token: text must be a quoted string literal, \
				 dynamic content must be `{expr}`",
			);
			let _ = input.parse::<TokenTree>();
			None
		}
	}

	/// Parse an element `<name ..>..</name>` or `<name ../>`.
	fn element(&mut self, input: ParseStream) -> Option<RsxElement> {
		self.ok(input.parse::<Token![<]>())?;
		let name = self.name(input)?;
		let (attr_tokens, self_closing) = self.scan_open_tag_end(input);
		let attributes = self.attributes(attr_tokens);
		let children = if self_closing {
			Vec::new()
		} else {
			let children = self.children(input);
			self.close_tag(input, &name);
			children
		};
		Some(RsxElement {
			name,
			attributes,
			children,
		})
	}

	/// Parse a `<>..</>` fragment.
	fn fragment(&mut self, input: ParseStream) -> Option<RsxFragment> {
		self.ok(input.parse::<Token![<]>())?;
		self.ok(input.parse::<Token![>]>())?;
		let children = self.children(input);
		if input.peek(Token![<]) && input.peek2(Token![/]) {
			let _ = input.parse::<Token![<]>();
			let _ = input.parse::<Token![/]>();
			if input.peek(Ident::peek_any) {
				self.error(
					input.span(),
					"expected fragment close `</>`, found an element close tag",
				);
			}
			let _ = input.parse::<Token![>]>();
		} else {
			self.error(
				input.span(),
				"fragment has no corresponding close `</>`",
			);
		}
		Some(RsxFragment { children })
	}

	/// Parse children up to (but not consuming) the next top-level `</`.
	fn children(&mut self, input: ParseStream) -> Vec<RsxNode> {
		let mut children = Vec::new();
		loop {
			if input.is_empty() {
				break;
			}
			// a `</` starts a close tag, ending this child list.
			if input.peek(Token![<]) && input.peek2(Token![/]) {
				break;
			}
			let before = input.cursor();
			if let Some(node) = self.node(input) {
				children.push(node);
			}
			if input.cursor() == before {
				break;
			}
		}
		children
	}

	/// Consume a `</name>` close tag, reporting a missing or mismatched one.
	fn close_tag(&mut self, input: ParseStream, open: &RsxName) {
		if !(input.peek(Token![<]) && input.peek2(Token![/])) {
			self.error(open.span(), "open tag has no corresponding close tag");
			return;
		}
		let _ = input.parse::<Token![<]>();
		let _ = input.parse::<Token![/]>();
		if let Some(close) = self.name(input) {
			if close.value() != open.value() {
				self.errors.push(
					Diagnostic::spanned(
						close.span(),
						Level::Error,
						"wrong close tag",
					)
					.span_help(open.span(), "the open tag started here"),
				);
			}
		}
		let _ = input.parse::<Token![>]>();
	}

	/// Parse a `<!DOCTYPE ..>`, returning the value with the keyword stripped.
	fn doctype(&mut self, input: ParseStream) -> Option<String> {
		self.ok(input.parse::<Token![<]>())?;
		self.ok(input.parse::<Token![!]>())?;
		let keyword = self.ok(input.call(Ident::parse_any))?;
		if !keyword.to_string().eq_ignore_ascii_case("doctype") {
			self.error(keyword.span(), "expected the DOCTYPE keyword");
		}
		// the value is whatever follows the keyword up to `>`, ie `html`.
		let mut value = String::new();
		while !input.peek(Token![>]) && !input.is_empty() {
			if let Some(tree) = self.ok(input.parse::<TokenTree>()) {
				value.push_str(&tree.to_string());
			}
		}
		let _ = input.parse::<Token![>]>();
		Some(value)
	}

	/// Parse a `<!-- "comment" -->`, returning the inner value.
	fn comment(&mut self, input: ParseStream) -> Option<String> {
		self.ok(input.parse::<Token![<]>())?;
		self.ok(input.parse::<Token![!]>())?;
		self.ok(input.parse::<Token![-]>())?;
		self.ok(input.parse::<Token![-]>())?;
		let mut value = String::new();
		loop {
			if input.peek(Token![-])
				&& input.peek2(Token![-])
				&& input.peek3(Token![>])
			{
				let _ = input.parse::<Token![-]>();
				let _ = input.parse::<Token![-]>();
				let _ = input.parse::<Token![>]>();
				break;
			}
			if input.is_empty() {
				self.error(
					input.span(),
					"comment has no corresponding close `-->`",
				);
				break;
			}
			if input.peek(LitStr) {
				if let Some(text) = self.ok(input.parse::<LitStr>()) {
					value.push_str(&text.value());
				}
			} else if let Some(tree) = self.ok(input.parse::<TokenTree>()) {
				value.push_str(&tree.to_string());
			}
		}
		Some(value)
	}

	/// Scan tokens up to the open-tag end (`>` or `/>`), returning the attribute
	/// tokens and whether the tag self-closes.
	///
	/// Scanning first, then sub-parsing the buffer, is what keeps an unbraced
	/// expression value safe: `<div bang=bang/>` is delimited at `/>` before the
	/// `/` could be mistaken for a division operator inside the value.
	fn scan_open_tag_end(&mut self, input: ParseStream) -> (TokenStream, bool) {
		let mut tokens = TokenStream::new();
		loop {
			if let Some(self_closing) = parse_open_tag_end(input) {
				return (tokens, self_closing);
			}
			if input.is_empty() {
				self.error(input.span(), "expected end of tag `>`");
				return (tokens, false);
			}
			if let Some(tree) = self.ok(input.parse::<TokenTree>()) {
				tokens.extend([tree]);
			}
		}
	}

	/// Sub-parse the bounded attribute token buffer into attributes.
	fn attributes(&mut self, tokens: TokenStream) -> Vec<RsxAttr> {
		if tokens.is_empty() {
			return Vec::new();
		}
		let mut attrs = Vec::new();
		let run = |input: ParseStream| {
			while !input.is_empty() {
				let before = input.cursor();
				if let Some(attr) = self.attribute(input) {
					attrs.push(attr);
				}
				if input.cursor() == before {
					let _ = input.parse::<TokenTree>();
				}
			}
			Ok(())
		};
		let _ = run.parse2(tokens);
		attrs
	}

	/// Parse one attribute: a bare `{..}` spread or a keyed `key`/`key=value`.
	fn attribute(&mut self, input: ParseStream) -> Option<RsxAttr> {
		if input.peek(Brace) {
			return self.ok(input.parse::<syn::Block>()).map(RsxAttr::Spread);
		}
		let key = self.name(input)?;
		let value = if input.peek(Token![=]) {
			let eq = self.ok(input.parse::<Token![=]>())?;
			if input.is_empty() {
				self.error(eq.span(), "missing attribute value");
				return None;
			}
			RsxAttrValue::Expr(self.ok(input.parse::<syn::Expr>())?)
		} else {
			RsxAttrValue::None
		};
		Some(RsxAttr::Keyed(RsxKeyedAttr { key, value }))
	}

	/// Parse a tag or attribute name: a `::`-path, a `-`/`:`/`.`-punctuated name,
	/// or a single ident.
	fn name(&mut self, input: ParseStream) -> Option<RsxName> {
		let first = self.ok(input.call(Ident::parse_any))?;
		let span = first.span();

		// a `::` makes it a Rust path, ie `path::to::Foo`.
		if input.peek(Token![::]) {
			let mut segments = syn::punctuated::Punctuated::new();
			segments.push_value(syn::PathSegment::from(first));
			while input.peek(Token![::]) {
				let sep = self.ok(input.parse::<Token![::]>())?;
				segments.push_punct(sep);
				let ident = self.ok(input.call(Ident::parse_any))?;
				segments.push_value(syn::PathSegment::from(ident));
			}
			let path = syn::Path {
				leading_colon: None,
				segments,
			};
			return Some(RsxName::Path { path, span });
		}

		// a `-`, `:`, or `.` makes it an SGML-style punctuated name.
		if peek_name_punct(input) {
			let mut value = first.to_string();
			while let Some(punct) = self.name_punct(input) {
				value.push(punct);
				if input.peek(Ident::peek_any) {
					let fragment = self.ok(input.call(Ident::parse_any))?;
					value.push_str(&fragment.to_string());
				} else if input.peek(syn::LitInt) {
					let fragment = self.ok(input.parse::<syn::LitInt>())?;
					value.push_str(&fragment.to_string());
				} else {
					break;
				}
			}
			return Some(RsxName::Punctuated { value, span });
		}

		// a lone ident, ie `div` or `class`.
		Some(RsxName::Path {
			path: syn::Path::from(syn::PathSegment::from(first)),
			span,
		})
	}

	/// Consume a single name punctuation (`-`, `:`, `.`), returning its char.
	fn name_punct(&mut self, input: ParseStream) -> Option<char> {
		if input.peek(Token![-]) {
			let _ = input.parse::<Token![-]>();
			Some('-')
		} else if input.peek(Token![:]) && !input.peek(Token![::]) {
			let _ = input.parse::<Token![:]>();
			Some(':')
		} else if input.peek(Token![.]) {
			let _ = input.parse::<Token![.]>();
			Some('.')
		} else {
			None
		}
	}
}

/// Whether the next token starts a name punctuation (`-`, `:` but not `::`, `.`).
fn peek_name_punct(input: ParseStream) -> bool {
	input.peek(Token![-])
		|| (input.peek(Token![:]) && !input.peek(Token![::]))
		|| input.peek(Token![.])
}

/// Consume an open-tag end if present, returning whether it self-closes (`/>`).
/// Leaves the stream untouched otherwise (eg a `/` of a division `a/b`).
fn parse_open_tag_end(input: ParseStream) -> Option<bool> {
	if input.peek(Token![/]) && input.peek2(Token![>]) {
		let _ = input.parse::<Token![/]>();
		let _ = input.parse::<Token![>]>();
		Some(true)
	} else if input.peek(Token![>]) {
		let _ = input.parse::<Token![>]>();
		Some(false)
	} else {
		None
	}
}

#[cfg(test)]
mod test {
	use super::super::lower::lower_nodes;
	use super::*;
	use alloc::string::String;
	use quote::quote;

	/// Parse then lower, asserting no diagnostics, returning the lowered string.
	fn lower(input: TokenStream) -> String {
		let (nodes, errors) = parse_rsx(input);
		assert!(errors.is_empty(), "unexpected diagnostics");
		lower_nodes(&nodes).to_string()
	}

	/// Parse, asserting a recoverable error: diagnostics plus a partial tree.
	fn recover(input: TokenStream) -> (usize, usize) {
		let (nodes, errors) = parse_rsx(input);
		assert!(!errors.is_empty(), "expected a diagnostic");
		(nodes.len(), errors.len())
	}

	#[test]
	fn element_arms() {
		assert!(lower(quote! { <div/> }).contains("Element :: new (\"div\")"));
		assert!(
			lower(quote! { <foo-bar/> })
				.contains("Element :: new (\"foo-bar\")")
		);
		let attrs = lower(quote! { <div class="card" disabled hidden=h/> });
		assert!(attrs.contains("Attribute :: new (\"class\")"));
		assert!(attrs.contains("Value :: new (\"card\")"));
		assert!(attrs.contains("Attribute :: new (\"disabled\")"));
		assert!(attrs.contains("Value :: new (h)"));
	}

	#[test]
	fn unbraced_expr_value_survives_self_close() {
		// the `/>` must not be mistaken for a division operator in `bang`.
		assert!(
			lower(quote! { <div bang=bang/> }).contains("Value :: new (bang)")
		);
	}

	#[test]
	fn text_and_block_arms() {
		assert!(
			lower(quote! { <p>"hi"</p> }).contains("Value :: new (\"hi\")")
		);
		assert!(lower(quote! { <p>{title}</p> }).contains("into_snippet"));
		assert!(
			lower(quote! { <div {Name::new("x")}/> })
				.contains("into_snippet_bundle")
		);
	}

	#[test]
	fn component_arms() {
		let out = lower(quote! { <Foo a=x b/> });
		assert!(out.contains("Foo {"));
		assert!(out.contains("into_prop"));
		assert!(out.contains("into_snippet_bundle"));
		// a lowercase-leading path tag stays an element (dispatch is on the first
		// char of the whole tag).
		assert!(
			lower(quote! { <foo::Bar/> })
				.contains("Element :: new (\"foo::Bar\")")
		);
		// an uppercase-leading path tag is a component, keeping its full path.
		assert!(lower(quote! { <Foo::Bar a=x/> }).contains("Foo :: Bar {"));
	}

	#[test]
	fn slot_arms() {
		assert!(lower(quote! { <Slot/> }).contains("SlotTarget :: new"));
		assert!(
			lower(quote! { <Slot name="x"/> })
				.contains("SlotTarget :: named (\"x\")")
		);
		// a slot transfer carries both a target and a child marker.
		let transfer = lower(quote! { <Slot name="x" bx:slot="y"/> });
		assert!(transfer.contains("SlotTarget :: named (\"x\")"));
		assert!(transfer.contains("SlotChild :: named (\"y\")"));
		// `slot=` routes an element into a parent slot.
		assert!(
			lower(quote! { <div slot="a"/> })
				.contains("SlotChild :: named (\"a\")")
		);
	}

	#[test]
	fn event_arm() {
		assert!(
			lower(quote! { <button onclick=handler/> })
				.contains("into_snippet")
		);
	}

	#[test]
	fn fragment_and_roots() {
		// a fragment of two children lowers to a `children!`.
		assert!(lower(quote! { <><br/><hr/></> }).contains("children !"));
		// multiple roots also lower to a `children!`.
		assert!(lower(quote! { <br/> <hr/> }).contains("children !"));
	}

	#[test]
	fn comment_and_doctype_arms() {
		assert!(
			lower(quote! { <!DOCTYPE html> })
				.contains("Doctype :: new (\"html\")")
		);
		assert!(
			lower(quote! { <!-- "note" --> })
				.contains("Comment :: new (\"note\")")
		);
	}

	#[test]
	fn recovers_from_mismatched_close() {
		// a partial tree (the element) plus a diagnostic, never a panic.
		let (nodes, _) = recover(quote! { <div></span> });
		assert_eq!(nodes, 1);
	}

	#[test]
	fn recovers_from_missing_close() { recover(quote! { <div> }); }

	#[test]
	fn recovers_from_unquoted_text() {
		// unquoted text is rejected, but parsing continues past it.
		recover(quote! { <p>hello</p> });
	}
}
