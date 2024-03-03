use super::*;
use anyhow::Result;
use std::fmt::Display;
use std::ops::Deref;
use std::ops::DerefMut;
use strum::IntoEnumIterator;

pub trait SelectFieldValue: FieldValue + Display + IntoEnumIterator {}
impl<T: FieldValue + Display + IntoEnumIterator> SelectFieldValue for T {}

#[derive(Clone)]
pub struct SelectField {
	pub reflect: FieldReflect<usize>,
	pub options: Vec<String>,
}

impl SelectField {
	pub fn new<T: 'static + SelectFieldValue>(
		field_name: String,
		get_cb: GetFunc<T>,
		set_cb: SetFunc<T>,
	) -> Self {
		Self {
			options: T::iter().map(|s| s.to_string()).collect(),
			reflect: FieldReflect::new(
				field_name,
				move || {
					let a = get_cb().to_string();
					T::iter().position(|s| s.to_string() == a).unwrap()
				},
				move |index| {
					let options = T::iter().collect::<Vec<_>>();
					set_cb(options[index].clone());
				},
			),
		}
	}
	// TODO set from Vec<u8> using ciborium

	// pub fn set_index(&self, index: usize) { self.reflect.set(index); }
	/// This ignores the value of the variant, but updates the ui for it to be set,
	/// ie `MyEnum::Variant1(0.5)` will be set to `MyEnum::Variant1(0.0)`
	pub fn set_variant_ignoring_value(&self, val: impl Display) -> Result<()> {
		let val_str = val.to_string();
		let index = self
			.options
			.iter()
			.position(|s| s == &val_str)
			.ok_or_else(|| anyhow::anyhow!("key not found: {}", val))?;
		self.reflect.set(index);
		Ok(())
	}

	pub fn selected_option(&self) -> String {
		self.options[self.reflect.get()].clone()
	}
}


impl Into<FieldUi> for SelectField {
	fn into(self) -> FieldUi { FieldUi::Select(self) }
}

impl Deref for SelectField {
	type Target = FieldReflect<usize>;
	fn deref(&self) -> &Self::Target { &self.reflect }
}
impl DerefMut for SelectField {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.reflect }
}


impl Display for SelectField {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("SelectField")
			.field("name", &self.reflect.field_name)
			.field("value", &self.selected_option())
			.field("index", &self.reflect.get())
			.field("options", &self.options)
			.finish()
	}
}

// impl IntoFieldUi<ValueT> for ValueT {
// 	fn into_field_ui(reflect: FieldReflect<ValueT>) -> FieldUi {
// 		SelectField {
// 			reflect: Box::new(reflect),
// 		}
// 		.into()
// 	}
// }
