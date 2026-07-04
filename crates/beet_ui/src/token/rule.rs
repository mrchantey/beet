use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;
use std::sync::Arc;

/// A set of declarations applied to elements matching the given selector.
///
/// An optional [`media`](Self::media) query gates the rule behind an `@media`
/// at-rule: such rules serialize to CSS wrapped in `@media (…) { … }`, and the
/// charcell cascade applies them per [`MediaQuery::applies_in_terminal`]:
/// `Terminal` rules always, width-gated rules against the surface's
/// [`MediaViewport`], web-only media never.
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
	/// `@media screen` — applies on screen, ie the web.
	///
	/// Prefer [`Terminal`](Self::Terminal) for terminal-only styling: a `Screen`
	/// rule is dropped in print contexts, so gating layout behind it breaks
	/// printed output. `Screen` is for the rare rule that is genuinely
	/// screen-only (eg sticky positioning).
	Screen,
	/// Terminal/char-cell only. This is the inverse of the other queries: it has
	/// no CSS equivalent, so it is *excluded* from the serialized stylesheet and
	/// *included* by the charcell cascade. The idiom for terminal-only styling
	/// (eg the colored prose headings) that must not leak into web or print.
	Terminal,
	/// `@media (prefers-reduced-motion: reduce)`.
	ReducedMotion,
	/// `@media (max-width: {0}px)` — applies at or below the given viewport width
	/// in pixels, the idiom for narrow-screen (mobile) overrides. The charcell
	/// cascade evaluates it too, against the surface's [`MediaViewport`], so
	/// one responsive rule drives both targets.
	MaxWidth(u32),
}

impl MediaQuery {
	/// The CSS condition placed after `@media`, or `None` for a query with no
	/// web equivalent ([`Terminal`](Self::Terminal)), which is skipped during
	/// CSS serialization.
	pub fn as_css(&self) -> Option<String> {
		match self {
			MediaQuery::Print => Some("print".into()),
			MediaQuery::Screen => Some("screen".into()),
			MediaQuery::Terminal => None,
			MediaQuery::ReducedMotion => {
				Some("(prefers-reduced-motion: reduce)".into())
			}
			MediaQuery::MaxWidth(px) => Some(format!("(max-width: {px}px)")),
		}
	}

	/// Whether the charcell cascade applies rules gated by this query.
	/// [`Terminal`](Self::Terminal) always applies (the query whose context *is*
	/// that cascade), and [`MaxWidth`](Self::MaxWidth) applies when the surface
	/// `viewport` sits at or below the breakpoint — `None` (no surface, eg a
	/// server world building static HTML) leaves width rules to the browser's
	/// own evaluator. Print/screen/reduced-motion are web media and never apply.
	pub fn applies_in_terminal(self, viewport: Option<MediaViewport>) -> bool {
		match self {
			Self::Terminal => true,
			Self::MaxWidth(px) => {
				viewport.is_some_and(|viewport| viewport.width_px() <= px as f32)
			}
			_ => false,
		}
	}
}

/// The viewport that width-gated media queries ([`MediaQuery::MaxWidth`])
/// evaluate against, in px — the unit breakpoints are authored in. Target
/// agnostic: whichever renderer hosts the tree supplies it, eg the charcell
/// engine mirrors its buffer width (owning the cell→px density), and a
/// windowed target would report logical pixels.
///
/// A *required* component of every render surface (the charcell buffers
/// require it), so it is never absent where a surface exists; the [`Default`]
/// zero width (the narrowest possible) is only ever visible before the
/// surface's first sync, which runs ahead of the cascade in the same frame.
/// Maintained with `set_if_neq`, so `Changed<MediaViewport>` is a true resize
/// signal — paint's per-frame buffer writes never touch it — and
/// `resolve_styles` re-cascades the surface's tree when it fires. Resolved for
/// an element by walking the same Portal-aware parent chain inheritance uses;
/// a world with no surface (eg a server building static HTML) resolves none
/// and skips width-gated rules, leaving them to the browser.
#[derive(Debug, Default, Clone, Copy, PartialEq, Component, Reflect)]
#[reflect(Component)]
pub struct MediaViewport(f32);

impl MediaViewport {
	pub fn new(width_px: f32) -> Self { Self(width_px) }

	/// Viewport width in px, the unit media breakpoints are authored in.
	pub fn width_px(&self) -> f32 { self.0 }
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
		Self {
			selector: Selector::any_tag(tags.iter().copied()),
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

	/// Whether `el` matches this rule's selector; `ancestors` (nearest-first)
	/// supplies the context the combinator selectors need. See
	/// [`Selector::matches`].
	pub fn matches(&self, el: &ElementView, ancestors: &[ElementView]) -> bool {
		self.selector.matches(el, ancestors)
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
	/// Match `descendant` when nested anywhere under an element matching
	/// `ancestor`, ie in css `ancestor descendant` (note the space). Serializes
	/// to CSS for the browser, and the charcell cascade evaluates it against
	/// the real ancestor chain (see [`matches`](Self::matches)).
	Descendant {
		ancestor: Arc<Self>,
		descendant: Arc<Self>,
	},
	/// Match `child` when it is a *direct* child of an element matching
	/// `parent`, ie in css `parent > child` (note the `>`). Like
	/// [`Descendant`](Self::Descendant) but one level only, likewise evaluated
	/// on both targets.
	Child {
		parent: Arc<Self>,
		child: Arc<Self>,
	},
}

impl Selector {
	pub fn class(class: impl Into<ClassName>) -> Self {
		Self::Class(class.into().as_selector())
	}
	pub fn tag(tag: impl Into<SmolStr>) -> Self { Self::Tag(tag.into()) }
	pub fn state(state: ElementState) -> Self { Self::State(state) }

	/// Match any of the given tags, eg `Selector::any_tag(["h1", "h2"])`. A
	/// single tag collapses to a plain [`Tag`](Self::Tag); multiple to an
	/// [`AnyOf`](Self::AnyOf).
	pub fn any_tag(tags: impl IntoIterator<Item: Into<SmolStr>>) -> Self {
		let mut tags = tags.into_iter().map(Self::tag).collect::<Vec<_>>();
		match tags.len() {
			1 => tags.remove(0),
			_ => Self::AnyOf(tags),
		}
	}

	pub fn attribute(key: impl Into<SmolStr>, value: Option<Value>) -> Self {
		Self::Attribute {
			key: key.into(),
			value,
		}
	}
	pub fn not(inner: Selector) -> Self { Self::Not(Arc::new(inner)) }

	/// A descendant combinator, ie css `ancestor descendant`.
	pub fn descendant(ancestor: Selector, descendant: Selector) -> Self {
		Self::Descendant {
			ancestor: Arc::new(ancestor),
			descendant: Arc::new(descendant),
		}
	}

	/// A direct-child combinator, ie css `parent > child`.
	pub fn child(parent: Selector, child: Selector) -> Self {
		Self::Child {
			parent: Arc::new(parent),
			child: Arc::new(child),
		}
	}

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
			// a combinator's weight is the sum of both sides, mirroring CSS.
			Selector::Descendant {
				ancestor,
				descendant,
			} => ancestor.specificity() + descendant.specificity(),
			Selector::Child { parent, child } => {
				parent.specificity() + child.specificity()
			}
		}
	}

	/// Whether `el` matches this selector. `ancestors` is `el`'s ancestor
	/// element views, nearest-first (`ancestors[0]` is the immediate parent),
	/// used to evaluate the [`Child`](Self::Child)/[`Descendant`](Self::Descendant)
	/// combinators. Pass `&[]` when no ancestor context is available (as on the
	/// web, where the browser evaluates combinators); the combinators then never
	/// match. The charcell cascade supplies the real chain so `main > *` resolves
	/// on both targets.
	pub fn matches(&self, el: &ElementView, ancestors: &[ElementView]) -> bool {
		match self {
			Selector::Root => true,
			Selector::Any => true,
			Selector::Entity(entity) => el.entity == *entity,
			Selector::AnyOf(rules) => {
				rules.iter().any(|rule| rule.matches(el, ancestors))
			}
			Selector::AllOf(rules) => {
				rules.iter().all(|rule| rule.matches(el, ancestors))
			}
			Selector::Tag(tag) => el.element.tag() == tag.as_str(),
			Selector::Attribute { key, value } => match value {
				Some(expected) => el
					.attribute(key)
					.map(|attr| attr.value == expected)
					.unwrap_or(false),
				None => el.attribute(key).is_some(),
			},
			Selector::State(state) => el.contains_state(state),
			Selector::Class(class) => el.contains_class(class),
			Selector::Not(inner) => !inner.matches(el, ancestors),
			// `el` matches when `descendant` matches it and `ancestor` matches
			// some element above it (evaluated against that ancestor's own tail).
			Selector::Descendant {
				ancestor,
				descendant,
			} => {
				descendant.matches(el, ancestors)
					&& ancestors.iter().enumerate().any(|(index, view)| {
						ancestor.matches(view, &ancestors[index + 1..])
					})
			}
			// like `Descendant` but the parent must be the *immediate* ancestor.
			Selector::Child { parent, child } => {
				child.matches(el, ancestors)
					&& ancestors.first().is_some_and(|view| {
						parent.matches(view, &ancestors[1..])
					})
			}
		}
	}

	/// Whether this selector, or any nested part, is a combinator
	/// ([`Child`](Self::Child)/[`Descendant`](Self::Descendant)) and so needs
	/// ancestor context to match. Lets the cascade skip building the ancestor
	/// chain for the common combinator-free rule set.
	pub fn is_combinator_deep(&self) -> bool {
		match self {
			Selector::Child { .. } | Selector::Descendant { .. } => true,
			Selector::AnyOf(parts) | Selector::AllOf(parts) => {
				parts.iter().any(Selector::is_combinator_deep)
			}
			Selector::Not(inner) => inner.is_combinator_deep(),
			_ => false,
		}
	}
}
