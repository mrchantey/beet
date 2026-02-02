//! Language-specific node types and directives.
//!
//! This module provides components for language-specific elements like
//! scripts, styles, and code blocks that require special processing.

use crate::prelude::*;
use beet_core::prelude::*;
use std::hash::Hash;

/// Specify types for variadic functions like TokenizeComponent
pub type LangDirectives = (
	StyleScope,
	StyleCascade,
	ScriptElement,
	StyleElement,
	CodeNode,
	LangSnippetHash,
	InnerText,
	FileInnerText,
);



/// A hash of all aspects of a lang node that makes it unique,
/// including:
/// - [`NodeTag`]
/// - [`InnerText`]
/// - [`StyleScope`]
/// - [`HtmlHoistDirective`]
/// This is used for several purposes:
/// - deduplication of lang nodes
/// - assigning unique style ids to css content
/// 	- in this case the hash may be compressed to a shorter alphanumeric string
#[derive(
	Debug, Copy, Clone, PartialEq, Eq, Hash, Deref, Component, Reflect,
)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[component(immutable)]
pub struct LangSnippetHash(pub u64);
impl LangSnippetHash {
	/// Create a new [`LangSnippetHash`] from a `u64`.
	pub fn new(hash: u64) -> Self { Self(hash) }
}
impl std::fmt::Display for LangSnippetHash {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}


/// Marker component for `<script>` elements.
#[derive(Debug, Clone, PartialEq, Hash, Component, Reflect)]
#[reflect(Component)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct ScriptElement;

/// Marker component for `<style>` elements.
#[derive(Debug, Clone, PartialEq, Hash, Component, Reflect)]
#[reflect(Component)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct StyleElement;

/// Convenience component equivelent to `children![TextNode]`, often used by elements
/// like `script`,`style` or `code` which require further processing.
///
/// Adding an [`InnerText`] also adds a [`ElementNode::open`].
#[derive(
	Debug, Default, Clone, PartialEq, Hash, Deref, DerefMut, Component, Reflect,
)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[component(on_add= on_add)]
pub struct InnerText(pub String);

fn on_add(mut world: DeferredWorld, cx: HookContext) {
	world
		.commands()
		.entity(cx.entity)
		.insert(ElementNode::open());
}


/// An intermediate representation of an [`InnerText`] defined by a `src` attribute,
/// ie `<style src="style.css">`.
/// Upon tokenization this is replaced with an include_str,
/// ie [`InnerText(include_str!("style.css"))`],
/// feature gated behind a  [`not(feature="client")`] to avoid excessivly large
/// client bundles.
///
/// This type is also collected in beet_build and manually loaded, so we can
/// live reload the contents of an `include_str!`.
#[derive(Debug, Clone, PartialEq, Hash, Component, Reflect)]
#[reflect(Component)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FileInnerText {
	/// path to the file relative to the source file,
	pub path: String,
}
impl FileInnerText {
	/// Create a new [`FileInnerText`] from a path `String`.
	pub fn new(path: impl Into<String>) -> Self { Self { path: path.into() } }
}


/// A FileInnerText is tokenized directly `include_str!(file)`
///
/// When used in `beet_parse`, any [`FileInnerText`] that requires macro-level parsing
/// will not be tokenized, and instead loaded by `load_file_inner_text`
#[cfg(feature = "tokens")]
impl TokenizeSelf for FileInnerText {
	fn self_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
		let path = &self.path;
		// it would be nice to exclude this in client side but that doesnt work
		// for dynamically added content.
		tokens.extend(quote::quote! {
				InnerText::new(include_str!(#path))
		});
	}
}

impl InnerText {
	/// Create a new [`InnerText`] from a `String`.
	pub fn new(text: impl Into<String>) -> Self { Self(text.into()) }
}
