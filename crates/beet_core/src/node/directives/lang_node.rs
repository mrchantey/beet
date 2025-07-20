use crate::as_beet::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use std::hash::Hash;

/// Specify types for variadic functions like TokenizeComponent
pub type LangDirectives = (
	StyleScope,
	StyleCascade,
	ScriptElement,
	StyleElement,
	CodeElement,
	LangSnippetHash,
	StaticLangNode,
	InnerText,
	FileInnerText,
);



/// A hash of all aspects of a lang node that makes it unique,
/// including:
/// - [`NodeTag`]
/// - [`InnerText`]
/// - [`StyleScope`]
/// - [`HtmlHoistDirective`]
/// - [`AttributeKey`]
/// - [`AttributeLit`]
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


/// The deduplicated form of a 'lang node' is a clone of the original,
/// this marker indicates this is the canonical deduplicated version,
/// and should be used to filter for processing so that we arent performing
/// duplicated work on the same node.
#[derive(Debug, Component, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[reflect(Component)]
#[component(immutable)]
pub struct StaticLangNode;


/// The replacement for [`InnerText`] after the lang snippet has been
/// extracted, referencing the path to the snippet scene file.
#[derive(Debug, Clone, PartialEq, Hash, Deref, Component, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[reflect(Component)]
// #[component(immutable)]
pub struct LangSnippetPath(pub WsPathBuf);


#[derive(Debug, Clone, PartialEq, Hash, Component, Reflect)]
#[reflect(Component)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct ScriptElement;

#[derive(Debug, Clone, PartialEq, Hash, Component, Reflect)]
#[reflect(Component)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct StyleElement;

#[derive(Debug, Clone, PartialEq, Hash, Component, Reflect)]
#[reflect(Component)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct CodeElement;


/// Convenience component equivelent to `children![TextNode]`, often used by elements
/// like `script`,`style` or `code` which require further processing.
#[derive(Debug, Default, Clone, PartialEq, Hash, Component, Reflect)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct InnerText(pub String);

/// An intermediate representation of an [`InnerText`] defined by a `src` attribute,
/// ie `<style src="style.css">`.
/// Upon tokenization this is replaced with an include_str,
/// ie [`InnerText(include_str!("style.css"))`],
/// feature gated behind a  [`not(feature="client")`] to avoid excessivly large
/// client bundles, otherwise inserting a unit type
#[derive(Debug, Clone, PartialEq, Hash, Component, Reflect)]
#[reflect(Component)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FileInnerText(
	/// path to the file relative to the source file,
	pub String,
);


/// if `beet_parse` has the `css` feature FileInnerText will be loaded by the macro
/// and replaced with an `InnerText` containing the css, so this tokenization
/// will not occur.
#[cfg(feature = "tokens")]
impl TokenizeSelf for FileInnerText {
	fn self_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
		let path = &self.0;
		tokens.extend(quote::quote! {{
			#[cfg(not(feature = "client"))]
			{
				InnerText::new(include_str!(#path))
			}
			#[cfg(feature = "client")]
			{
				()
			}
		}});
	}
}

impl InnerText {
	/// Create a new [`InnerText`] from a `String`.
	pub fn new(text: impl Into<String>) -> Self { Self(text.into()) }
}
