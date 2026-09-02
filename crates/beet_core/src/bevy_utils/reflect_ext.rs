//! Reflection helpers built on `bevy_reflect`.
use crate::prelude::*;
use alloc::boxed::Box;
use bevy_reflect::PartialReflect;
use bevy_reflect::ReflectFromReflect;
use bevy_reflect::TypeRegistration;
use bevy_reflect::TypeRegistry;

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

/// Look up a registered type by the name a human wrote, whether in markup or in
/// a script.
///
/// A generic type's short path keeps its arguments (eg `Repeat<()>`), so a bare
/// `{Repeat}` spread, `<Repeat>` tag or `"Repeat"` script identifier misses the
/// exact lookup; it then falls back to the unique generic instantiation whose
/// base name matches (the `<` boundary guards against prefix collisions like
/// `Repeat` vs `RepeatTimes`).
pub fn registration_by_name<'a>(
	registry: &'a TypeRegistry,
	name: &str,
) -> Option<&'a TypeRegistration> {
	if let Some(registration) = registry.get_with_short_type_path(name) {
		return Some(registration);
	}
	// a `::`-qualified name may be a fully-qualified type path: the way to name a
	// type whose short path is ambiguous (eg the two registered `Transform`s,
	// `bevy::transform::components::Transform` vs the CSS one). A bare ambiguous
	// short path resolves to nothing above rather than guessing.
	if name.contains("::")
		&& let Some(registration) = registry.get_with_type_path(name)
	{
		return Some(registration);
	}
	// an ambiguous short path resolves in favour of a sole template candidate,
	// whose short path is the only name it has.
	if let Some(registration) =
		ReflectTemplate::registration_named(registry, name)
	{
		return Some(registration);
	}
	let mut matches = registry.iter().filter(|registration| {
		let short = registration.type_info().type_path_table().short_path();
		short.len() > name.len()
			&& short.starts_with(name)
			&& short.as_bytes()[name.len()] == b'<'
	});
	let first = matches.next()?;
	matches.next().is_none().then_some(first)
}
