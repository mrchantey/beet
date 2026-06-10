//! Reflection helpers built on `bevy_reflect`.
use alloc::boxed::Box;
use bevy_reflect::PartialReflect;
use bevy_reflect::ReflectFromReflect;
use bevy_reflect::TypeRegistration;

/// Attempts to clone a [`PartialReflect`] value using various methods.
///
/// This first attempts to clone via [`PartialReflect::reflect_clone`],
/// then falls back to [`ReflectFromReflect::from_reflect`],
/// and finally [`PartialReflect::to_dynamic`] if the first two methods fail.
///
/// This helps ensure the original type and type data is retained,
/// only returning a dynamic type if all other methods fail.
pub fn clone_reflect_value(
	value: &dyn PartialReflect,
	type_registration: &TypeRegistration,
) -> Box<dyn PartialReflect> {
	value
		.reflect_clone()
		.map(PartialReflect::into_partial_reflect)
		.unwrap_or_else(|_| {
			type_registration
				.data::<ReflectFromReflect>()
				.and_then(|from_reflect| {
					from_reflect.from_reflect(value.as_partial_reflect())
				})
				.map(PartialReflect::into_partial_reflect)
				.unwrap_or_else(|| value.to_dynamic())
		})
}
