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
/// let page = fwd_path!("routes/blog/index.md");
/// DocPath::parse("./Card").unwrap()
/// 	.resolve(&page)
/// 	.unwrap()
/// 	.xpect_eq(fwd_path!("routes/blog/Card"));
/// DocPath::parse("../Card.bsx").unwrap()
/// 	.resolve(&page)
/// 	.unwrap()
/// 	.xpect_eq(fwd_path!("routes/Card.bsx"));
/// DocPath::parse("widgets/Card.bsx").unwrap()
/// 	.resolve(&page)
/// 	.unwrap()
/// 	.xpect_eq(fwd_path!("widgets/Card.bsx"));
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
		let path = Self(SmolStr::new(text));
		// a package absolute reference is a path within the package
		if !path.is_relative() {
			path.fwd_path()?;
		}
		path.xok()
	}

	/// Whether the reference is relative to the document holding it.
	pub fn is_relative(&self) -> bool {
		matches!(self.0.split('/').next(), Some("." | ".."))
	}

	/// The package absolute path this names, held by the document at
	/// `document`. The extension stays as authored.
	///
	/// # Errors
	/// Errors when a relative reference climbs above the package.
	pub fn resolve(&self, document: &FwdPath) -> Result<FwdPath> {
		match self.is_relative() {
			true => document
				.parent()
				.unwrap_or_default()
				.as_smol_path()
				.join(&self.0)
				.xmap(FwdPath::new),
			false => self.fwd_path(),
		}
	}

	/// The path as written, package absolute once resolved.
	///
	/// # Errors
	/// Errors when the reference is rooted or climbs out of the package.
	pub fn fwd_path(&self) -> Result<FwdPath> { FwdPath::new(&self.0) }
}

/// A package absolute path as the reference that names it.
impl From<FwdPath> for DocPath {
	fn from(path: FwdPath) -> Self { Self(path.into()) }
}

impl From<&FwdPath> for DocPath {
	fn from(path: &FwdPath) -> Self { path.clone().into() }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn resolves_against_its_document() {
		let resolve = |path: &str, document: &str| {
			DocPath::parse(path)
				.unwrap()
				.resolve(&FwdPath::new(document).unwrap())
				.unwrap()
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
		DocPath::parse("/Card.bsx").unwrap_err();
		DocPath::parse("../../Card")
			.unwrap()
			.resolve(&fwd_path!("a/Page.bsx"))
			.unwrap_err();
	}
}
