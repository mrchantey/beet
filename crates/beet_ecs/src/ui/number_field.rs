use super::*;
// use num_traits::AsPrimitive;
use num_traits::FromPrimitive;
use num_traits::ToPrimitive;
use std::fmt::Display;
use std::ops::Deref;
use std::ops::DerefMut;
use std::str::FromStr;

#[derive(Debug, Clone, Default)]
pub enum NumberFieldVariant {
	Text,
	Slider,
	#[default]
	SliderText,
}


pub trait NumberFieldValue:
	'static
	+ FieldValue
	+ PartialOrd
	+ Display
	+ FromStr
	+ ToPrimitive
	+ FromPrimitive
{
}
impl<
		T: 'static
			+ FieldValue
			+ PartialOrd
			+ Display
			+ FromStr
			+ ToPrimitive
			+ FromPrimitive,
	> NumberFieldValue for T
{
}

#[derive(Clone)]
pub struct NumberField<T: NumberFieldValue> {
	pub reflect: FieldReflect<T>,
	pub min: T,
	pub max: T,
	pub step: T,
	pub variant: NumberFieldVariant,
}

impl<T: NumberFieldValue> Default for NumberField<T> {
	fn default() -> Self {
		Self {
			reflect: Default::default(),
			min: T::from_i32(0).unwrap(),
			max: T::from_i32(100).unwrap(),
			step: T::from_i32(1).unwrap(),
			variant: Default::default(),
		}
	}
}


impl Into<NumberField<f64>> for NumberField<f32> {
	fn into(self) -> NumberField<f64> {
		let FieldReflect {
			field_name,
			get_cb,
			set_cb,
			..
		} = self.reflect;
		let reflect = FieldReflect::new(
			field_name,
			move || get_cb() as f64,
			move |v| set_cb(round_to_step_f32(v as f32, self.step)),
		);

		NumberField {
			reflect,
			min: self.min as f64,
			max: self.max as f64,
			step: self.step as f64,
			variant: self.variant,
		}
	}
}

fn round_to_step_f32(value: f32, step: f32) -> f32 {
	(value / step).round() * step
}

impl<T: NumberFieldValue> NumberField<T> {
	pub fn new(
		field_name: String,
		get_cb: impl 'static + Fn() -> T,
		set_cb: impl 'static + Fn(T),
		min: T,
		max: T,
		step: T,
		variant: NumberFieldVariant,
	) -> Self {
		Self {
			reflect: FieldReflect::new(field_name, get_cb, set_cb),
			min,
			max,
			step,
			variant,
		}
	}
	pub fn from_reflect(
		reflect: FieldReflect<T>,
		min: T,
		max: T,
		step: T,
		variant: NumberFieldVariant,
	) -> Self {
		Self {
			reflect,
			variant,
			min,
			max,
			step,
		}
	}
}

impl<T: NumberFieldValue> Deref for NumberField<T> {
	type Target = FieldReflect<T>;
	fn deref(&self) -> &Self::Target { &self.reflect }
}

impl<T: NumberFieldValue> DerefMut for NumberField<T> {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.reflect }
}

impl<T: NumberFieldValue> Display for NumberField<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("NumberField")
			.field("name", &self.reflect.field_name)
			.field("value", &self.reflect.get().to_string())
			.field("min", &self.min.to_string())
			.field("max", &self.max.to_string())
			.field("step", &self.step.to_string())
			.field("display", &self.variant)
			.finish()
	}
}

impl IntoFieldUi for u8 {
	fn into_field_ui(reflect: FieldReflect<Self>) -> FieldUi {
		FieldUi::NumberU8(NumberField {
			reflect,
			variant: NumberFieldVariant::default(),
			min: Self::MIN,
			max: Self::MAX,
			step: 1,
		})
	}
}

impl IntoFieldUi for u16 {
	fn into_field_ui(reflect: FieldReflect<Self>) -> FieldUi {
		FieldUi::NumberU16(NumberField {
			reflect,
			variant: NumberFieldVariant::default(),
			min: 0,
			max: 100,
			step: 1,
		})
	}
}

impl IntoFieldUi for u32 {
	fn into_field_ui(reflect: FieldReflect<Self>) -> FieldUi {
		FieldUi::NumberU32(NumberField {
			reflect,
			variant: NumberFieldVariant::default(),
			min: 0,
			max: 100,
			step: 1,
		})
	}
}

impl IntoFieldUi for u64 {
	fn into_field_ui(reflect: FieldReflect<Self>) -> FieldUi {
		FieldUi::NumberU64(NumberField {
			reflect,
			variant: NumberFieldVariant::default(),
			min: 0,
			max: 100,
			step: 1,
		})
	}
}

impl IntoFieldUi for i8 {
	fn into_field_ui(reflect: FieldReflect<Self>) -> FieldUi {
		FieldUi::NumberI8(NumberField {
			reflect,
			variant: NumberFieldVariant::default(),
			min: 0,
			max: 100,
			step: 1,
		})
	}
}

impl IntoFieldUi for i16 {
	fn into_field_ui(reflect: FieldReflect<Self>) -> FieldUi {
		FieldUi::NumberI16(NumberField {
			reflect,
			variant: NumberFieldVariant::default(),
			min: 0,
			max: 100,
			step: 1,
		})
	}
}

impl IntoFieldUi for i32 {
	fn into_field_ui(reflect: FieldReflect<Self>) -> FieldUi {
		FieldUi::NumberI32(NumberField {
			reflect,
			variant: NumberFieldVariant::default(),
			min: 0,
			max: 100,
			step: 1,
		})
	}
}

impl IntoFieldUi for i64 {
	fn into_field_ui(reflect: FieldReflect<Self>) -> FieldUi {
		FieldUi::NumberI64(NumberField {
			reflect,
			variant: NumberFieldVariant::default(),
			min: 0,
			max: 100,
			step: 1,
		})
	}
}

impl IntoFieldUi for f32 {
	fn into_field_ui(reflect: FieldReflect<Self>) -> FieldUi {
		FieldUi::NumberF32(NumberField {
			reflect,
			variant: NumberFieldVariant::default(),
			min: 0.,
			max: 100.,
			step: 1.,
		})
	}
}

impl IntoFieldUi for f64 {
	fn into_field_ui(reflect: FieldReflect<Self>) -> FieldUi {
		FieldUi::NumberF64(NumberField {
			reflect,
			variant: NumberFieldVariant::default(),
			min: 0.,
			max: 100.,
			step: 1.,
		})
	}
}
