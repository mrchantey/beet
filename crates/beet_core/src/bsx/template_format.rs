//! The pluggable source-format layer beneath the [`BsxTemplateRegistry`] scan.
//!
//! A template file does not have to be `.bsx`: [`TemplateFormats`] maps a
//! [`MediaType`] to the parser that lowers that file's text into a [`BsxNode`]
//! tree, so the directory scan registers `<Foo>` from any format it knows. `.bsx`
//! parses through the full markup grammar; `.js` wraps its text verbatim in a
//! `<script>`, so a `Foo.js` registers `<Foo>` exactly as a `Foo.bsx` would. A
//! plugin teaches the scan a new format by inserting another parser.

use super::ast::*;
use super::parse::*;
use crate::prelude::*;

/// Lowers a template source file's text into a [`BsxNode`] tree.
///
/// A plain function pointer so a [`TemplateFormats`] entry is `Copy` and a plugin
/// registers a format with a free function, not a boxed closure.
pub type TemplateParseFn = fn(&str) -> Result<Vec<BsxNode>>;

/// Maps a [`MediaType`] to the [`TemplateParseFn`] that lowers a file of that type
/// into a [`BsxNode`] tree, the pluggable layer beneath the [`BsxTemplateRegistry`]
/// directory scan.
///
/// The scan keys on the file's media type, so `.bsx` resolves to the markup parser
/// and `.js` to the `<script>` wrapper. A file whose type has no registered format
/// is skipped, the same way a non-`.bsx` file was skipped before. Register a format
/// to teach the scan a new extension:
///
/// ```
/// # use beet_core::prelude::*;
/// let mut formats = TemplateFormats::default();
/// assert!(formats.get(&MediaType::Bsx).is_some());
/// assert!(formats.get(&MediaType::Javascript).is_some());
/// // `.css` is not a template format, so the scan skips it.
/// assert!(formats.get(&MediaType::Css).is_none());
/// ```
#[derive(Clone, Resource)]
pub struct TemplateFormats {
	parsers: HashMap<MediaType, TemplateParseFn>,
}

impl Default for TemplateFormats {
	/// The built-in formats: `.bsx` markup and `.js` (the `<script>` wrapper).
	fn default() -> Self {
		let mut formats = Self {
			parsers: HashMap::default(),
		};
		formats.insert(MediaType::Bsx, parse_bsx_format);
		formats.insert(MediaType::Javascript, parse_js_format);
		formats
	}
}

impl TemplateFormats {
	/// Register `parse` as the format for `media_type`, replacing any prior one.
	pub fn insert(
		&mut self,
		media_type: MediaType,
		parse: TemplateParseFn,
	) -> &mut Self {
		self.parsers.insert(media_type, parse);
		self
	}

	/// The parser registered for `media_type`, if any. The scan skips a file whose
	/// type returns `None`.
	pub fn get(&self, media_type: &MediaType) -> Option<TemplateParseFn> {
		self.parsers.get(media_type).copied()
	}
}

/// Parse a `.bsx` source through the full BSX grammar.
fn parse_bsx_format(source: &str) -> Result<Vec<BsxNode>> {
	parse_document(source, &BsxParseConfig::bsx())
}

/// Wrap a `.js` source verbatim in a single `<script>`, the JS-as-template form.
/// The body is raw text, so the renderer emits it unescaped (valid JS), matching
/// how the parser treats a hand-written `<script>`.
fn parse_js_format(source: &str) -> Result<Vec<BsxNode>> {
	let children = if source.is_empty() {
		Vec::new()
	} else {
		vec![BsxNode::Text(source.to_string())]
	};
	vec![BsxNode::Element(BsxElement {
		tag: "script".to_string(),
		tag_literal: None,
		attributes: Vec::new(),
		children,
		self_closing: false,
	})]
	.xok()
}

#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	fn default_registers_bsx_and_js() {
		let formats = TemplateFormats::default();
		formats.get(&MediaType::Bsx).is_some().xpect_true();
		formats.get(&MediaType::Javascript).is_some().xpect_true();
		// a non-template type is skipped by the scan.
		formats.get(&MediaType::Css).is_none().xpect_true();
	}

	#[beet_core::test]
	fn js_wraps_in_script() {
		let nodes = parse_js_format("let x = 1 < 2;").unwrap();
		let BsxNode::Element(el) = &nodes[0] else {
			panic!("expected a script element");
		};
		el.tag.clone().xpect_eq("script".to_string());
		// the JS body is one verbatim text child, `<` and all.
		el.children[0]
			.clone()
			.xpect_eq(BsxNode::Text("let x = 1 < 2;".to_string()));
	}

	#[beet_core::test]
	fn empty_js_is_an_empty_script() {
		let nodes = parse_js_format("").unwrap();
		let BsxNode::Element(el) = &nodes[0] else {
			panic!("expected a script element");
		};
		el.children.is_empty().xpect_true();
	}
}
