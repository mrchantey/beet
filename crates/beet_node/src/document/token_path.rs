use crate::prelude::*;
use beet_core::exports::Itertools;
use beet_core::prelude::*;

/// A slash seperated stable type path, prefixed by
/// the origin domain in reverse domain name notation.
/// Rust types use the module path, ie:
/// `io.crates/beet_node/style/material/colors/OnPrimary`
#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Deref,
	DerefMut,
	Reflect,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TokenPath(SmolStr);

impl TokenPath {
	pub fn new(path: impl Into<SmolStr>) -> Self { Self(path.into()) }

	/// Splits by double colons `::`
	pub fn from_module_path(val: &str) -> Self {
		let val = "io.crates/".xtend(val.replace("::", "/"));
		Self(val.into())
	}

	pub fn of<T: TypePath>() -> Self { Self::from_module_path(T::type_path()) }
}

impl From<TokenPath> for FieldPath {
	fn from(token_path: TokenPath) -> Self {
		FieldPath::new(token_path.0.split('/'))
	}
}

impl std::fmt::Display for TokenPath {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.fmt(f)
	}
}

impl From<FieldPath> for TokenPath {
	fn from(field_path: FieldPath) -> Self {
		Self(
			field_path
				.into_inner()
				.into_iter()
				.map(|seg| seg.to_string())
				.join("/")
				.into(),
		)
	}
}
