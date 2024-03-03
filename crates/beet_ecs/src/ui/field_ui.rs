use super::*;
use crate::prelude::Tree;
use strum_macros::Display;

pub trait IntoFieldUi: 'static + Clone + Sized {
	fn into_field_ui(reflect: FieldReflect<Self>) -> FieldUi;
}

// #[derive(Display)]
#[derive(Clone, Display)]
pub enum FieldUi {
	Heading(HeadingField),
	Group(GroupField),
	Text(TextField),
	Bool(BoolField),
	Select(SelectField),
	// Duration(UnitField<Duration, DurationUnit>),
	// number
	NumberF32(NumberField<f32>),
	NumberF64(NumberField<f64>),
	NumberI8(NumberField<i8>),
	NumberI16(NumberField<i16>),
	NumberI32(NumberField<i32>),
	NumberI64(NumberField<i64>),
	NumberU8(NumberField<u8>),
	NumberU16(NumberField<u16>),
	NumberU32(NumberField<u32>),
	NumberU64(NumberField<u64>),
}

impl FieldUi {
	pub fn into_string_tree(&self) -> Tree<String> {
		match self {
			FieldUi::Heading(val) => Tree::new(val.text.clone()),
			FieldUi::Group(val) => Tree {
				value: val.to_string(),
				children: val
					.children
					.iter()
					.map(|child| child.into_string_tree())
					.collect(),
			},
			FieldUi::Text(val) => Tree::new(val.to_string()),
			FieldUi::Bool(val) => Tree::new(val.to_string()),
			FieldUi::Select(val) => Tree::new(val.to_string()),
			// FieldUi::Duration(val) => Tree::new(val.to_string()),
			FieldUi::NumberF32(val) => Tree::new(val.to_string()),
			FieldUi::NumberF64(val) => Tree::new(val.to_string()),
			FieldUi::NumberI8(val) => Tree::new(val.to_string()),
			FieldUi::NumberI16(val) => Tree::new(val.to_string()),
			FieldUi::NumberI32(val) => Tree::new(val.to_string()),
			FieldUi::NumberI64(val) => Tree::new(val.to_string()),
			FieldUi::NumberU8(val) => Tree::new(val.to_string()),
			FieldUi::NumberU16(val) => Tree::new(val.to_string()),
			FieldUi::NumberU32(val) => Tree::new(val.to_string()),
			FieldUi::NumberU64(val) => Tree::new(val.to_string()),
		}
	}

	pub fn is_equal_graph(&self, other: &FieldUi) -> bool {
		match (self, other) {
			(FieldUi::Group(val), FieldUi::Group(other)) => {
				val.display_name == other.display_name
					&& val.children.len() == other.children.len()
					&& val
						.children
						.iter()
						.zip(other.children.iter())
						.all(|(a, b)| a.is_equal_graph(b))
			}
			(FieldUi::Heading(val), FieldUi::Heading(other)) => val == other,
			(FieldUi::Bool(val), FieldUi::Bool(other)) => {
				val.reflect.field_name == other.reflect.field_name
					&& val.reflect.get() == other.reflect.get()
			}
			(FieldUi::Text(val), FieldUi::Text(other)) => {
				val.reflect.field_name == other.reflect.field_name
					&& val.reflect.get() == other.reflect.get()
			}
			(FieldUi::Select(val), FieldUi::Select(other)) => {
				val.reflect.field_name == other.reflect.field_name
					&& val.reflect.get() == other.reflect.get()
			}
			(FieldUi::NumberF32(val), FieldUi::NumberF32(other)) => {
				val.reflect.field_name == other.reflect.field_name
					&& val.reflect.get() == other.reflect.get()
			}
			(FieldUi::NumberF64(val), FieldUi::NumberF64(other)) => {
				val.reflect.field_name == other.reflect.field_name
					&& val.reflect.get() == other.reflect.get()
			}
			(FieldUi::NumberI8(val), FieldUi::NumberI8(other)) => {
				val.reflect.field_name == other.reflect.field_name
					&& val.reflect.get() == other.reflect.get()
			}
			(FieldUi::NumberI16(val), FieldUi::NumberI16(other)) => {
				val.reflect.field_name == other.reflect.field_name
					&& val.reflect.get() == other.reflect.get()
			}
			(FieldUi::NumberI32(val), FieldUi::NumberI32(other)) => {
				val.reflect.field_name == other.reflect.field_name
					&& val.reflect.get() == other.reflect.get()
			}
			(FieldUi::NumberI64(val), FieldUi::NumberI64(other)) => {
				val.reflect.field_name == other.reflect.field_name
					&& val.reflect.get() == other.reflect.get()
			}
			(FieldUi::NumberU8(val), FieldUi::NumberU8(other)) => {
				val.reflect.field_name == other.reflect.field_name
					&& val.reflect.get() == other.reflect.get()
			}
			(FieldUi::NumberU16(val), FieldUi::NumberU16(other)) => {
				val.reflect.field_name == other.reflect.field_name
					&& val.reflect.get() == other.reflect.get()
			}
			(FieldUi::NumberU32(val), FieldUi::NumberU32(other)) => {
				val.reflect.field_name == other.reflect.field_name
					&& val.reflect.get() == other.reflect.get()
			}
			(FieldUi::NumberU64(val), FieldUi::NumberU64(other)) => {
				val.reflect.field_name == other.reflect.field_name
					&& val.reflect.get() == other.reflect.get()
			}
			// (FieldUi::Duration(val), FieldUi::Duration(other)) => {
			// 	val.reflect.field_name == other.reflect.field_name
			// 		&& val.reflect.get() == other.reflect.get()
			// }
			_ => false,
		}
	}
}


impl Into<FieldUi> for HeadingField {
	fn into(self) -> FieldUi { FieldUi::Heading(self) }
}
impl Into<FieldUi> for BoolField {
	fn into(self) -> FieldUi { FieldUi::Bool(self) }
}
impl Into<FieldUi> for TextField {
	fn into(self) -> FieldUi { FieldUi::Text(self) }
}
impl Into<FieldUi> for GroupField {
	fn into(self) -> FieldUi { FieldUi::Group(self) }
}
// impl Into<FieldUi> for UnitField<Duration, DurationUnit> {
// 	fn into(self) -> FieldUi { FieldUi::Duration(self) }
// }
impl Into<FieldUi> for NumberField<u8> {
	fn into(self) -> FieldUi { FieldUi::NumberU8(self) }
}
impl Into<FieldUi> for NumberField<u16> {
	fn into(self) -> FieldUi { FieldUi::NumberU16(self) }
}
impl Into<FieldUi> for NumberField<u32> {
	fn into(self) -> FieldUi { FieldUi::NumberU32(self) }
}
impl Into<FieldUi> for NumberField<u64> {
	fn into(self) -> FieldUi { FieldUi::NumberU64(self) }
}
impl Into<FieldUi> for NumberField<i8> {
	fn into(self) -> FieldUi { FieldUi::NumberI8(self) }
}
impl Into<FieldUi> for NumberField<i16> {
	fn into(self) -> FieldUi { FieldUi::NumberI16(self) }
}
impl Into<FieldUi> for NumberField<i32> {
	fn into(self) -> FieldUi { FieldUi::NumberI32(self) }
}
impl Into<FieldUi> for NumberField<i64> {
	fn into(self) -> FieldUi { FieldUi::NumberI64(self) }
}
impl Into<FieldUi> for NumberField<f32> {
	fn into(self) -> FieldUi { FieldUi::NumberF32(self) }
}
impl Into<FieldUi> for NumberField<f64> {
	fn into(self) -> FieldUi { FieldUi::NumberF64(self) }
}
