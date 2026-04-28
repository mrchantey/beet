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
pub struct TokenKey(SmolStr);

impl TokenKey {
	pub fn new(path: impl Into<SmolStr>) -> Self { Self(path.into()) }

	pub fn assert_eq_ty<Expected: TypedTokenKey>(&self) -> Result<&Self> {
		self.assert_eq(&Expected::token_key())
	}
	pub fn assert_eq(&self, expected: &TokenKey) -> Result<&Self> {
		if self == expected {
			self.xok()
		} else {
			bevybail!(
				"Token Key Mismatch\nExpected: `{expected}`\nReceived: `{self}`"
			)
		}
	}

	/// Splits by double colons `::`
	pub fn from_module_path(val: &str) -> Self {
		let val = "io.crates/".xtend(val.replace("::", "/"));
		Self(val.into())
	}

	pub fn of<T: TypedTokenKey>() -> Self { T::token_key() }
}

pub trait TypedTokenKey {
	fn token_key() -> TokenKey;
}
impl<T: TypePath> TypedTokenKey for T {
	fn token_key() -> TokenKey { TokenKey::from_module_path(T::type_path()) }
}
impl From<TokenKey> for FieldPath {
	fn from(token_path: TokenKey) -> Self {
		FieldPath::new(token_path.0.split('/'))
	}
}

impl std::fmt::Display for TokenKey {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.fmt(f)
	}
}

impl From<FieldPath> for TokenKey {
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
