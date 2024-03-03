use super::*;
use std::fmt::Display;
use std::ops::Deref;
use std::ops::DerefMut;


#[derive(Clone)]
pub struct BoolField {
	pub reflect: FieldReflect<bool>,
}

impl Display for BoolField {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("BoolField")
			.field("name", &self.reflect.field_name)
			.field("value", &self.reflect.get())
			.finish()
	}
}


impl Deref for BoolField {
	type Target = FieldReflect<bool>;
	fn deref(&self) -> &Self::Target { &self.reflect }
}
impl DerefMut for BoolField {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.reflect }
}

impl IntoFieldUi for bool {
	fn into_field_ui(reflect: FieldReflect<bool>) -> FieldUi {
		BoolField { reflect }.into()
	}
}
