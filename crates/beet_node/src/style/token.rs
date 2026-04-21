//! Token system based based on the
//! [material design spec](https://m3.material.io/foundations/design-tokens/overview),
//! and adapted to fit the rust and bevy type systems
use beet_core::prelude::*;
use std::hash::Hasher;

#[derive(Reflect)]
pub struct Token<T> {
	/// The namespace for the key, defaults to env!(CARGO_PKG_NAME)
	namespace: SmolStr,
	descriptor: SmolStr,
	#[reflect(ignore)]
	phantom: PhantomData<T>,
}

/// Shorthand for defining style token metadata
#[macro_export]
macro_rules! token {
	($kind:ident,$name:ident, $meta_name:ident, $descriptor:expr, $label:expr, $description:expr) => {
		pub const $name: Token<$kind> = Token::new_static($descriptor);
		pub const $meta_name: TokenMeta<$kind> =
			TokenMeta::new_static($descriptor, $label, $description);
	};
}

impl<T: AsTokenValue> std::fmt::Debug for Token<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Property")
			.field("namespace", &self.namespace)
			.field("descriptor", &self.descriptor)
			.field("category", &T::category())
			.finish()
	}
}

impl<T> PartialEq for Token<T> {
	fn eq(&self, other: &Self) -> bool {
		self.namespace == other.namespace && self.descriptor == other.descriptor
	}
}

impl<T> Clone for Token<T> {
	fn clone(&self) -> Self {
		Self {
			namespace: self.namespace.clone(),
			descriptor: self.descriptor.clone(),
			phantom: PhantomData,
		}
	}
}

impl<T> Eq for Token<T> {}
impl<T> PartialOrd for Token<T> {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}
impl<T> Ord for Token<T> {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.namespace
			.cmp(&other.namespace)
			.then_with(|| self.descriptor.cmp(&other.descriptor))
	}
}

impl<T: AsTokenValue> std::hash::Hash for Token<T> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.namespace.hash(state);
		T::category().hash(state);
		self.descriptor.hash(state);
	}
}

impl<T: AsTokenValue> Token<T> {
	pub const fn new(descriptor: SmolStr) -> Self {
		Self {
			namespace: SmolStr::new_static(env!("CARGO_PKG_NAME")),
			descriptor,
			phantom: PhantomData,
		}
	}
	pub const fn new_static(descriptor: &'static str) -> Self {
		Self {
			namespace: SmolStr::new_static(env!("CARGO_PKG_NAME")),
			descriptor: SmolStr::new_static(descriptor),
			phantom: PhantomData,
		}
	}

	pub const fn new_with_namespace(
		namespace: SmolStr,
		descriptor: SmolStr,
	) -> Self {
		Self {
			namespace,
			descriptor,
			phantom: PhantomData,
		}
	}

	pub fn to_css_key(&self) -> String {
		format!("{}-{}-{}", self.namespace, T::category(), self.descriptor)
	}
}

impl<T: AsTokenValue> std::fmt::Display for Token<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.to_css_key().fmt(f)
	}
}


/// A style token, defined by its [`TokenKey`], to be wrapped in
/// newtype components like `ForegroundColor(Property<Color>)`.
/// There must only be a single definition per key used in a
/// single application, PartialEq, PartialOrd, Hash etc
/// only use the `key` field.
#[derive(Get, Reflect, Deref)]
pub struct TokenMeta<T> {
	/// This token's key
	#[deref]
	key: Token<T>,
	/// Human readable label for the token
	label: SmolStr,
	/// Short description for this token
	description: SmolStr,
	#[reflect(ignore)]
	phantom: PhantomData<T>,
}

impl<T: AsTokenValue> std::fmt::Debug for TokenMeta<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Token")
			.field("key", &self.key)
			.field("label", &self.label)
			.field("description", &self.description)
			.finish()
	}
}
impl<T: AsTokenValue> std::fmt::Display for TokenMeta<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.key.fmt(f)
	}
}

impl<T> Clone for TokenMeta<T> {
	fn clone(&self) -> Self {
		Self {
			label: self.label.clone(),
			description: self.description.clone(),
			key: self.key.clone(),
			phantom: std::marker::PhantomData,
		}
	}
}

impl<T> PartialEq for TokenMeta<T> {
	fn eq(&self, other: &Self) -> bool { self.key == other.key }
}

impl<T> Eq for TokenMeta<T> {}

impl<T> PartialOrd for TokenMeta<T> {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl<T> Ord for TokenMeta<T> {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.key.cmp(&other.key)
	}
}

impl<T: AsTokenValue> std::hash::Hash for TokenMeta<T> {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.key.hash(state);
	}
}

impl<T: AsTokenValue> TokenMeta<T> {
	pub fn new(
		key: Token<T>,
		label: impl Into<SmolStr>,
		description: impl Into<SmolStr>,
	) -> Self {
		Self {
			key,
			label: label.into(),
			description: description.into(),
			phantom: PhantomData,
		}
	}

	pub const fn new_static(
		descriptor: &'static str,
		label: &'static str,
		description: &'static str,
	) -> Self {
		Self {
			key: Token::new_static(descriptor),
			label: SmolStr::new_static(label),
			description: SmolStr::new_static(description),
			phantom: PhantomData,
		}
	}
}

pub enum TokenValue<T> {
	Value(T),
	Ref(Token<T>),
}

impl<T> TokenValue<T> {
	pub fn as_value(&self) -> Option<&T> {
		match self {
			TokenValue::Value(value) => Some(value),
			_ => None,
		}
	}
	pub fn as_ref(&self) -> Option<&Token<T>> {
		match self {
			TokenValue::Ref(token) => Some(token),
			_ => None,
		}
	}
}

impl<T> From<T> for TokenValue<T> {
	fn from(value: T) -> Self { Self::Value(value) }
}
impl<T> From<Token<T>> for TokenValue<T> {
	fn from(token: Token<T>) -> Self { Self::Ref(token) }
}
/// Allows for second order into token value,
/// ie &'static str into SmolStr or SrgbColor into Color
pub trait IntoTokenValue<T, M> {
	fn into_token_value(self) -> TokenValue<T>;
}

pub struct ValueIntoTokenValueMarker;
impl<T, U: Into<T>> IntoTokenValue<T, ValueIntoTokenValueMarker> for U {
	fn into_token_value(self) -> TokenValue<T> {
		TokenValue::Value(self.into())
	}
}

impl<T> IntoTokenValue<T, Self> for Token<T> {
	fn into_token_value(self) -> TokenValue<T> { TokenValue::Ref(self) }
}
pub trait AsTokenValue {
	fn category() -> &'static str;
	fn to_css_value(&self) -> String;
}

pub struct TokenStore<T>(HashMap<Token<T>, TokenValue<T>>);

impl<T> Default for TokenStore<T> {
	fn default() -> Self { Self::new() }
}

impl<T> TokenStore<T> {
	pub fn new() -> Self { Self(HashMap::new()) }
}

impl<T: AsTokenValue> TokenStore<T> {
	pub fn insert<M>(
		&mut self,
		token: Token<T>,
		value: impl IntoTokenValue<T, M>,
	) -> Result<()> {
		if self.0.contains_key(&token) {
			bevybail!("Token key already exists: {:?}", token.to_css_key());
		}
		self.0.insert(token, value.into_token_value());
		Ok(())
	}

	pub fn get(&self, key: &Token<T>) -> Option<&TokenValue<T>> {
		self.0.get(key)
	}

	pub fn resolve(&self, key: &Token<T>) -> Result<&T, ResolveKeyError> {
		let mut visited = HashSet::new();
		let mut current_key = key;

		loop {
			if visited.contains(current_key) {
				return Err(ResolveKeyError::CircularReference(
					current_key.to_css_key(),
				));
			}
			visited.insert(current_key);

			match self.get(current_key) {
				Some(TokenValue::Value(value)) => return Ok(value),
				Some(TokenValue::Ref(ref_key)) => current_key = ref_key,
				None => {
					return Err(ResolveKeyError::KeyNotFound(
						current_key.to_css_key(),
					));
				}
			}
		}
	}
}


#[derive(Debug, thiserror::Error)]
pub enum ResolveKeyError {
	#[error("Token key not found: {0:?}")]
	KeyNotFound(String),
	#[error(
		"Token value is a reference, but the referenced key was not found: {0:?}"
	)]
	ReferenceNotFound(String),
	#[error("Circular reference detected for token key: {0:?}")]
	CircularReference(String),
}


#[cfg(test)]
mod tests {
	use super::*;
	use crate::style::color::*;

	#[test]
	fn token_store() {
		let mut store = TokenStore::<Color>::default();

		store
			.insert(PRIMARY_BACKGROUND, palettes::basic::RED)
			.unwrap();
		store.insert(SURFACE_TINT, PRIMARY_BACKGROUND).unwrap();
		store
			.get(&PRIMARY_BACKGROUND)
			.unwrap()
			.as_value()
			.unwrap()
			.to_srgba()
			.xpect_eq(palettes::basic::RED);
		store
			.resolve(&PRIMARY_BACKGROUND)
			.unwrap()
			.to_srgba()
			.xpect_eq(palettes::basic::RED);
		store
			.resolve(&SURFACE_TINT)
			.unwrap()
			.to_srgba()
			.xpect_eq(palettes::basic::RED);
		store.resolve(&PRIMARY_FOREGROUND).unwrap_err();
	}
}
