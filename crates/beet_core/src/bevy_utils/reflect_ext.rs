//! Reflection helpers built on `bevy_reflect`.
use crate::prelude::*;
use alloc::boxed::Box;
use bevy_reflect::PartialReflect;
use bevy_reflect::ReflectFromReflect;
use bevy_reflect::TypeInfo;
use bevy_reflect::TypeRegistration;
use bevy_reflect::TypeRegistry;
use bevy_reflect::enums::DynamicEnum;
use bevy_reflect::enums::DynamicVariant;
use bevy_reflect::enums::EnumInfo;
use bevy_reflect::enums::VariantInfo;

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

/// The `Some` variant's inner [`TypeInfo`] when `info` is an `Option<T>`, else
/// `None`.
///
/// Every seam that reads a *declared* type has to see through the wrapper: an
/// `Option<u16>` port is authored, validated and documented as a `u16`, its
/// optionality carried by whether the value is present at all.
pub fn option_some_inner(info: &TypeInfo) -> Option<&'static TypeInfo> {
	let TypeInfo::Enum(info) = info else {
		return None;
	};
	if !info.type_path().starts_with("core::option::Option<") {
		return None;
	}
	match info.variant("Some")? {
		VariantInfo::Tuple(tuple) => tuple.field_at(0)?.type_info(),
		_ => None,
	}
}

/// The unit variant of `enum_info` that `name` spells, as the value to write
/// into the field.
///
/// The one place a human's word becomes a variant, shared by markup (a
/// `kind="user"` attribute) and the cli (a `--format=jsonl` flag), so the two
/// never drift.
///
/// An exact match wins outright, so a lookup costs one probe and an enum whose
/// variants differ only by case still resolves each one exactly. Failing that,
/// the match is case-insensitive: authored input is prose, where
/// `visibility="draft"` is what a human writes, while the Rust variant is
/// `Draft`.
///
/// # Errors
/// A name that spells no unit variant, listing the ones it could have spelt:
/// falling through to a `String` would miss in `from_reflect` and leave the
/// field's DEFAULT in place, so a mistyped variant would read as a working
/// declaration. Also errors when a case-insensitive spelling matches more than
/// one variant, since quietly picking one would make the declaration mean
/// whichever variant happened to be declared first.
pub fn unit_variant_value(
	enum_info: &EnumInfo,
	name: &str,
) -> Result<Box<dyn PartialReflect>> {
	let is_unit =
		|variant: &&VariantInfo| matches!(variant, VariantInfo::Unit(_));
	let unit_variants =
		|| enum_info.iter().filter(is_unit).map(VariantInfo::name);
	let unit = |variant: &str| -> Box<dyn PartialReflect> {
		Box::new(DynamicEnum::new(variant, DynamicVariant::Unit))
	};
	if let Some(variant) = enum_info.variant(name)
		&& is_unit(&variant)
	{
		return unit(variant.name()).xok();
	}
	let matches = unit_variants()
		.filter(|variant| variant.eq_ignore_ascii_case(name))
		.collect::<Vec<_>>();
	match matches.as_slice() {
		[variant] => unit(variant).xok(),
		[] => bevybail!(
			"`{name}` names no unit variant of `{}`; expected one of: {}",
			enum_info.type_path(),
			unit_variants().collect::<Vec<_>>().join(", ")
		),
		ambiguous => bevybail!(
			"`{name}` matches {} variants of `{}` case-insensitively: {}; spell one exactly",
			ambiguous.len(),
			enum_info.type_path(),
			ambiguous.join(", ")
		),
	}
}

/// Look up a registered type by the name a human wrote, whether in markup or in
/// a script.
///
/// A generic type's short path keeps its arguments (eg `Repeat<(), ()>`), so a bare
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
