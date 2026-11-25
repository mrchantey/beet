//! [`DocPath`]: a document's reference to another file of its package.
use crate::prelude::*;

/// A reference to another file, held by any component: relative to the
/// document holding it when it starts with `./` or `../`, package absolute
/// otherwise. The extension may be left off when exactly one file matches.
///
/// The type is the mark: every `DocPath` a document's components hold is
/// rewritten package absolute when the document's body is reflected
/// (`beet_scene`'s `DynamicTemplate::reflect_at`), so a component holding one needs nothing but
/// the type, and what reads it never asks which document it was written in.
///
/// ```
/// # use beet_core::prelude::*;
/// let page = RelPath::new("routes/blog/index.md");
/// DocPath::parse("./Card").unwrap()
/// 	.resolve(&page)
/// 	.xpect_eq(RelPath::new("routes/blog/Card"));
/// DocPath::parse("../Card.bsx").unwrap()
/// 	.resolve(&page)
/// 	.xpect_eq(RelPath::new("routes/Card.bsx"));
/// DocPath::parse("widgets/Card.bsx").unwrap()
/// 	.resolve(&page)
/// 	.xpect_eq(RelPath::new("widgets/Card.bsx"));
/// DocPath::parse("https://example.com/Card.bsx").unwrap_err();
/// ```
#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, StringPrimitive,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "SmolStr", into = "SmolStr"))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct DocPath(SmolStr);

impl DocPath {
	/// Parse a reference, refusing the empty one and a uri, which names no
	/// file of a package.
	pub fn parse(text: &str) -> Result<Self> {
		if text.is_empty() {
			bevybail!("a document reference is never empty");
		}
		if text.contains("://") {
			bevybail!(
				"`{text}` is a uri: a document reference names a file of its \
				 package, ie `./Card` or `widgets/Card.bsx`"
			);
		}
		Self(SmolStr::new(text)).xok()
	}

	/// Whether the reference is relative to the document holding it.
	pub fn is_relative(&self) -> bool {
		matches!(self.0.split('/').next(), Some("." | ".."))
	}

	/// The package absolute path this names, held by the document at
	/// `document`. The extension stays as authored.
	pub fn resolve(&self, document: &RelPath) -> RelPath {
		match self.is_relative() {
			true => document.parent().unwrap_or_default().join(&self.0),
			false => self.rel_path(),
		}
	}

	/// The path as written, package absolute once resolved.
	pub fn rel_path(&self) -> RelPath { RelPath::new(&self.0) }
}

/// A package absolute path as the reference that names it.
impl From<RelPath> for DocPath {
	fn from(path: RelPath) -> Self { Self(path.into()) }
}

impl From<&RelPath> for DocPath {
	fn from(path: &RelPath) -> Self { path.clone().into() }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn resolves_against_its_document() {
		let resolve = |path: &str, document: &str| {
			DocPath::parse(path)
				.unwrap()
				.resolve(&RelPath::new(document))
				.to_string()
		};
		resolve("./Card", "Page.bsx").xpect_eq("Card");
		resolve("./Card", "a/b/Page.bsx").xpect_eq("a/b/Card");
		resolve("../Card", "a/b/Page.bsx").xpect_eq("a/Card");
		resolve("a/Card", "x/Page.bsx").xpect_eq("a/Card");
		// a dotted file name is no relative prefix
		resolve(".well-known/Card", "x/Page.bsx").xpect_eq(".well-known/Card");
	}

	#[crate::test]
	fn refuses_what_names_no_file() {
		DocPath::parse("").unwrap_err();
		DocPath::parse("s3://bucket/Card.bsx").unwrap_err();
	}
}
