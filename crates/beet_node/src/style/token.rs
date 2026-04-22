//! Token system based on the
//! [material design spec](https://m3.material.io/foundations/design-tokens/overview),
//! and adapted to fit the rust and bevy type systems
use crate::style::*;
use beet_core::prelude::*;

/// Associates a static CSS type-tag string with a Rust type.
pub trait TypeTag {
	const TYPE_TAG: SmolStr;
}

/// Converts a value to its CSS string representation.
pub trait CssValue {
	fn to_css_value(&self) -> String;
}

/// The unit type acts as an "unset" sentinel in the token system.
impl TypeTag for () {
	const TYPE_TAG: SmolStr = SmolStr::new_static("unset");
}

impl CssValue for () {
	fn to_css_value(&self) -> String { "unset".to_string() }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Get, Reflect)]
pub struct Token {
	/// The namespace for the key, defaults to env!(CARGO_PKG_NAME)
	namespace: SmolStr,
	descriptor: SmolStr,
	type_tag: SmolStr,
}

/// Shorthand for defining style tokens
#[macro_export]
macro_rules! token {
	($kind:ty,$name:ident, $descriptor:expr) => {
		pub const $name: Token = Token::new_static::<$kind>($descriptor);
	};
}

impl Token {
	pub const fn new(
		namespace: SmolStr,
		descriptor: SmolStr,
		type_tag: SmolStr,
	) -> Self {
		Self {
			namespace,
			descriptor,
			type_tag,
		}
	}

	pub const fn new_static<T: TypeTag>(descriptor: &'static str) -> Self {
		Self {
			namespace: SmolStr::new_static(env!("CARGO_PKG_NAME")),
			descriptor: SmolStr::new_static(descriptor),
			type_tag: T::TYPE_TAG,
		}
	}

	pub const fn new_with_namespace<T: TypeTag>(
		namespace: SmolStr,
		descriptor: SmolStr,
	) -> Self {
		Self {
			namespace,
			descriptor,
			type_tag: T::TYPE_TAG,
		}
	}

	pub fn to_css_key(&self) -> String {
		format!("{}-{}-{}", self.namespace, self.type_tag, self.descriptor)
	}
}

/// A style token, defined by its [`Token`], to be wrapped in
/// metadata for docs and tooling.
#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Get, Deref, Reflect,
)]
pub struct TokenMeta {
	/// This token's key
	#[deref]
	key: Token,
	/// Human readable label for the token
	label: SmolStr,
	/// Short description for this token
	description: SmolStr,
}

impl std::fmt::Display for Token {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.to_css_key().fmt(f)
	}
}

impl TokenMeta {
	pub fn new(
		key: Token,
		label: impl Into<SmolStr>,
		description: impl Into<SmolStr>,
	) -> Self {
		Self {
			key,
			label: label.into(),
			description: description.into(),
		}
	}

	pub const fn new_static<T: TypeTag>(
		descriptor: &'static str,
		label: &'static str,
		description: &'static str,
	) -> Self {
		Self {
			key: Token::new_static::<T>(descriptor),
			label: SmolStr::new_static(label),
			description: SmolStr::new_static(description),
		}
	}
}

impl std::fmt::Display for TokenMeta {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.key.fmt(f)
	}
}


#[derive(Debug, Clone, Component, Resource, Deref)]
pub struct TokenStore<T = ()>(HashMap<Token, TokenValue<T>>);

impl<T> Default for TokenStore<T> {
	fn default() -> Self { Self(default()) }
}

impl TokenStore<()> {
	pub fn new() -> Self { Self(HashMap::new()) }
}

impl<T> TokenStore<T> {
	pub fn new_with_map(map: HashMap<Token, TokenValue<T>>) -> Self {
		Self(map)
	}
	pub fn with(
		mut self,
		token: Token,
		value: impl Into<TokenValue<T>>,
	) -> Self {
		self.0.insert(token, value.into());
		self
	}

	pub fn insert(
		&mut self,
		token: Token,
		value: impl Into<TokenValue<T>>,
	) -> Option<TokenValue<T>> {
		self.0.insert(token, value.into())
	}
}

impl<T> Merge for TokenStore<T> {
	fn merge(&mut self, other: Self) {
		for (key, value) in other.0 {
			self.0.insert(key, value);
		}
	}
}

#[derive(Debug, thiserror::Error)]
pub enum ResolveKeyError {
	#[error("Token not found: {0:?}")]
	KeyNotFound(String),
	#[error("Circular reference detected for token: {0:?}")]
	CircularReference(String),
}

/// Maps tokens to other tokens, allowing for high level aliasing,
/// light/dark theming etc.
/// A token map can be a global resource or applied to entities,
/// resolving from global, to root and continuing to the child,
/// overwriting as nessecary.
#[derive(Debug, Clone, Component, Resource, Deref)]
pub struct TokenMap(HashMap<Token, Token>);

impl Default for TokenMap {
	fn default() -> Self { Self::new() }
}

impl TokenMap {
	pub fn new() -> Self { Self(HashMap::new()) }

	pub fn with(mut self, from: Token, to: Token) -> Self {
		self.0.insert(from, to);
		self
	}

	pub fn with_checked(mut self, from: Token, to: Token) -> Result<Self> {
		if self.0.contains_key(&from) {
			bevybail!("Token mapping already exists: {:?}", from.to_css_key());
		}
		self.0.insert(from, to);
		Ok(self)
	}

	pub fn insert(&mut self, from: Token, to: Token) -> Result<()> {
		if self.0.contains_key(&from) {
			bevybail!("Token mapping already exists: {:?}", from.to_css_key());
		}
		self.0.insert(from, to);
		Ok(())
	}

	pub fn get(&self, key: &Token) -> Option<&Token> { self.0.get(key) }

	/// Iterates over all `(from, to)` token mappings.
	pub fn iter(&self) -> impl Iterator<Item = (&Token, &Token)> {
		self.0.iter()
	}
}

impl Merge for TokenMap {
	fn merge(&mut self, other: Self) {
		for (key, value) in other.0 {
			self.0.insert(key, value);
		}
	}
}
