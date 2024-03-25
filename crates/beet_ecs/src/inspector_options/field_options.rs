use crate::prelude::*;
use bevy::reflect::TypeData;

/// Value is ignored for `[PartialEq]`
pub struct FieldOptions {
	pub ident: FieldIdent,
	pub value: Box<dyn TypeData>,
}

impl FieldOptions {
	pub fn new(ident: FieldIdent, value: Box<dyn TypeData>) -> Self {
		Self { ident, value }
	}
}


impl Clone for FieldOptions {
	fn clone(&self) -> Self {
		Self {
			ident: self.ident.clone(),
			value: self.value.clone_type_data(),
		}
	}
}

impl PartialEq for FieldOptions {
	fn eq(&self, other: &Self) -> bool { self.ident == other.ident }
}
