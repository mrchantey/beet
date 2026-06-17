//! The basic markup-node data types shared by every front-end (the `rsx!`
//! macro, the BSX parser, serde) and read by the renderers.
//!
//! An [`Element`] is a single XML node (`div`, `span`, …); its
//! [`Attribute`](crate::prelude::Attribute)s are related entities. [`Comment`]
//! and [`Doctype`] are the sibling node kinds. These are pure data: rendering
//! them to HTML or charcell lives in `beet_ui`.
use crate::prelude::*;
#[cfg(feature = "tokens")]
use beet_core_macros::ToTokens;

/// A single markup element node, ie `<div>`/`<span>`/`<p>`. Its tag name is the
/// inner string; its attributes are related [`Attribute`] entities and its
/// children are the usual [`Children`].
#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Reflect,
	Component,
)]
#[reflect(Default, Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[component(immutable)]
// `Default` (empty tag) exists so `Element` picks up Bevy's blanket
// `Template`/`FromTemplate` impls (`Clone + Default`), making it usable as a
// scene template via `template_value`. The default is always overwritten.
pub struct Element(SmolStr);

impl Element {
	/// Construct an element with the given tag name.
	pub fn new(name: impl Into<SmolStr>) -> Self { Self(name.into()) }
	/// The tag name of this element, ie `div`, `span`, `p`.
	pub fn tag(&self) -> &str { &self.0 }
	/// Bundle this element with inner text, using [`OnSpawn`] to avoid
	/// clobbering other children.
	pub fn with_inner_text(self, text: &str) -> impl Bundle {
		(self, OnSpawn::insert_child(Value::Str(text.into())))
	}
}


/// Tags whose text content is whitespace-significant, so their children are left
/// verbatim (the cascade reads `white-space: pre` on these). The single shared
/// list for every whitespace-normalisation pass (the BSX parser, the markdown
/// tree builder).
pub const PRE_ELEMENTS: &[&str] = &["pre", "textarea", "script", "style"];

/// A comment node. The inner string is the comment content excluding the
/// `<!--` and `-->` delimiters.
#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Deref,
	Reflect,
	Component,
)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[component(immutable)]
pub struct Comment(pub String);

impl Comment {
	/// Construct a comment from its (delimiter-free) content.
	pub fn new(content: impl Into<String>) -> Self { Self(content.into()) }
}

/// A doctype declaration. The inner string is the doctype value, usually
/// `"html"` for `<!DOCTYPE html>`.
#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Deref,
	Reflect,
	Component,
)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[component(immutable)]
pub struct Doctype(pub SmolStr);

impl Doctype {
	/// Construct a doctype from its value (ie `"html"`).
	pub fn new(value: impl Into<SmolStr>) -> Self { Self(value.into()) }
}
