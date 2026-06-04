use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;
use std::sync::Arc;

/// A set of declarations applied to elements matching the given selector.
///
/// An optional [`media`](Self::media) query gates the rule behind an `@media`
/// at-rule: such rules serialize to CSS wrapped in `@media (…) { … }`, and the
/// non-web cascade ([`RuleSet::cascade`](crate::prelude::RuleSet)) ignores them
/// since there is no target media context for charcell/native yet.
#[derive(Debug, Default, Clone, Reflect, Get, GetMut, SetWith)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Rule {
	/// Predicate for which entities this rule applies to
	selector: Selector,
	declarations: HashMap<TokenKey, TokenValue>,
	/// Optional `@media` gate; `None` means the rule always applies.
	#[set_with(unwrap_option)]
	media: Option<MediaQuery>,
}

/// An `@media` at-rule gate for a [`Rule`].
///
/// Only the media features beet needs today are modelled; the variants
/// serialize to the CSS condition inside `@media …`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum MediaQuery {
	/// `@media print` — applies when printing.
	Print,
	/// `@media (prefers-reduced-motion: reduce)`.
	ReducedMotion,
}

impl MediaQuery {
	/// The CSS condition placed after `@media`.
	pub fn as_css(&self) -> &'static str {
		match self {
			MediaQuery::Print => "print",
			MediaQuery::ReducedMotion => "(prefers-reduced-motion: reduce)",
		}
	}
}

impl Rule {
	pub fn new() -> Self { Self::default() }

	/// Create a rule with a class selector, eg `Rule::class("btn-filled")`.
	pub fn class(class: &str) -> Self {
		Self {
			selector: Selector::Class(class.into()),
			declarations: default(),
			media: None,
		}
	}

	/// Create a rule with a tag selector, eg `Rule::tag("button")`.
	pub fn tag(tag: &str) -> Self {
		Self {
			selector: Selector::Tag(tag.into()),
			declarations: default(),
			media: None,
		}
	}

	/// Create a rule matching any of the given tags, eg
	/// `Rule::tags(&["strong", "b"])`. A single tag yields a plain
	/// [`Selector::Tag`]; multiple tags an [`Selector::AnyOf`].
	pub fn tags(tags: &[&str]) -> Self {
		let selector = match tags {
			[tag] => Selector::tag(*tag),
			_ => Selector::AnyOf(
				tags.iter().map(|t| Selector::tag(*t)).collect(),
			),
		};
		Self {
			selector,
			declarations: default(),
			media: None,
		}
	}

	pub fn insert(
		&mut self,
		key: impl Into<Token>,
		value: impl Into<TokenValue>,
	) -> Result<&mut Self> {
		let value = value.into();
		let key = key.into();
		key.schema().assert_eq(value.schema())?;
		self.declarations.insert(key.key().clone(), value);
		self.xok()
	}

	fn with(
		mut self,
		key: impl Into<Token>,
		value: impl Into<TokenValue>,
	) -> Result<Self> {
		self.insert(key, value)?;
		self.xok()
	}

	pub fn with_token(
		self,
		key: impl Into<Token>,
		value: impl Into<Token>,
	) -> Result<Self> {
		self.with(key, value)
	}

	#[cfg(feature = "serde")]
	pub fn with_value<T>(self, key: T, value: impl Into<T::Value>) -> Self
	where
		T: TypedToken + Into<Token>,
		T::Value: Typed + Serialize,
	{
		self.with(
			key,
			TypedValue::new(value.into())
				.expect("failed to serialize typed value"),
		)
		.expect(
			"Schema assertion failed for a typed value, this shouldnt be possible",
		)
	}
	/// Set a value whose property token is inferred from the value's type via
	/// its [`CanonicalToken`], eg `Rule::new().with_canonical(Display::None)`.
	/// For multi-property values (eg [`Color`]) name the token with
	/// [`with_value`](Self::with_value) instead.
	#[cfg(feature = "serde")]
	pub fn with_canonical<V>(self, value: V) -> Self
	where
		V: CanonicalToken + Typed + Serialize,
	{
		self.with_value(V::Token::default(), value)
	}

	#[cfg(feature = "serde")]
	pub fn with_value_untyped(
		self,
		key: impl Into<Token>,
		value: impl Typed + Serialize,
	) -> Result<Self> {
		self.with(key, TypedValue::new(value)?)
	}

	#[cfg(feature = "serde")]
	#[track_caller]
	pub fn with_inline_value<T>(self, value: T) -> Result<Self>
	where
		T: Typed + Serialize,
	{
		let key = Token::new_inline(FieldSchema::of::<T>());
		self.with(key, TypedValue::new(value)?)
	}

	/// Extend declarations from an iterator of `(TokenKey, TokenValue)` pairs.
	pub fn with_extend(
		mut self,
		iter: impl IntoIterator<Item = (TokenKey, TokenValue)>,
	) -> Self {
		self.declarations.extend(iter);
		self
	}

	/// Get a declaration value by token, performing schema validation.
	pub fn get(&self, token: &Token) -> Result<&TokenValue> {
		match self.declarations.get(token.key()) {
			Some(value) => {
				token.schema().assert_eq(value.schema())?;
				Ok(value)
			}
			None => bevybail!("Token Not Found: `{token}`"),
		}
	}

	pub fn get_mut(&mut self, token: &Token) -> Result<&mut TokenValue> {
		match self.declarations.get_mut(token.key()) {
			Some(value) => {
				token.schema().assert_eq(value.schema())?;
				Ok(value)
			}
			None => bevybail!("Token Not Found: `{token}`"),
		}
	}

	pub fn contains_key(&self, key: &TokenKey) -> bool {
		self.declarations.contains_key(key)
	}

	/// Iterate over all declarations.
	pub fn iter(&self) -> impl Iterator<Item = (&TokenKey, &TokenValue)> {
		self.declarations.iter()
	}

	/// Iterate mutably over all declarations.
	pub fn iter_mut(
		&mut self,
	) -> impl Iterator<Item = (&TokenKey, &mut TokenValue)> {
		self.declarations.iter_mut()
	}

	/// Merge another rule's declarations into this one (builder pattern).
	pub fn extend_declarations(mut self, other: Self) -> Self {
		self.declarations.extend(other.declarations);
		self
	}

	/// Mutable: merge another rule's declarations into self.
	pub fn push_declarations(&mut self, other: Self) -> &mut Self {
		self.declarations.extend(other.declarations);
		self
	}

	/// Get a typed value, performing schema and type validation.
	#[cfg(feature = "serde")]
	pub fn get_typed<T: Typed + serde::de::DeserializeOwned>(
		&self,
		key: &Token,
	) -> Result<T> {
		key.schema().assert_eq_ty::<T>()?;
		match self.get(key)? {
			TokenValue::Value(value) => value.into_typed::<T>(),
			TokenValue::Token(_) => {
				bevybail!("Expected Value, found Token: `{key}`")
			}
		}
	}

	pub fn merge_any(mut self, other: Self) -> Self {
		self.selector = self.selector.clone().merge_any(other.selector);
		self.declarations.extend(other.declarations);
		self
	}

	/// Matches all rules, or `true` if empty
	pub fn matches(&self, el: &ElementView) -> bool {
		self.selector.matches(el)
	}
}

impl IntoIterator for Rule {
	type Item = (TokenKey, TokenValue);
	type IntoIter =
		bevy::platform::collections::hash_map::IntoIter<TokenKey, TokenValue>;
	fn into_iter(self) -> Self::IntoIter { self.declarations.into_iter() }
}

// akin to a lightningcss Component, ie `/selectors/parser.rs#1392`
/// A match rule
#[derive(
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Selector {
	/// A global predicate, in css this will evaluate to `:root`,
	/// and in bevy apps will always pass predicates
	#[default]
	Root,
	/// Only match a specific entity
	Entity(Entity),
	/// Selects any element, in css this will evaluate to `*`,
	/// and in bevy apps will always pass predicates
	Any,
	/// Match any of the rules, ie in css `div, .my-class` (note the comma) in css
	AnyOf(Vec<Selector>),
	/// Match all of the rules, ie in css `div.my-class` (note no comma) in css
	AllOf(Vec<Selector>),
	/// Must have this tag, ie in css `div`
	Tag(SmolStr),
	/// Must have this class, ie in css `.my-class`
	Class(SmolStr),
	/// Must be in this state, ie in css `:hover`
	State(ElementState),
	/// Must have the attribute, ie 'display=flex'
	Attribute {
		key: SmolStr,
		/// Optionally also
		value: Option<Value>,
	},
	/// Negate a rule, ie must not have tag
	Not(Arc<Self>),
}

impl Selector {
	pub fn class(class: impl Into<ClassName>) -> Self {
		Self::Class(class.into().as_selector())
	}
	pub fn tag(tag: impl Into<SmolStr>) -> Self { Self::Tag(tag.into()) }
	pub fn state(state: ElementState) -> Self { Self::State(state) }

	pub fn attribute(key: impl Into<SmolStr>, value: Option<Value>) -> Self {
		Self::Attribute {
			key: key.into(),
			value,
		}
	}
	pub fn not(inner: Selector) -> Self { Self::Not(Arc::new(inner)) }

	/// Merge two selectors as an AnyOf, collapsing global selectors
	pub fn merge_any(self, other: Self) -> Self {
		if self == other {
			return self;
		}
		match (self, other) {
			(Self::Any, _) | (_, Self::Any) => Self::Any,
			(Self::AnyOf(mut rules), Self::AnyOf(other)) => {
				rules.extend(other);
				Self::AnyOf(rules)
			}
			(Self::AnyOf(mut rules), r) | (r, Self::AnyOf(mut rules)) => {
				rules.push(r);
				Self::AnyOf(rules)
			}
			(r1, r2) => Self::AnyOf(vec![r1, r2]),
		}
	}

	/// CSS-like cascade weight: a more specific selector wins regardless of
	/// insertion order. Class/state/attribute selectors outweigh tag selectors,
	/// which outweigh the universal/root match, so a `.container` rule beats a
	/// bare `div` rule even when the tag rule was registered first.
	pub fn specificity(&self) -> u32 {
		match self {
			Selector::Root | Selector::Any => 0,
			Selector::Tag(_) => 1,
			Selector::Class(_)
			| Selector::State(_)
			| Selector::Attribute { .. } => 10,
			Selector::Entity(_) => 100,
			Selector::Not(inner) => inner.specificity(),
			// a compound `div.btn` sums its parts; a `div, .btn` group takes the
			// strongest branch, mirroring CSS.
			Selector::AllOf(parts) => {
				parts.iter().map(Selector::specificity).sum()
			}
			Selector::AnyOf(parts) => {
				parts.iter().map(Selector::specificity).max().unwrap_or(0)
			}
		}
	}

	pub fn matches(&self, el: &ElementView) -> bool {
		match self {
			Selector::Root => true,
			Selector::Any => true,
			Selector::Entity(entity) => el.entity == *entity,
			Selector::AnyOf(rules) => rules.iter().any(|rule| rule.matches(el)),
			Selector::AllOf(rules) => rules.iter().all(|rule| rule.matches(el)),
			Selector::Tag(tag) => el.element.tag() == tag,
			Selector::Attribute { key, value } => match value {
				Some(expected) => el
					.attribute(key)
					.map(|attr| attr.value == expected)
					.unwrap_or(false),
				None => el.attribute(key).is_some(),
			},
			Selector::State(state) => el.contains_state(state),
			Selector::Class(class) => el.contains_class(class),
			Selector::Not(inner) => !inner.matches(el),
		}
	}
}
