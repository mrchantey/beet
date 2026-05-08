use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;
use std::sync::Arc;

/// A resolved CSS rule: a predicate (selector) paired with a set of
/// property/value declarations, ready to be serialized to CSS text.
#[derive(Default, Get, SetWith)]
pub struct CssRule {
	predicate: Predicate,
	declarations: HashMap<CssKey, CssValue>,
}

impl CssRule {
	/// Build a [`CssRule`] from a [`Rule`] by resolving each token declaration.
	pub fn from_rule(token_map: &CssTokenMap, rule: &Rule) -> Result<Self> {
		let predicate = rule.predicate().clone();
		let mut declarations = HashMap::default();
		for (key, value) in rule.declarations().iter() {
			let css_rule = Self::resolve(token_map, key, value)?;
			declarations.extend(css_rule.into_declarations());
		}
		Self::default()
			.with_predicate(predicate)
			.with_declarations(declarations)
			.xok()
	}

	/// Consumes this rule, returning its declarations map.
	pub fn into_declarations(self) -> HashMap<CssKey, CssValue> {
		self.declarations
	}

	pub fn merge_any(&mut self, other: CssRule) {
		self.predicate = self.predicate.clone().merge_any(other.predicate);
		self.declarations.extend(other.declarations);
	}

	/// Resolves a single token entry to a [`CssRule`].
	///
	/// [`Rule`] values are handled recursively via [`Self::from_rule`].
	/// All other tokens are looked up by key in the [`CssTokenMap`].
	pub fn resolve(
		css_map: &CssTokenMap,
		key: &TokenKey,
		value: &TokenValue,
	) -> Result<Self> {
		// Rules are handled recursively, bypassing the map entirely.
		if value.schema() == &TokenSchema::of::<Rule>() {
			let rule: Rule = match value {
				TokenValue::Value(tv) => tv.value().clone().into_serde()?,
				TokenValue::Token(_) => bevybail!("Expected Rule value"),
			};
			return Self::from_rule(css_map, &rule);
		}
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

	pub fn predicate_to_css(&self) -> String {
		Self::predicate_to_css_inner(&self.predicate)
	}

	fn predicate_to_css_inner(rule: &Predicate) -> String {
		match rule {
			Predicate::Any => "*".to_string(),
			Predicate::Root => ":root".to_string(),
			Predicate::AnyOf(rules) => rules
				.iter()
				.map(|rule| Self::predicate_to_css_inner(rule))
				.collect::<Vec<_>>()
				.join(", "),
			Predicate::AllOf(_rules) => {
				unimplemented!("how to do this properly?")
			}
			Predicate::Tag(tag) => tag.to_string(),
			Predicate::Class(class) => format!(".{}", class),
			Predicate::State(ElementState::Hovered) => ":hover".to_string(),
			Predicate::State(ElementState::Focused) => ":focus".to_string(),
			Predicate::State(ElementState::Pressed) => ":active".to_string(),
			Predicate::State(ElementState::Selected) => {
				"[aria-selected=\"true\"]".to_string()
			}
			Predicate::State(ElementState::Dragged) => {
				"[data-dragging=\"true\"]".to_string()
			}
			Predicate::State(ElementState::Disabled) => ":disabled".to_string(),
			Predicate::State(ElementState::Custom(val)) => {
				format!("[data-state-{}]", val)
			}
			Predicate::Attribute { key, value } => match value {
				Some(value) => format!("[{}=\"{}\"]", key, value),
				None => format!("[{}]", key),
			},
			Predicate::Not(inner) => {
				format!(":not({})", Self::predicate_to_css_inner(inner))
			}
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
  $($property: expr),+
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
   	$crate::prelude::style::CssRule::from_props_value::<$schema_ty>(
   	vec![$($crate::prelude::style::CssKey::static_property($property)),+],
    value
   )
   }
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

	#[test]
	fn name() {
		Bar.xinto::<Token>()
			.key()
			.to_string()
			.xpect_eq("io.crates/beet_ui/style/css/css_rule/tests/Bar");
	}
}
