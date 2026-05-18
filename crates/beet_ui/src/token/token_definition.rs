use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;


#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deref, Reflect, Get,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TokenDefinition<T> {
	#[deref]
	pub(super) token: Token,
	pub(super) initial: TokenValue,
	phantom: PhantomData<T>,
}
// SAETY: T is only used for PhantomData
unsafe impl<T> Send for TokenDefinition<T> {}
// SAETY: T is only used for PhantomData
unsafe impl<T> Sync for TokenDefinition<T> {}

impl<T> TokenDefinition<T>
where
	T: Typed + Serialize + DeserializeOwned,
{
	/// Define a new inline token with an initial value.
	#[track_caller]
	pub fn inline(initial: T) -> Self {
		Self {
			token: Token::new(TokenKey::new_inline(), TokenSchema::of::<T>()),
			initial: initial.into_token_value(),
			phantom: default(),
		}
	}

	pub fn set(&self, new_val: T) -> TokenCommand {
		TokenCommand::mutate_value(self.token.clone(), move |old_val| {
			*old_val = TypedValue::new(new_val)?.into();
			Ok(())
		})
	}
	pub fn update(
		&self,
		func: impl 'static + Send + Sync + FnOnce(&mut T),
	) -> TokenCommand {
		TokenCommand::mutate_value(self.token.clone(), move |old_value| {
			let old_val = match old_value {
				TokenValue::Value(typed) => typed,
				TokenValue::Token(token) => {
					bevybail!("Expected a value, received token {:?}", token)
				}
			}
			.value()
			.clone();
			let mut val = old_val.into_serde::<T>()?;
			func(&mut val);
			*old_value = TypedValue::new(val)?.into();
			Ok(())
		})
	}
}

impl<T> Into<Token> for TokenDefinition<T> {
	fn into(self) -> Token { self.token }
}

impl<T> Into<Token> for &TokenDefinition<T> {
	fn into(self) -> Token { self.token.clone() }
}

impl<T: 'static> IntoBundle<(NotBundleMarker, Self)> for TokenDefinition<T> {
	fn into_bundle(self) -> impl Bundle {
		OnSpawn::new(move |entity| -> Result {
			// The token carries a stable inline key per definition callsite,
			// so derive a shared class from it. Multiple entities created
			// from the same definition reuse a single registered rule.
			let class = ClassName::String(self.token.key().as_str().into());
			let selector = Selector::Class(class.as_selector());

			let mut rule = Rule::new().with_selector(selector);
			if self.schema() == &TokenSchema::of::<i32>() {
				rule.insert(I32Value, &self)?;
			}
			rule.insert_definition(self)?;

			// register the rule once in the global RuleSet resource
			entity.world_scope(move |world| {
				world
					.get_resource_or_init::<RuleSet>()
					.try_insert_inline(rule);
			});
			// ensure the entity carries the class so the rule matches
			if let Some(mut classes) = entity.get_mut::<Classes>() {
				classes.insert_class(class);
			} else {
				entity.insert(Classes::from_iter([class]));
			}
			Ok(())
		})
	}
}


token!(
	///An `i32` representation of the [`Value`] component.
	I32Value, i32);
