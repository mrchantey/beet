use crate::prelude::*;
use beet_core::prelude::*;

#[derive(Debug, Default, Clone, Reflect, Deref, DerefMut, Resource)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TokenSet {
	#[deref]
	tokens: HashMap<Token, TokenValue>,
	listeners: HashMap<Token, HashSet<Entity>>,
}


impl TokenSet {
	pub fn register(
		&mut self,
		entity: Entity,
		token: Token,
		value: TokenValue,
	) {
		self.tokens.insert(token.clone(), value);
		self.listeners.entry(token).or_default().insert(entity);
	}
	/// The cached value for a token, if any entity has registered it.
	pub fn value_mut(&mut self, token: &Token) -> Option<&mut TokenValue> {
		self.tokens.get_mut(token)
	}
	/// Entities that should receive the resolved [`Value`] when this token
	/// changes.
	pub fn listeners(&self, token: &Token) -> Option<&HashSet<Entity>> {
		self.listeners.get(token)
	}
	pub fn deregister(&mut self, entity: Entity, token: Token) {
		if let Some(listeners) = self.listeners.get_mut(&token) {
			listeners.remove(&entity);
			if listeners.is_empty() {
				self.listeners.remove(&token);
				// no more listeners, can also remove the cached value
				self.tokens.remove(&token);
			}
		}
	}
}
