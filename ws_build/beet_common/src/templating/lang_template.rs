use crate::prelude::*;
use std::hash::Hash;

/// A hash of the following elements of a lang template:
/// - The tag of the template
/// - The content of the template, either inline or a file path
/// - The directives of the template
/// The hash used by the [`TemplateDirective::LangTemplate`] directive
/// and also as the key in the [`LangTemplateMap`]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LangContentHash(u64);
impl LangContentHash {
	pub fn new(hash: u64) -> Self { Self(hash) }
	pub fn hash(&self) -> u64 { self.0 }
}

impl std::fmt::Display for LangContentHash {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

#[cfg(feature = "tokens")]
impl RustTokens for LangContentHash {
	fn into_rust_tokens(&self) -> proc_macro2::TokenStream {
		let hash = proc_macro2::Literal::u64_unsuffixed(self.0);
		quote::quote! { LangContentHash::new(#hash) }
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LangTemplate {
	/// The tag of the template, either `script` or `style`
	pub tag: String,
	/// the scope of the style
	/// An incremented unique id for this template, counted up from 0 so suitable
	/// as a short html data attribute.
	pub id: u64,
	pub directives: Vec<TemplateDirectiveEnum>,
	/// The content of the template, either inline or a file path
	pub content: String,
	/// The content hash of the template
	content_hash: LangContentHash,
	/// Each span that references this template,
	/// there may be multiple because we deduplicate identical
	/// templates.
	spans: Vec<FileSpan>,
}

impl LangTemplate {
	/// Create a new [`NodeMeta`] for this template, using the first span or default.
	pub fn create_meta(&self) -> NodeMeta {
		match self.spans.first() {
			Some(span) => NodeMeta::new(span.clone(), self.directives.clone()),
			None => NodeMeta::new(FileSpan::default(), self.directives.clone()),
		}
	}
	pub fn spans(&self) -> &[FileSpan] { &self.spans }

	pub fn new(
		tag: String,
		id: u64,
		directives: Vec<TemplateDirectiveEnum>,
		content: String,
		content_hash: LangContentHash,
		spans: Vec<FileSpan>,
	) -> Self {
		Self {
			tag,
			directives,
			content,
			content_hash,
			spans,
			id,
		}
	}
	pub fn push_span(&mut self, span: FileSpan) { self.spans.push(span); }
}
