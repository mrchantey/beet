use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;


/// System parameter for reading and mutating tokens registered in the
/// [`TokenSet`]. Mutations transparently update the cached value and notify
/// every listening entity by inserting the resolved [`Value`].
#[derive(SystemParam, Get)]
pub struct TokenQuery<'w, 's> {
	commands: Commands<'w, 's>,
	token_set: ResMut<'w, TokenSet>,
}

impl TokenQuery<'_, '_> {
	/// Replace the token's value with `value`.
	pub fn set<T: Typed + Serialize>(
		&mut self,
		token: &Token,
		value: T,
	) -> Result {
		token.schema().assert_eq_ty::<T>()?;
		self.mutate_value(token, None::<fn() -> Value>, move |token_value| {
			*token_value = TypedValue::new(value)?.into();
			Ok(())
		})
	}

	/// Append a value to a list-typed token. Initializes the token to an
	/// empty list if it has no cached value.
	pub fn push<T: Typed + Serialize>(
		&mut self,
		token: &Token,
		value: T,
	) -> Result {
		token.schema().assert_eq_ty::<Vec<T>>()?;
		self.mutate_value(
			token,
			Some(|| Value::List(Vec::new())),
			move |token_value| {
				list_mut(token_value)?.push(Value::from_serde(value)?);
				Ok(())
			},
		)
	}

	/// Insert a value at `index` in a list-typed token. Initializes the
	/// token to an empty list if missing. Clamps `index` to the list length.
	pub fn insert<T: Typed + Serialize>(
		&mut self,
		token: &Token,
		index: usize,
		value: T,
	) -> Result {
		token.schema().assert_eq_ty::<Vec<T>>()?;
		self.mutate_value(
			token,
			Some(|| Value::List(Vec::new())),
			move |token_value| {
				let list = list_mut(token_value)?;
				let index = index.min(list.len());
				list.insert(index, Value::from_serde(value)?);
				Ok(())
			},
		)
	}

	/// Remove the value at `index` from a list-typed token, returning it if
	/// present. Returns `Ok(None)` if the index is out of bounds.
	pub fn remove_at(
		&mut self,
		token: &Token,
		index: usize,
	) -> Result<Option<Value>> {
		let mut removed = None;
		self.mutate_value(token, None::<fn() -> Value>, |token_value| {
			let list = list_mut(token_value)?;
			if index < list.len() {
				removed = Some(list.remove(index));
			}
			Ok(())
		})?;
		Ok(removed)
	}

	pub fn handle_token_command(
		&mut self,
		_entity: Entity,
		cmd: TokenCommand,
	) -> Result {
		let TokenCommand { token, handler } = cmd;
		match handler {
			TokenCommandHandler::MutateValue { init, handler } => {
				self.mutate_value(&token, init, handler)
			}
		}
	}

	/// Apply a closure to the token's value, then notify every listener with
	/// the resolved [`Value`]. The token must be cached, unless `init` is
	/// provided in which case it seeds the entry.
	fn mutate_value(
		&mut self,
		token: &Token,
		init: Option<impl FnOnce() -> Value>,
		handler: impl FnOnce(&mut TokenValue) -> Result,
	) -> Result {
		// mutate the cached value, seeding it from `init` if missing
		let value = {
			let token_value = match (self.token_set.value_mut(token), init) {
				(Some(value), _) => value,
				(None, Some(init)) => self.token_set.insert_value(
					token.clone(),
					TypedValue::from_value(init(), token.schema().clone())
						.into(),
				),
				(None, None) => bevybail!(
					"Token not registered in TokenSet\nkey: {}",
					token.key()
				),
			};
			handler(token_value)?;
			token_value.clone()
		};

		// notify every entity listening on this token with the resolved value
		if let TokenValue::Value(ty_value) = value {
			let value = ty_value.value();
			for entity in self.token_set.iter_listeners(token) {
				self.commands.entity(*entity).insert(value.clone());
			}
		}
		Ok(())
	}
}

/// Coerce a [`TokenValue`] into its inner list, erroring otherwise.
fn list_mut(token_value: &mut TokenValue) -> Result<&mut Vec<Value>> {
	let TokenValue::Value(typed) = token_value else {
		bevybail!("expected value, received token reference")
	};
	match typed.value_mut() {
		Value::List(list) => Ok(list),
		other => bevybail!("expected list, received {}", other.kind()),
	}
}
