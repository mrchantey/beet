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
	($kind:ident,$name:ident, $descriptor:expr) => {
		pub const $name: Token<$kind> = Token::new_static($descriptor);
	};
}

impl<T> std::fmt::Debug for Token<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Property")
			.field("namespace", &self.namespace)
			.field("descriptor", &self.descriptor)
			.field("category", &Token::<T>::category())
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

impl<T> std::hash::Hash for Token<T> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.namespace.hash(state);
		Token::<T>::category().hash(state);
		self.descriptor.hash(state);
	}
}

impl<T> Token<T> {
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

	pub fn category() -> String {
		ShortName::of::<T>().to_string().to_lowercase()
	}

	pub fn to_css_key(&self) -> String {
		format!(
			"{}-{}-{}",
			self.namespace,
			Self::category(),
			self.descriptor
		)
	}
}

impl<T> std::fmt::Display for Token<T> {
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

impl<T> std::fmt::Debug for TokenMeta<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Token")
			.field("key", &self.key)
			.field("label", &self.label)
			.field("description", &self.description)
			.finish()
	}
}
impl<T> std::fmt::Display for TokenMeta<T> {
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

impl<T> std::hash::Hash for TokenMeta<T> {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.key.hash(state);
	}
}

impl<T> TokenMeta<T> {
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

pub trait CssToken {
	fn to_css_value(&self) -> String;
}
#[derive(Debug, Resource, Deref)]
pub struct TokenStore<T>(HashMap<Token<T>, T>);

impl<T> Default for TokenStore<T> {
	fn default() -> Self { Self::new() }
}

impl<T> TokenStore<T> {
	pub fn new() -> Self { Self(HashMap::new()) }
	pub fn with(mut self, token: Token<T>, value: impl Into<T>) -> Self {
		self.0.insert(token, value.into());
		self
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
/// Token maps are applied from root to child entity when resolving
/// properties
#[derive(Debug, Clone, Component)]
pub struct TokenMap<T>(HashMap<Token<T>, Token<T>>);

impl<T> Default for TokenMap<T> {
	fn default() -> Self { Self::new() }
}
impl<T> TokenMap<T> {
	pub fn new() -> Self { Self(HashMap::new()) }

	pub fn with(mut self, from: Token<T>, to: Token<T>) -> Self {
		self.0.insert(from, to);
		self
	}
	pub fn with_checked(
		mut self,
		from: Token<T>,
		to: Token<T>,
	) -> Result<Self> {
		if self.0.contains_key(&from) {
			bevybail!("Token mapping already exists: {:?}", from.to_css_key());
		}
		self.0.insert(from, to);
		Ok(self)
	}

	pub fn insert(&mut self, from: Token<T>, to: Token<T>) -> Result<()> {
		if self.0.contains_key(&from) {
			bevybail!("Token mapping already exists: {:?}", from.to_css_key());
		}
		self.0.insert(from, to);
		Ok(())
	}

	pub fn get(&self, key: &Token<T>) -> Option<&Token<T>> { self.0.get(key) }
}

impl<T> Merge for TokenMap<T> {
	fn merge(&mut self, other: Self) -> Result {
		for (key, value) in other.0 {
			self.0.insert(key, value);
		}
		Ok(())
	}
}
