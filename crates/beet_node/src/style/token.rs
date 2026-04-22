//! Token system based on the
//! [material design spec](https://m3.material.io/foundations/design-tokens/overview),
//! and adapted to fit the rust and bevy type systems
use beet_core::prelude::*;

use crate::style::AlignItems;
use crate::style::AlignSelf;
use crate::style::Direction;
use crate::style::FlexSize;
use crate::style::JustifyContent;
use crate::style::Unit;

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

/// Shorthand for defining style token metadata
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

/// A concrete style value carried by a [`TokenStore`].
///
/// The `T` parameter allows custom extension values via `Custom(T)`.
/// The unit type `()` acts as the default "unset" custom variant.
#[derive(Debug, Clone, PartialEq, Reflect)]
pub enum TokenValue<T = ()> {
	Color(Color),
	Unit(Unit),
	JustifyContent(JustifyContent),
	AlignItems(AlignItems),
	AlignSelf(AlignSelf),
	FlexSize(FlexSize),
	Direction(Direction),
	Custom(T),
}

impl<T: TypeTag> TokenValue<T> {
	/// Returns the CSS type-tag for the currently stored variant.
	pub fn type_tag(&self) -> SmolStr {
		match self {
			Self::Color(_) => Color::TYPE_TAG,
			Self::Unit(_) => Unit::TYPE_TAG,
			Self::JustifyContent(_) => JustifyContent::TYPE_TAG,
			Self::AlignItems(_) => AlignItems::TYPE_TAG,
			Self::AlignSelf(_) => AlignSelf::TYPE_TAG,
			Self::FlexSize(_) => FlexSize::TYPE_TAG,
			Self::Direction(_) => Direction::TYPE_TAG,
			Self::Custom(_) => T::TYPE_TAG,
		}
	}
}

/// The type tag delegates to the custom type `T`, representing the
/// "default" variant kind for this token value parameterisation.
impl<T: TypeTag> TypeTag for TokenValue<T> {
	const TYPE_TAG: SmolStr = T::TYPE_TAG;
}

impl<T: CssValue> CssValue for TokenValue<T> {
	fn to_css_value(&self) -> String {
		match self {
			Self::Color(v) => v.to_css_value(),
			Self::Unit(v) => v.to_css_value(),
			Self::JustifyContent(v) => v.to_css_value(),
			Self::AlignItems(v) => v.to_css_value(),
			Self::AlignSelf(v) => v.to_css_value(),
			Self::FlexSize(v) => v.to_css_value(),
			Self::Direction(v) => v.to_css_value(),
			Self::Custom(v) => v.to_css_value(),
		}
	}
}

// Generic From impls: the Color/Unit/layout variants don't depend on T.

impl<T> From<Color> for TokenValue<T> {
	fn from(value: Color) -> Self { Self::Color(value) }
}

impl<T> From<Unit> for TokenValue<T> {
	fn from(value: Unit) -> Self { Self::Unit(value) }
}

impl<T> From<JustifyContent> for TokenValue<T> {
	fn from(value: JustifyContent) -> Self { Self::JustifyContent(value) }
}

impl<T> From<AlignItems> for TokenValue<T> {
	fn from(value: AlignItems) -> Self { Self::AlignItems(value) }
}

impl<T> From<AlignSelf> for TokenValue<T> {
	fn from(value: AlignSelf) -> Self { Self::AlignSelf(value) }
}

impl<T> From<FlexSize> for TokenValue<T> {
	fn from(value: FlexSize) -> Self { Self::FlexSize(value) }
}

impl<T> From<Direction> for TokenValue<T> {
	fn from(value: Direction) -> Self { Self::Direction(value) }
}

#[derive(Debug, Clone, Component, Resource, Deref)]
pub struct TokenStore<T = ()>(HashMap<Token, TokenValue<T>>);

impl<T> Default for TokenStore<T> {
	fn default() -> Self { Self::new() }
}

impl<T> TokenStore<T> {
	pub fn new() -> Self { Self(HashMap::new()) }

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
	fn merge(&mut self, other: Self) -> Result {
		for (key, value) in other.0 {
			self.0.insert(key, value);
		}
		Ok(())
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
	fn merge(&mut self, other: Self) -> Result {
		for (key, value) in other.0 {
			self.0.insert(key, value);
		}
		Ok(())
	}
}
