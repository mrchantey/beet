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
use bevy_reflect::tuple::DynamicTuple;

/// Clones a [`PartialReflect`] value, trying [`PartialReflect::reflect_clone`],
/// then [`ReflectFromReflect::from_reflect`], then [`PartialReflect::to_dynamic`].
///
/// This retains the original type and type data where possible, only returning
/// a dynamic type if the first two fail.
///
/// # Errors
///
/// Errors if all three fail, ie the value holds a field that cannot be cloned.
pub fn clone_reflect_value(
	value: &dyn PartialReflect,
	type_registration: &TypeRegistration,
) -> Result<Box<dyn PartialReflect>> {
	value
		.reflect_clone()
		.map(PartialReflect::into_partial_reflect)
		.or_else(|_| {
			type_registration
				.data::<ReflectFromReflect>()
				.and_then(|from_reflect| {
					from_reflect.from_reflect(value.as_partial_reflect())
				})
				.map(PartialReflect::into_partial_reflect)
				.map_or_else(|| value.to_dynamic(), Ok)
		})?
		.xok()
}

/// Refuse `value` where bevy's reflected insert would panic: a partial value
/// of a type with no `#[reflect(Default)]` or `#[reflect(FromWorld)]` to fill
/// the fields it lacks, which [`ReflectFromReflect`] cannot build whole. A
/// type whose fields have no meaningful default, ie a `Did`, has none either,
/// so every field without `#[reflect(default)]` must be present.
///
/// # Errors
///
/// Errors naming the type and the fields `value` lacks.
pub fn assert_constructible(
	value: &dyn PartialReflect,
	registration: &TypeRegistration,
) -> Result {
	use bevy_ecs::reflect::ReflectFromWorld;
	use bevy_reflect::ReflectRef;
	use bevy_reflect::std_traits::ReflectDefault;
	if registration.data::<ReflectDefault>().is_some()
		|| registration.data::<ReflectFromWorld>().is_some()
		|| registration
			.data::<ReflectFromReflect>()
			.is_some_and(|from_reflect| from_reflect.from_reflect(value).is_some())
	{
		return OK;
	}
	let absent = match (registration.type_info(), value.reflect_ref()) {
		(TypeInfo::Struct(info), ReflectRef::Struct(value)) => info
			.iter()
			.filter(|field| value.field(field.name()).is_none())
			.map(|field| format!("`{}`", field.name()))
			.collect::<Vec<_>>(),
		_ => Vec::new(),
	};
	bevybail!(
		"`{}` has no default to complete it from, so it names every field \
		 without `#[reflect(default)]`, absent: {}",
		registration.type_info().type_path_table().short_path(),
		match absent.is_empty() {
			true => "none, but a field could not be built".to_string(),
			false => absent.join(", "),
		}
	)
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
/// into the field. A tuple variant whose every field is `()` reads as unit
/// too, so `Fail` spells beet_action's `Outcome::Fail(())`, which carries
/// nothing to write.
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
) -> Result<DynamicEnum> {
	let is_unit = |variant: &&VariantInfo| match variant {
		VariantInfo::Unit(_) => true,
		VariantInfo::Tuple(tuple) => tuple
			.iter()
			.all(|field| field.type_id() == core::any::TypeId::of::<()>()),
		VariantInfo::Struct(_) => false,
	};
	let unit_variants =
		|| enum_info.iter().filter(is_unit).map(VariantInfo::name);
	let unit = |name: &str| -> DynamicEnum {
		let variant = match enum_info.variant(name) {
			Some(VariantInfo::Tuple(tuple)) => {
				let mut fields = DynamicTuple::default();
				for _ in 0..tuple.field_len() {
					fields.insert(());
				}
				DynamicVariant::Tuple(fields)
			}
			_ => DynamicVariant::Unit,
		};
		DynamicEnum::new(name, variant)
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

/// The resource `registration` names as `world` holds it, `None` when absent
/// or not a reflected resource. Resources are entity-backed components, so it
/// is read through its [`ReflectComponent`] on the resource's entity.
pub fn reflect_resource<'a>(
	world: &'a World,
	registration: &TypeRegistration,
) -> Option<&'a dyn Reflect> {
	registration.data::<ReflectResource>()?;
	let component_id = world.components().get_id(registration.type_id())?;
	let entity = world.resource_entities().get(component_id)?;
	registration
		.data::<ReflectComponent>()?
		.reflect(world.get_entity(entity).ok()?)
}

/// [`registration_by_name`] for a name an entity carries, a tag or a scene
/// body's key, so an ambiguous short path resolves in favour of its sole
/// component or resource: a plain value type sharing the name (registered by
/// `bevy/reflect_auto_register`, ie the data `Rule` beside the `<Rule>`
/// component) is no candidate for a slot only a component fills.
pub fn carried_registration_by_name<'a>(
	registry: &'a TypeRegistry,
	name: &str,
) -> Option<&'a TypeRegistration> {
	if let Some(registration) = registration_by_name(registry, name) {
		return Some(registration);
	}
	if !registry.is_ambiguous(name) {
		return None;
	}
	let mut carried = registry.iter().filter(|registration| {
		registration.type_info().type_path_table().short_path() == name
			&& (registration.data::<ReflectComponent>().is_some()
				|| registration.data::<ReflectResource>().is_some())
	});
	let first = carried.next()?;
	carried.next().is_none().then_some(first)
}
