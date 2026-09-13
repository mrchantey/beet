use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;
use std::sync::Arc;

/// A resolved CSS rule: a predicate (selector) paired with a set of
/// property/value declarations, ready to be serialized to CSS text.
#[derive(Default, Get, SetWith)]
pub struct CssRule {
	selector: Selector,
	declarations: HashMap<CssKey, CssValue>,
	/// Optional `@media` gate carried over from the source [`Rule`]; when set,
	/// serialization wraps this rule in `@media (…) { … }`.
	media: Option<MediaQuery>,
}

impl CssRule {
	/// Build a [`CssRule`] from a [`Rule`] by resolving each token declaration.
	pub fn from_rule(token_map: &CssTokenMap, rule: &Rule) -> Result<Self> {
		let selector = rule.selector().clone();
		let mut declarations = HashMap::default();
		for (key, value) in rule.declarations().iter() {
			let css_rule = Self::resolve(token_map, key, value)?;
			declarations.extend(css_rule.into_declarations());
		}
		Self::default()
			.with_selector(selector)
			.with_declarations(declarations)
			.with_media(*rule.media())
			.xok()
	}

	/// Consumes this rule, returning its declarations map.
	pub fn into_declarations(self) -> HashMap<CssKey, CssValue> {
		self.declarations
	}

	pub fn merge_any(&mut self, other: CssRule) {
		self.selector = self.selector.clone().merge_any(other.selector);
		self.declarations.extend(other.declarations);
	}

	/// Resolves a single token entry to a [`CssRule`].
	///
	/// Tokens are looked up by key in the [`CssTokenMap`].
	pub fn resolve(
		css_map: &CssTokenMap,
		key: &TokenKey,
		value: &TokenValue,
	) -> Result<Self> {
		css_map.get(key)?.as_css_rule(value)
	}

	/// Builds a rule from a token key and a typed value, using the key as the
	/// CSS variable name and appending property suffixes when needed.
	pub fn from_key_value<
		V: 'static
			+ Send
			+ Sync
			+ DeserializeOwned
			+ Typed
			+ TypedTokenKey
			+ AsCssValues,
	>(
		key: &TokenKey,
		value: &TokenValue,
	) -> Result<Self> {
		let key = CssVariable::from_token_key(&key);
		let values = CssValue::from_token_value::<V>(value)?;
		let suffixes = V::suffixes();
		let declarations = if suffixes.len() <= 1 {
			// no need for suffix for zero or one props
			key.xinto::<CssKey>().xvec()
		} else {
			if suffixes.len() != values.len() {
				bevybail!(
					"Property count mismatch:\nkeys: {suffixes:#?}\nvalues:{values:#?}",
				);
			}
			suffixes
				.into_iter()
				.map(|suffix| {
					key.with_suffix(suffix.to_string()).xinto::<CssKey>()
				})
				.collect::<Vec<_>>()
		};
		Self::default()
			.with_declarations(declarations.into_iter().zip(values).collect())
			.xok()
	}

	/// Builds a rule with explicit CSS property keys and a typed value.
	#[cfg(feature = "serde")]
	pub fn from_props_value<
		V: 'static
			+ Send
			+ Sync
			+ DeserializeOwned
			+ Typed
			+ TypedTokenKey
			+ AsCssValues,
	>(
		keys: Vec<CssKey>,
		value: &TokenValue,
	) -> Result<Self> {
		let values = CssValue::from_token_value::<V>(value)?;
		if keys.len() != values.len() {
			bevybail!(
				"Property count mismatch:\nkeys: {keys:#?}\nvalues:{values:#?}",
			);
		}
		Self::default()
			.with_declarations(keys.into_iter().zip(values).collect())
			.xok()
	}

	/// Serialize this rule's selector to CSS.
	///
	/// A css selector list (`a, b`) is only valid at the top level, so a nested
	/// [`Selector::AnyOf`] is distributed rather than emitted inline:
	/// `AllOf([AnyOf([button, .btn]), :hover])` is `button:hover, .btn:hover`,
	/// never `button, .btn:hover`, which would match every button.
	///
	/// ## Panics
	///
	/// Panics on a selector containing [`Selector::Entity`], which is resolved
	/// at runtime against the entity and has no CSS text form. Callers filter
	/// those out with [`Selector::has_entity`] before serializing.
	pub fn selector_to_css(&self) -> String {
		Self::selector_alternatives(&self.selector).join(", ")
	}

	/// The selector's disjunctive normal form: a list of alternatives with no
	/// inner `AnyOf`. `AllOf` and the combinators take the cartesian product of
	/// their parts' alternatives, `Not` applies De Morgan (`:not(a):not(b)`).
	fn selector_alternatives(rule: &Selector) -> Vec<String> {
		match rule {
			Selector::Any => vec!["*".to_string()],
			Selector::Root => vec![":root".to_string()],
			Selector::Entity(_) => unimplemented!(
				"`Selector::Entity` has no CSS form, it is applied at runtime to the entity. Filter with `Selector::has_entity` before serializing"
			),
			Selector::AnyOf(rules) => {
				rules.iter().flat_map(Self::selector_alternatives).collect()
			}
			// concatenated with no separator, ie `.input:focus` or `div.btn`
			Selector::AllOf(rules) => {
				rules.iter().fold(vec![String::new()], |acc, rule| {
					Self::product(&acc, &Self::selector_alternatives(rule), "")
				})
			}
			Selector::Tag(tag) => vec![tag.to_string()],
			Selector::Class(class) => vec![format!(".{}", class)],
			Selector::State(state) => vec![Self::state_to_css(state)],
			Selector::Attribute { key, value } => vec![match value {
				Some(value) => format!("[{}=\"{}\"]", key, value),
				None => format!("[{}]", key),
			}],
			// not (a or b) is not(a) and not(b)
			Selector::Not(inner) => Self::selector_alternatives(inner)
				.iter()
				.map(|alt| format!(":not({alt})"))
				.collect::<String>()
				.xvec(),
			// the descendant combinator, ie `ancestor descendant` (space-joined).
			Selector::Descendant {
				ancestor,
				descendant,
			} => Self::product(
				&Self::selector_alternatives(ancestor),
				&Self::selector_alternatives(descendant),
				" ",
			),
			// the direct-child combinator, ie `parent > child`.
			Selector::Child { parent, child } => Self::product(
				&Self::selector_alternatives(parent),
				&Self::selector_alternatives(child),
				" > ",
			),
		}
	}

	/// Every `left` alternative joined to every `right` alternative.
	fn product(left: &[String], right: &[String], joiner: &str) -> Vec<String> {
		left.iter()
			.flat_map(|left| {
				right
					.iter()
					.map(move |right| format!("{left}{joiner}{right}"))
			})
			.collect()
	}

	/// `Focused` is `:focus-visible`, not `:focus`: a click leaves a button
	/// focused until the next click, so `:focus` would ring the hamburger for as
	/// long as the drawer stays open. `:focus-visible` rings on Tab (the case the
	/// ring exists for) and, per the browser's heuristic, always on a text field.
	fn state_to_css(state: &ElementState) -> String {
		match state {
			ElementState::Hovered => ":hover".to_string(),
			ElementState::Focused => ":focus-visible".to_string(),
			ElementState::Pressed => ":active".to_string(),
			ElementState::Selected => "[aria-selected=\"true\"]".to_string(),
			ElementState::Dragged => "[data-dragging=\"true\"]".to_string(),
			ElementState::Disabled => ":disabled".to_string(),
			ElementState::Custom(val) => format!("[data-state-{}]", val),
		}
	}
}

/// Converts a token value to a [`CssRule`].
pub trait AsCssRule {
	fn as_css_rule(&self, value: &TokenValue) -> Result<CssRule>;
}

/// Maps token keys to their [`AsCssRule`] resolvers.
#[derive(Default, Deref, Resource)]
pub struct CssTokenMap(
	HashMap<TokenKey, Arc<dyn 'static + Send + Sync + AsCssRule>>,
);

impl CssTokenMap {
	/// Registers a CSS resolver keyed on `T`'s type path.
	pub fn insert<T: 'static + Send + Sync + TypedTokenKey + AsCssRule>(
		mut self,
		token: T,
	) -> Self {
		self.0.insert(TokenKey::of::<T>(), Arc::new(token));
		self
	}

	pub fn get(
		&self,
		key: &TokenKey,
	) -> Result<&(dyn Send + Sync + AsCssRule)> {
		self.0.get(key).map(|arc| arc.as_ref()).ok_or_else(|| {
			bevyhow!("No CSS resolver registered for token key:\n{}", key)
		})
	}

	pub fn extend(&mut self, other: Self) -> &mut Self {
		self.0.extend(other.0);
		self
	}

	pub fn with_extend(mut self, other: Self) -> Self {
		self.0.extend(other.0);
		self
	}
}

/// Generates a token type that resolves to named CSS properties.
///
/// ```rust
/// # use beet_ui::prelude::*;
/// # use beet_ui::prelude::style::*;
/// css_property!(MyOpacity, f32, "opacity");
/// ```
#[macro_export]
macro_rules! css_property {
 (
  $(#[$meta:meta])*
  $new_ty:ident,
  $schema_ty:ident,
  $($property:literal),+
 ) => {
  css_property!(
	 $(#[$meta])*
	 $new_ty,
	 $schema_ty,
	 Default::default(),
	 $($property),+
	);
 };
 (
  $(#[$meta:meta])*
  $new_ty:ident,
  $schema_ty:ident,
	$inherited:expr,
  $($property:literal),+
 ) => {
  $crate::token!(
   $(#[$meta])*
   $new_ty,
   $schema_ty,
   $inherited
  );
  impl $crate::prelude::style::AsCssRule for $new_ty {
   fn as_css_rule(
    &self,
    value: &$crate::prelude::TokenValue,
   ) -> ::bevy::prelude::Result<$crate::prelude::style::CssRule> {
    $crate::prelude::style::CssRule::from_props_value::<$schema_ty>(
    vec![$($crate::prelude::style::CssKey::static_property($property)),+],
    value
   )
   }
  }
 };
}

/// Like [`css_property!`] but also implements [`CanonicalToken`] for the value
/// type, so a [`Rule`] can set it via [`Rule::with_canonical`] without naming
/// the property token, eg `Rule::new().with_canonical(Display::None)`.
///
/// Use only when the value type maps to exactly one property (eg `Display` →
/// `display`); a multi-property value like `Color` (`color`, `background-color`,
/// …) has no canonical token and must use [`css_property!`].
///
/// [`CanonicalToken`]: crate::prelude::CanonicalToken
/// [`Rule::with_canonical`]: crate::prelude::Rule::with_canonical
#[macro_export]
macro_rules! canonical_property {
 (
  $(#[$meta:meta])*
  $new_ty:ident,
  $schema_ty:ident,
  $($rest:tt)+
 ) => {
  $crate::css_property!(
   $(#[$meta])*
   $new_ty,
   $schema_ty,
   $($rest)+
  );
  impl $crate::prelude::CanonicalToken for $schema_ty {
   type Token = $new_ty;
  }
 };
}

/// Generates a token type that resolves to a CSS variable declaration.
///
/// ```rust
/// # use beet_ui::prelude::*;
/// # use beet_ui::prelude::style::*;
/// css_variable!(MyOpacityVar, f32);
/// ```
#[macro_export]
macro_rules! css_variable {
 (
  $(#[$meta:meta])*
  $new_ty:ident,
  $schema_ty:ident
 ) => {
  $crate::token!(
   $(#[$meta])*
   $new_ty,
   $schema_ty
  );
  impl $crate::prelude::style::AsCssRule for $new_ty {
   fn as_css_rule(
    &self,
    value: &$crate::prelude::TokenValue,
   ) -> ::bevy::prelude::Result<$crate::prelude::style::CssRule> {
    $crate::prelude::style::CssRule::from_key_value::<$schema_ty>(&$new_ty::token_key(), value)
   }
  }
 };
}

#[cfg(test)]
mod tests {
	use super::*;
	css_property!(
		#[allow(unused)]
		Foo,
		Color,
		"color"
	);
	css_variable!(
		#[allow(unused)]
		Bar,
		Color
	);

	#[beet_core::test]
	fn name() {
		Bar.xinto::<Token>()
			.key()
			.to_string()
			.xpect_eq("io.crates/beet_ui/style/css/css_rule/tests/Bar");
	}

	// the two combinators serialize with their css joiner: a space for
	// descendant, ` > ` for direct child.
	#[beet_core::test]
	fn combinator_selectors() {
		CssRule::default()
			.with_selector(Selector::child(
				Selector::tag("main"),
				Selector::Any,
			))
			.selector_to_css()
			.xpect_eq("main > *");
		CssRule::default()
			.with_selector(Selector::descendant(
				Selector::tag("main"),
				Selector::class("prose"),
			))
			.selector_to_css()
			.xpect_eq("main .prose");
	}

	// a nested `AnyOf` distributes into a top-level list: `button, .btn:focus-visible`
	// would ring every button.
	#[beet_core::test]
	fn nested_any_of_distributes() {
		let buttons = Selector::tag("button").merge_any(Selector::class("btn"));
		let focused = Selector::state(ElementState::Focused);
		CssRule::default()
			.with_selector(Selector::AllOf(vec![
				buttons.clone(),
				focused.clone(),
			]))
			.selector_to_css()
			.xpect_eq("button:focus-visible, .btn:focus-visible");
		// both sides of a compound: the cartesian product
		CssRule::default()
			.with_selector(Selector::AllOf(vec![
				buttons.clone(),
				Selector::class("a").merge_any(Selector::class("b")),
			]))
			.selector_to_css()
			.xpect_eq("button.a, button.b, .btn.a, .btn.b");
		CssRule::default()
			.with_selector(Selector::child(buttons.clone(), Selector::Any))
			.selector_to_css()
			.xpect_eq("button > *, .btn > *");
		CssRule::default()
			.with_selector(Selector::descendant(
				Selector::tag("main"),
				buttons.clone(),
			))
			.selector_to_css()
			.xpect_eq("main button, main .btn");
		// de morgan: not (a or b) is not(a) and not(b)
		CssRule::default()
			.with_selector(Selector::AllOf(vec![
				Selector::class("x"),
				Selector::not(buttons),
			]))
			.selector_to_css()
			.xpect_eq(".x:not(button):not(.btn)");
	}
}
