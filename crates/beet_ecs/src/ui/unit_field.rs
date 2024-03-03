use super::*;
use bevy_utils::default;
use std::time::Duration;
// use std::fmt::Debug;
// use std::fmt::Display;
// use std::ops::Deref;
// use std::ops::DerefMut;
// use strum::IntoEnumIterator;
// use strum_macros::EnumIter;



impl IntoFieldUi for Duration {
	fn into_field_ui(reflect: FieldReflect<Self>) -> FieldUi {
		FieldUi::NumberF64(NumberField {
			reflect: FieldReflect::<f64>::new(
				reflect.field_name.clone(),
				{
					let reflect = reflect.clone();
					move || reflect.get().as_secs_f64()
				},
				{
					let reflect = reflect.clone();
					move |val| reflect.set(Duration::from_secs_f64(val))
				},
			),
			min: 0.,
			max: 10.,
			step: 0.01,
			..default()
		})
	}
}

// pub trait UnitFieldValue: 'static + Debug + FieldValue + PartialOrd {
// 	fn from_f64(&self, val: f64);
// 	fn to_f64(&self) -> f64;
// }
// // impl<T: 'static + Debug + FieldValue + PartialOrd> UnitFieldValue for T {}
// pub trait UnitFieldIter: 'static + IntoEnumIterator + Display {
// 	fn min(&self) -> f64;
// 	fn max(&self) -> f64;
// 	fn step(&self) -> f64;
// }
// // impl<T: 'static + IntoEnumIterator + Display> UnitFieldIter for T {}

// #[derive(Default, EnumIter, strum_macros::Display)]
// pub enum DurationUnit {
// 	Nanoseconds,
// 	Microseconds,
// 	Milliseconds,
// 	#[default]
// 	Seconds,
// 	Minutes,
// 	Hours,
// }

// pub struct UnitField<ValueT: UnitFieldValue, UnitT: UnitFieldIter> {
// 	pub reflect: FieldReflect<ValueT>,
// 	pub unit: UnitT,
// 	pub slider: SliderField<f64>,
// }

// impl<ValueT: UnitFieldValue, UnitT: UnitFieldIter> UnitField<ValueT, UnitT> {
// 	pub fn new(
// 		field_name: String,
// 		get_cb: impl 'static + Fn() -> ValueT,
// 		set_cb: impl 'static + Fn(ValueT),
// 		unit: UnitT,
// 	) -> Self {
// 		Self::from_reflect(FieldReflect::new(field_name, get_cb, set_cb), unit)
// 	}
// 	pub fn from_reflect(reflect: FieldReflect<ValueT>, unit: UnitT) -> Self {
// 		let slider = SliderField::new(
// 			"value".to_string(),
// 			|| {
// 				let this = reflect.get();
// 				this.to_f64()
// 			},
// 			|val| {
// 				let this = reflect.get();
// 				reflect.set(this.from_f64(val));
// 			},
// 			unit.min(),
// 			unit.max(),
// 			unit.step(),
// 		);

// 		Self {
// 			reflect,
// 			unit,
// 			slider,
// 		}
// 	}
// }

// impl<T: UnitFieldValue, UnitT: UnitFieldIter> Deref for UnitField<T, UnitT> {
// 	type Target = FieldReflect<T>;
// 	fn deref(&self) -> &Self::Target { &self.reflect }
// }

// impl<T: UnitFieldValue, UnitT: UnitFieldIter> DerefMut for UnitField<T, UnitT> {
// 	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.reflect }
// }

// impl<T: UnitFieldValue, UnitT: UnitFieldIter> Display for UnitField<T, UnitT> {
// 	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
// 		f.debug_struct("UnitField")
// 			.field("name", &self.reflect.field_name)
// 			.field("value", &self.reflect.get())
// 			.field("unit", &self.unit.to_string())
// 			.finish()
// 	}
// }


// impl IntoFieldUi for Duration {
// 	fn into_field_ui(reflect: FieldReflect<Self>) -> FieldUi {
// 		FieldUi::Duration(UnitField::from_reflect(
// 			reflect,
// 			DurationUnit::default(),
// 		))
// 	}
// }

// impl UnitFieldValue for Duration {
// 	fn from_f64(&self, val: f64) {
// 		*self = Duration::from_secs_f64(val);
// 	}
// 	fn to_f64(&self) -> f64 { self.as_secs_f64() }
// }
