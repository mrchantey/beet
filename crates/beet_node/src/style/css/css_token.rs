use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;
use std::sync::Arc;


#[derive(Default, Get, SetWith)]
pub struct CssToken {
	predicate: Predicate,
	declarations: HashMap<CssKey, CssValue>,
}

impl CssToken {
	pub fn from_rule(token_map: &CssTokenMap, rule: &Rule) -> Result<Self> {
		// Collect all declarations under the rule's predicate, ignoring inner predicates
		let predicate = rule.predicate().clone();
		let mut declarations = HashMap::default();
		for (key, value) in rule.declarations().iter() {
			let css_token = Self::resolve(token_map, key, value)?;
			declarations.extend(css_token.into_declarations());
		}
		Self::default()
			.with_predicate(predicate)
			.with_declarations(declarations)
			.xok()
	}

	/// Consumes this token, returning its declarations map.
	pub fn into_declarations(self) -> HashMap<CssKey, CssValue> {
		self.declarations
	}

	pub fn merge_any(&mut self, other: CssToken) {
		self.predicate = self.predicate.clone().merge_any(other.predicate);
		self.declarations.extend(other.declarations);
	}

	pub fn resolve(
		css_map: &CssTokenMap,
		key: &TokenKey,
		value: &TokenValue,
	) -> Result<Self> {
		// Special-case Rule: deserialize and build CSS recursively
		if value.schema() == &TokenSchema::of::<Rule>() {
			let rule: Rule = match value {
				TokenValue::Value(tv) => tv.value().clone().into_serde()?,
				TokenValue::Token(_) => bevybail!("Expected Rule value"),
			};
			return Self::from_rule(css_map, &rule);
		}
		// Named tokens: look up by key; anonymous (inline) tokens: fall back to schema
		let schema_key = value.schema().as_token_key();
		match css_map.get(key) {
			Ok(resolver) => resolver.as_css_token(value),
			Err(_) => css_map.get(&schema_key)?.as_css_token(value),
		}
	}

	/// Used in CssToken declarations section.
	/// The value type will be checked for multiple properties, and
	/// appended to the key ident if found.
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


pub trait AsCssToken {
	fn as_css_token(&self, value: &TokenValue) -> Result<CssToken>;
}

pub trait TypeAsCssToken {
	fn as_css_token(value: &TokenValue) -> Result<CssToken>;
}
/// A wrapper for storing AsCssToken types in a hashmap
pub struct TypedCssToken<T>(PhantomData<T>);

impl<T> Default for TypedCssToken<T> {
	fn default() -> Self { Self(PhantomData) }
}
impl<T: TypeAsCssToken> AsCssToken for TypedCssToken<T> {
	fn as_css_token(&self, value: &TokenValue) -> Result<CssToken> {
		T::as_css_token(value)
	}
}

/// Store methods for looking up a schema path and resolving a value
#[derive(Default, Deref, Resource)]
pub struct CssTokenMap(
	HashMap<TokenKey, Arc<dyn 'static + Send + Sync + AsCssToken>>,
);
impl CssTokenMap {
	/// Registers a CSS value resolver keyed on `T::Tokens`'s type path.
	///
	/// Stored [`TypedValue`]s carry the schema of their *tokens* struct
	/// (the type actually passed to `with_value`), not the output type,
	/// so the key must match that tokens type.
	pub fn insert<T: 'static + Send + Sync + TypedTokenKey + AsCssToken>(
		mut self,
		token: T,
	) -> Self {
		self.0.insert(TokenKey::of::<T>(), Arc::new(token));
		self
	}
	pub fn insert_type<
		T: 'static + Send + Sync + TypedTokenKey + TypeAsCssToken,
	>(
		mut self,
	) -> Self {
		self.0.insert(
			TokenKey::of::<T>(),
			Arc::new(TypedCssToken::<T>::default()),
		);
		self
	}
	pub fn get(
		&self,
		key: &TokenKey,
	) -> Result<&(dyn Send + Sync + AsCssToken)> {
		self.0.get(key).map(|arc| arc.as_ref()).ok_or_else(|| {
			bevyhow!("No CSS Token registered for this schema:\n{}", key)
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

impl TypeAsCssToken for Rule {
	fn as_css_token(_value: &TokenValue) -> Result<CssToken> {
		// Rules are handled by CssToken::resolve before reaching here
		unimplemented!("Rule CSS generation goes through CssToken::resolve")
	}
}
impl TypeAsCssToken for TokenStore {
	fn as_css_token(_value: &TokenValue) -> Result<CssToken> {
		unimplemented!("TokenStore CSS generation not yet implemented")
	}
}


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
  impl $crate::prelude::style::AsCssToken for $new_ty {
   fn as_css_token(
    &self,
    value: &$crate::prelude::TokenValue,
   ) -> ::bevy::prelude::Result<$crate::prelude::style::CssToken> {
   	$crate::prelude::style::CssToken::from_props_value::<$schema_ty>(
   	vec![$($crate::prelude::style::CssKey::static_property($property)),+],
    value
   )
   }
  }
 };
}


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
  impl $crate::prelude::style::AsCssToken for $new_ty {
   fn as_css_token(
    &self,
    value: &$crate::prelude::TokenValue,
   ) -> ::bevy::prelude::Result<$crate::prelude::style::CssToken> {
    $crate::prelude::style::CssToken::from_key_value::<$schema_ty>(&$new_ty::token_key(), value)
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
	fn test_name() {
		Bar.xinto::<Token>()
			.key()
			.to_string()
			.xpect_eq("io.crates/beet_node/style/css/css_token/tests/Bar");
	}
}
