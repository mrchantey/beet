use crate::prelude::*;
use beet_core::prelude::*;

/// Converts a value to its CSS string representation.
pub trait CssValue {
	fn to_css_value(&self) -> String;
}

/// Map a token path to a css key,
/// Multiple tokens may point to the same key,
/// but usually dont when defined in the same crate.
#[derive(Default, Deref)]
pub struct CssKeyMap(HashMap<FieldPath, CssKey>);


impl CssKeyMap {
	pub fn with(mut self, path: FieldPath, key: CssKey) -> Self {
		self.0.insert(path, key);
		self
	}
	pub fn with_property<T: TypedToken>(
		self,
		prop: impl Into<SmolStr>,
	) -> Self {
		self.with(T::path(), CssKey::Property(prop.into()))
	}
	pub fn with_variable<T: TypedToken>(
		self,
		variable: impl Into<SmolStr>,
	) -> Self {
		self.with(T::path(), CssKey::Variable(variable.into()))
	}
}



pub enum CssKey {
	/// The variable name without a leading `--`
	Variable(SmolStr),
	Property(SmolStr),
}
