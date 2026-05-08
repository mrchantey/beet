use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;


#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deref, Reflect, Get,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TokenDefinition {
	#[deref]
	pub(super) token: Token,
	pub(super) initial: TokenValue,
}

impl Into<Token> for TokenDefinition {
	fn into(self) -> Token { self.token }
}

impl Into<Token> for &TokenDefinition {
	fn into(self) -> Token { self.token.clone() }
}

impl IntoBundle<(NotBundleMarker, Self)> for TokenDefinition {
	fn into_bundle(self) -> impl Bundle {
		OnSpawn::new(move |entity| {
			let mut store = entity.get_mut_or_default::<TokenStore>();
			try_insert_value(&mut store, &self);
			// avoid unnesecary change detection trigger
			if store.contains_key(self.token.key()) {
				store.insert_definition(self)?;
			}
			Ok(())
		})
	}
}

/// A token definition inserted as a bundle is a declaration that this
/// entities `Value` should be mapped to this token
fn try_insert_value(store: &mut Mut<TokenStore>, definition: &TokenDefinition) {
	match definition.schema() {
		schema if schema == &TokenSchema::of::<i32>() => {
			store.insert(I32Value, definition).unwrap();
		}
		_ => {}
	}
}

/// Define a new inline token with an initial value.
pub fn token<T: Typed + Serialize>(
	initial: T,
) -> (TokenDefinition, SetToken<T>) {
	let initial = initial.into_token_value();
	let token = Token::new(TokenKey::new_inline(), initial.schema().clone());
	(
		TokenDefinition {
			token: token.clone(),
			initial,
		},
		SetToken::new(token),
	)
}

pub struct SetToken<T> {
	token: Token,
	phantom: PhantomData<T>,
}
impl<T> SetToken<T> {
	pub fn new(token: Token) -> Self {
		Self {
			token,
			phantom: PhantomData,
		}
	}
}

impl<T: Typed + Serialize> SetToken<T> {
	pub fn set(&self, val: T) -> impl EntityCommand<Result> {
		let token = self.token.clone();
		move |entity: EntityWorldMut| -> Result {
			let ev =
				TokenEvent::mutate_value(entity.id(), token, move |value| {
					let new_value = TypedValue::new(val)?;
					*value = new_value.into();
					Ok(())
				});
			entity
				.into_world_mut()
				.run_system_cached_with::<_, Result, _, _>(
					handle_token_event,
					ev,
				)??;
			Ok(())
		}
	}
}
fn handle_token_event(ev: In<TokenEvent>, mut query: TokenQuery) -> Result {
	query.handle_token_event(ev.0)
}


token!(
	///An `i32` representation of the [`Value`] component.
	I32Value, i32);
