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

/// Registers
impl<T: 'static> IntoBundle<(NotBundleMarker, Self)> for TokenDefinition<T> {
	fn into_bundle(self) -> impl Bundle {
		OnSpawn::new(move |entity| -> Result {
			match &self.initial {
				TokenValue::Value(value) => {
					// resolve the initial value immediately so consumers can
					// read it on the same frame the entity is spawned.
					entity.insert(value.value().clone());
				}
				TokenValue::Token(_token) => {
					// a ref token has no concrete value of its own; it is
					// resolved through the token it points at.
				}
			};
			let id = entity.id();
			entity.world_scope(|world| {
				world.get_resource_or_init::<TokenSet>().register(
					id,
					self.token,
					self.initial,
				)
			});
			Ok(())
		})
	}
}
