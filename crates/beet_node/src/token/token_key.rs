use crate::prelude::*;
use beet_core::prelude::*;

/// Identifies a token instance.
///
/// Serializes as a single string with a prefix for disambiguation:
/// - `rust:io.crates/bevy_color/color/Color`
/// - `url:http://example.com/color`
/// - `inline:src/file.rs:10:5`
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
pub enum TokenKey {
	/// A Rust module path with `io.crates/` prefix and `/` delimiters,
	/// ie `io.crates/beet_node/style/material/colors/Primary`.
	RustType(SmolStr),
	/// A URL identifier, ie `http://example.com/color`.
	Url(SmolStr),
	/// An inline key at a specific source location (file:line:col).
	Inline(SmolStr),
	// /// uuid reflect impl feature gated behind bevy_scene?
	// #[cfg(feature="bevy_scene")]
	// Uuid(uuid::Uuid),
}

impl Default for TokenKey {
	fn default() -> Self { Self::RustType(SmolStr::default()) }
}

impl TokenKey {
	/// Creates an inline key from the caller's source location.
	#[track_caller]
	pub fn new_inline() -> Self {
		let location = core::panic::Location::caller();
		let key = format!(
			"{}:{}:{}",
			location.file(),
			location.line(),
			location.column()
		);
		Self::Inline(SmolStr::from(key))
	}

	pub fn new(path: impl Into<SmolStr>) -> Self { Self::RustType(path.into()) }

	pub fn assert_eq(&self, expected: &TokenKey) -> Result<&Self> {
		if self == expected {
			self.xok()
		} else {
			bevybail!(
				"Token Key Mismatch\nExpected: `{expected}`\nReceived: `{self}`"
			)
		}
	}

	/// Converts a Rust module path (with `::` delimiters) to a [`TokenKey::RustType`].
	pub fn from_module_path(val: &str) -> Self {
		let val = "io.crates/".xtend(val.replace("::", "/"));
		Self::RustType(val.into())
	}

	pub fn of<T: TypedTokenKey>() -> Self { T::token_key() }

	/// Returns `true` if this is an inline (anonymous) key.
	pub fn is_inline(&self) -> bool { matches!(self, Self::Inline(_)) }

	/// Returns the inner string value without any prefix.
	pub fn as_str(&self) -> &str {
		match self {
			Self::RustType(s) | Self::Url(s) | Self::Inline(s) => s.as_str(),
		}
	}

	/// Serializes with a prefix, ie `rust:io.crates/...`.
	fn to_prefixed_string(&self) -> alloc::string::String {
		match self {
			Self::RustType(s) => format!("rust:{}", s),
			Self::Url(s) => format!("url:{}", s),
			Self::Inline(s) => format!("inline:{}", s),
		}
	}

	/// Parses from a prefixed string produced by [`Self::to_prefixed_string`].
	fn from_prefixed_str(s: &str) -> Result<Self> {
		if let Some(rest) = s.strip_prefix("rust:") {
			Ok(Self::RustType(SmolStr::from(rest)))
		} else if let Some(rest) = s.strip_prefix("url:") {
			Ok(Self::Url(SmolStr::from(rest)))
		} else if let Some(rest) = s.strip_prefix("inline:") {
			Ok(Self::Inline(SmolStr::from(rest)))
		} else {
			bevybail!(
				"Invalid TokenKey format, expected prefix (rust:, url:, inline:): {:?}",
				s
			)
		}
	}
}

pub trait TypedTokenKey {
	fn token_key() -> TokenKey;
}
impl<T: TypePath> TypedTokenKey for T {
	fn token_key() -> TokenKey { TokenKey::from_module_path(T::type_path()) }
}

impl From<TokenKey> for FieldPath {
	fn from(token_path: TokenKey) -> Self {
		FieldPath::new(token_path.to_string().split('/'))
	}
}

impl core::fmt::Display for TokenKey {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::RustType(s) | Self::Url(s) | Self::Inline(s) => s.fmt(f),
		}
	}
}

#[cfg(feature = "serde")]
impl serde::Serialize for TokenKey {
	fn serialize<S: serde::Serializer>(
		&self,
		ser: S,
	) -> core::result::Result<S::Ok, S::Error> {
		ser.serialize_str(&self.to_prefixed_string())
	}
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for TokenKey {
	fn deserialize<D: serde::Deserializer<'de>>(
		des: D,
	) -> core::result::Result<Self, D::Error> {
		struct TokenKeyVisitor;
		impl<'de> serde::de::Visitor<'de> for TokenKeyVisitor {
			type Value = TokenKey;
			fn expecting(
				&self,
				f: &mut core::fmt::Formatter,
			) -> core::fmt::Result {
				write!(
					f,
					"a token key string with prefix (rust:, url:, inline:)"
				)
			}
			fn visit_str<E: serde::de::Error>(
				self,
				v: &str,
			) -> core::result::Result<TokenKey, E> {
				TokenKey::from_prefixed_str(v).map_err(E::custom)
			}
		}
		des.deserialize_str(TokenKeyVisitor)
	}
}


/// Identifies a value type, always derived from a Rust [`TypePath`].
///
/// Stored as a slash-separated module path with `io.crates/` prefix,
/// ie `io.crates/bevy_color/color/Color`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TokenSchema(SmolStr);

impl TokenSchema {
	/// Creates a schema from a Rust [`TypePath`].
	pub fn of<T: TypePath>() -> Self {
		let path = "io.crates/".xtend(T::type_path().replace("::", "/"));
		Self(path.into())
	}

	/// Returns the corresponding [`TokenKey`] for map lookups.
	pub fn as_token_key(&self) -> TokenKey {
		TokenKey::RustType(self.0.clone())
	}

	pub fn assert_eq(&self, other: &TokenSchema) -> Result<&Self> {
		if self == other {
			self.xok()
		} else {
			bevybail!(
				"Token Schema Mismatch\nExpected: `{other}`\nReceived: `{self}`"
			)
		}
	}

	pub fn assert_eq_ty<T: TypePath>(&self) -> Result<&Self> {
		self.assert_eq(&Self::of::<T>())
	}
}

impl core::fmt::Display for TokenSchema {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		self.0.fmt(f)
	}
}
