use crate::prelude::*;
use beet_core::prelude::*;



#[derive(Deref)]
pub struct CssKeyMap(HashMap<TokenSchema, CssKey>);


impl CssKeyMap {
	pub fn with(mut self, token: TokenSchema, key: CssKey) -> Self {
		self.0.insert(token, key);
		self
	}
	pub fn try_with(mut self, token: TokenSchema, key: CssKey) -> Result<Self> {
		self.0
			.try_insert(token.clone(), key)
			.map_err(|_| format!("Duplicate token in CssKeyMap: {token}"))?;
		self.xok()
	}
}



pub enum CssKey {
	Variable(SmolStr),
	Property(SmolStr),
}
