use super::*;
use bevy::prelude::*;
use num_traits::AsPrimitive;
use std::collections::VecDeque;

macro_rules! impl_options {
	($ty:ty => $options:ty) => {
		impl InspectorOptionsType for $ty {
			type DeriveOptions = $options;
			type Options = $options;

			fn options_from_derive(
				options: Self::DeriveOptions,
			) -> Self::Options {
				options
			}
		}
	};
}
macro_rules! impl_options_defer_generic {
	($name:ident < $generic:ident >) => {
		impl<$generic: InspectorOptionsType> InspectorOptionsType
			for $name<$generic>
		{
			type DeriveOptions =
				<$generic as InspectorOptionsType>::DeriveOptions;
			type Options = <$generic as InspectorOptionsType>::Options;

			fn options_from_derive(
				options: Self::DeriveOptions,
			) -> Self::Options {
				$generic::options_from_derive(options)
			}
		}
	};
}

#[derive(Clone, PartialEq)]
#[non_exhaustive]
pub struct NumberOptions<T: PartialEq> {
	pub min: Option<T>,
	pub max: Option<T>,
	pub step: T,
	pub prefix: String,
	pub suffix: String,
	pub display: NumberDisplay,
}

impl<T: 'static + Copy + PartialEq> Default for NumberOptions<T>
where
	f64: AsPrimitive<T>,
{
	fn default() -> Self {
		Self {
			min: None,
			max: None,
			step: 0.1.as_(),
			prefix: String::new(),
			suffix: String::new(),
			display: NumberDisplay::default(),
		}
	}
}

#[derive(Clone, Copy, Default, PartialEq)]
#[non_exhaustive]
pub enum NumberDisplay {
	#[default]
	Drag,
	Slider,
}

impl_options!(f32 => NumberOptions<f32>);
impl_options!(f64 => NumberOptions<f64>);
impl_options!(i8 => NumberOptions<i8>);
impl_options!(i16 => NumberOptions<i16>);
impl_options!(i32 => NumberOptions<i32>);
impl_options!(i64 => NumberOptions<i64>);
impl_options!(i128 => NumberOptions<i128>);
impl_options!(isize => NumberOptions<isize>);
impl_options!(u8 => NumberOptions<u8>);
impl_options!(u16 => NumberOptions<u16>);
impl_options!(u32 => NumberOptions<u32>);
impl_options!(u64 => NumberOptions<u64>);
impl_options!(u128 => NumberOptions<u128>);
impl_options!(usize => NumberOptions<usize>);

impl_options!(Vec2 => NumberOptions<f32>);
impl_options!(Vec3 => NumberOptions<f32>);


#[non_exhaustive]
pub struct RangeOptions<T: InspectorOptionsType> {
	pub start: T::Options,
	pub end: T::Options,
}

impl<T: InspectorOptionsType> Clone for RangeOptions<T> {
	fn clone(&self) -> Self {
		Self {
			start: self.start.clone(),
			end: self.end.clone(),
		}
	}
}

impl<T: InspectorOptionsType> Default for RangeOptions<T> {
	fn default() -> Self {
		Self {
			start: T::options_from_derive(T::DeriveOptions::default()),
			end: T::options_from_derive(T::DeriveOptions::default()),
		}
	}
}

impl<T: InspectorOptionsType + 'static> InspectorOptionsType
	for std::ops::Range<T>
{
	type DeriveOptions = RangeOptions<T>;
	type Options = RangeOptions<T>;

	fn options_from_derive(options: Self::DeriveOptions) -> Self::Options {
		options
	}
}

#[derive(Default, Clone)]
#[non_exhaustive]
pub struct QuatOptions {
	pub display: QuatDisplay,
}

#[derive(Copy, Clone, Default)]
pub enum QuatDisplay {
	Raw,
	#[default]
	Euler,
	YawPitchRoll,
	AxisAngle,
}

impl_options!(Quat => QuatOptions);

#[derive(Clone)]
#[non_exhaustive]
pub struct EntityOptions {
	pub display: EntityDisplay,
	pub despawnable: bool,
}

impl Default for EntityOptions {
	fn default() -> Self {
		Self {
			display: EntityDisplay::default(),
			despawnable: true,
		}
	}
}

#[derive(Copy, Clone, Default)]
#[non_exhaustive]
pub enum EntityDisplay {
	Id,
	#[default]
	Components,
}

impl_options!(Entity => EntityOptions);

impl<T: InspectorOptionsType> InspectorOptionsType for Option<T> {
	type DeriveOptions = T::DeriveOptions;
	type Options = InspectorOptions;

	fn options_from_derive(options: Self::DeriveOptions) -> Self::Options {
		let inner_options = T::options_from_derive(options);

		let mut inspector_options = InspectorOptions::new();
		inspector_options.insert(
			InspectorTarget::VariantField {
				variant_index: 1, // Some
				field_index: 0,
			},
			inner_options,
		);

		inspector_options
	}
}

impl_options_defer_generic!(Vec<T>);
impl_options_defer_generic!(VecDeque<T>);

impl<T: InspectorOptionsType, const N: usize> InspectorOptionsType for [T; N] {
	type DeriveOptions = T::DeriveOptions;
	type Options = T::Options;

	fn options_from_derive(options: Self::DeriveOptions) -> Self::Options {
		T::options_from_derive(options)
	}
}
