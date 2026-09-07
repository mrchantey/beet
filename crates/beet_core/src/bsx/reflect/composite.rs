//! Composite construction: a list, struct, enum or named literal to its dynamic
//! reflected value, recursing into each item or field against the target's info.

use super::literal::*;
use super::scalar::*;
use crate::prelude::*;
use bevy::reflect::PartialReflect;
use bevy::reflect::TypeInfo;
use bevy::reflect::TypeRegistry;
use bevy::reflect::array::DynamicArray;
use bevy::reflect::enums::DynamicEnum;
use bevy::reflect::enums::DynamicVariant;
use bevy::reflect::enums::VariantInfo;
use bevy::reflect::list::DynamicList;
use bevy::reflect::structs::DynamicStruct;
use bevy::reflect::tuple::DynamicTuple;
use bevy::reflect::tuple_struct::DynamicTupleStruct;
use core::any::TypeId;

/// Build a [`DynamicList`] (or a [`DynamicArray`] for an array-typed field, eg
/// `host: [u8; 4]`) from items, recursing per the collection's item info.
pub(super) fn list_to_reflect(
	items: &[DataLiteral],
	field_info: Option<&'static TypeInfo>,
	registry: &TypeRegistry,
	resolver: EntityResolver,
) -> Result<Box<dyn PartialReflect>> {
	// a list of patterns targeting a `GlobFilter` field builds the filter from
	// them as includes, so an allowlist authors as the list it reads like:
	// `{ScriptConfig{read:["guestbook.*","Text"]}}`. Its patterns are a
	// private `Vec<GlobPattern>`, so field-by-field reflect construction cannot
	// reach them.
	if let Some(info) = field_info
		&& info.type_id() == TypeId::of::<GlobFilter>()
	{
		let patterns = items
			.iter()
			.map(|item| match item {
				DataLiteral::Scalar(Value::Str(pattern)) => {
					pattern.as_str().xok()
				}
				other => bevybail!(
					"invalid glob pattern {other:?}: expected a string"
				),
			})
			.collect::<Result<Vec<_>>>()?;
		return glob_filter(patterns)
			.map(|filter| Box::new(filter) as Box<dyn PartialReflect>);
	}
	let item_info = match field_info {
		Some(TypeInfo::List(info)) => info.item_info(),
		Some(TypeInfo::Array(info)) => info.item_info(),
		_ => None,
	};
	let values = items
		.iter()
		.map(|item| {
			DataLiteral::to_reflect(item, item_info, registry, resolver)
		})
		.collect::<Result<Vec<_>>>()?;
	if let Some(info @ TypeInfo::Array(_)) = field_info {
		let mut array = DynamicArray::new(values.into_boxed_slice());
		array.set_represented_type(Some(info));
		return Ok(Box::new(array));
	}
	let mut list = DynamicList::default();
	for value in values {
		list.push_box(value);
	}
	list.set_represented_type(field_info);
	Ok(Box::new(list))
}

/// Build a [`DynamicStruct`] from named fields, recursing per field info.
///
/// The result is COMPLETED over the target's default when it can be, since a
/// nested value is consumed by `FromReflect` (a list item, a struct-typed
/// field), which needs every field and silently drops the whole value without
/// them. That is the same fill a top-level component patch gets, one level
/// down: `mailboxes={[{localpart:"probe"}]}` means a default mailbox at that
/// localpart, not a mailbox list that quietly stays empty.
pub(super) fn struct_to_reflect(
	fields: &[(SmolStr, DataLiteral)],
	field_info: Option<&'static TypeInfo>,
	registry: &TypeRegistry,
	resolver: EntityResolver,
) -> Result<Box<dyn PartialReflect>> {
	let struct_info = match field_info {
		Some(TypeInfo::Struct(info)) => Some(info),
		_ => None,
	};
	let mut dynamic = DynamicStruct::default();
	for (name, literal) in fields {
		let nested = struct_info
			.and_then(|info| info.field(name))
			.and_then(|field| field.type_info());
		dynamic.insert_boxed(
			name.as_str(),
			DataLiteral::to_reflect(literal, nested, registry, resolver)?,
		);
	}
	dynamic.set_represented_type(field_info);
	Ok(complete_over_default(
		Box::new(dynamic),
		field_info,
		registry,
	))
}

/// Apply `partial` over its target type's `Default`, yielding a CONCRETE value
/// every field of which is set. Falls through unchanged when the target is
/// unknown, carries no `#[reflect(Default)]`, or refuses the patch (a field the
/// type does not have), so a miss stays as loud as it was.
fn complete_over_default(
	partial: Box<dyn PartialReflect>,
	field_info: Option<&'static TypeInfo>,
	registry: &TypeRegistry,
) -> Box<dyn PartialReflect> {
	use bevy::reflect::std_traits::ReflectDefault;
	let Some(mut value) = field_info
		.and_then(|info| registry.get(info.type_id()))
		.and_then(|registration| registration.data::<ReflectDefault>())
		.map(ReflectDefault::default)
	else {
		return partial;
	};
	match value.try_apply(partial.as_ref()) {
		Ok(()) => value.into_partial_reflect(),
		Err(_) => partial,
	}
}

/// Build a named literal (`Name`, `Name(..)`, `Name { .. }`) to a reflected
/// value, dispatching on the target's [`TypeInfo`]: a struct/tuple-struct target
/// (a component spread) builds a [`DynamicStruct`]/[`DynamicTupleStruct`], an
/// enum (or unknown) target builds a [`DynamicEnum`].
pub(super) fn enum_to_reflect(
	named: &NamedLiteral,
	field_info: Option<&'static TypeInfo>,
	registry: &TypeRegistry,
	resolver: EntityResolver,
) -> Result<Box<dyn PartialReflect>> {
	match field_info {
		Some(TypeInfo::Struct(_)) => {
			return named_struct_to_reflect(
				named, field_info, registry, resolver,
			);
		}
		Some(TypeInfo::TupleStruct(_)) => {
			return named_tuple_struct_to_reflect(
				named, field_info, registry, resolver,
			);
		}
		_ => {}
	}
	let enum_info = match field_info {
		Some(TypeInfo::Enum(info)) => Some(info),
		_ => None,
	};
	// reflection keys on the bare variant name, so a qualified path
	// (`ButtonVariant::Outlined`) reduces to its last segment (`Outlined`), the
	// markup twin of Rust accepting either form. Without this the variant lookup
	// misses and the value silently falls back to the enum's default.
	let variant_name = named.name.rsplit("::").next().unwrap_or(&named.name);
	let variant = enum_info.and_then(|info| info.variant(variant_name));

	let dynamic_variant = match (&named.fields, variant) {
		(NamedFields::Unit, _) => DynamicVariant::Unit,
		(NamedFields::Tuple(items), variant) => {
			let mut tuple = DynamicTuple::default();
			for (index, item) in items.iter().enumerate() {
				let nested = match variant {
					Some(VariantInfo::Tuple(info)) => {
						info.field_at(index).and_then(|f| f.type_info())
					}
					_ => None,
				};
				tuple.insert_boxed(DataLiteral::to_reflect(
					item, nested, registry, resolver,
				)?);
			}
			DynamicVariant::Tuple(tuple)
		}
		(NamedFields::Struct(struct_fields), variant) => {
			assert_variant_complete(variant_name, variant, struct_fields)?;
			let mut dynamic = DynamicStruct::default();
			for (name, literal) in struct_fields {
				let nested = match variant {
					Some(VariantInfo::Struct(info)) => {
						info.field(name).and_then(|f| f.type_info())
					}
					_ => None,
				};
				dynamic.insert_boxed(
					name.as_str(),
					DataLiteral::to_reflect(
						literal, nested, registry, resolver,
					)?,
				);
			}
			DynamicVariant::Struct(dynamic)
		}
	};

	let mut dynamic_enum =
		DynamicEnum::new(variant_name.to_string(), dynamic_variant);
	dynamic_enum.set_represented_type(field_info);
	Ok(Box::new(dynamic_enum))
}

/// Reject a struct-variant literal that omits any of its variant's fields.
///
/// Unlike a struct target, an enum variant cannot be COMPLETED over a default:
/// a default names one variant and says nothing about the others, so
/// `FromReflect` needs every field of the variant actually written. Without
/// this check a partial literal reaches `from_reflect`, misses, and leaves the
/// target's default in place, so `dns={Cloudflare{authority:"x"}}` resolves to
/// no provider at all rather than to a bad one.
fn assert_variant_complete(
	variant_name: &str,
	variant: Option<&'static VariantInfo>,
	fields: &[(SmolStr, DataLiteral)],
) -> Result {
	let Some(VariantInfo::Struct(info)) = variant else {
		return Ok(());
	};
	let missing = info
		.iter()
		.map(|field| field.name())
		.filter(|name| !fields.iter().any(|(given, _)| given == name))
		.collect::<Vec<_>>();
	if !missing.is_empty() {
		bevybail!(
			"`{variant_name}` is missing {missing:?}: an enum variant has no default to fill from, so every field of the variant must be written"
		);
	}
	Ok(())
}

/// Build a [`DynamicStruct`] from a named literal targeting a struct component,
/// eg a `{MyComponent{foo:"bar"}}` spread. Unit/tuple forms become an empty
/// patch over default.
pub(super) fn named_struct_to_reflect(
	named: &NamedLiteral,
	field_info: Option<&'static TypeInfo>,
	registry: &TypeRegistry,
	resolver: EntityResolver,
) -> Result<Box<dyn PartialReflect>> {
	let struct_info = match field_info {
		Some(TypeInfo::Struct(info)) => Some(info),
		_ => None,
	};
	let mut dynamic = DynamicStruct::default();
	if let NamedFields::Struct(fields) = &named.fields {
		for (name, literal) in fields {
			let nested = struct_info
				.and_then(|info| info.field(name))
				.and_then(|field| field.type_info());
			dynamic.insert_boxed(
				name.as_str(),
				DataLiteral::to_reflect(literal, nested, registry, resolver)?,
			);
		}
	}
	dynamic.set_represented_type(field_info);
	Ok(Box::new(dynamic))
}

/// Build a [`DynamicTupleStruct`] from a named literal targeting a tuple-struct
/// component, eg `{Wrapper(1, 2)}`.
pub(super) fn named_tuple_struct_to_reflect(
	named: &NamedLiteral,
	field_info: Option<&'static TypeInfo>,
	registry: &TypeRegistry,
	resolver: EntityResolver,
) -> Result<Box<dyn PartialReflect>> {
	let tuple_info = match field_info {
		Some(TypeInfo::TupleStruct(info)) => Some(info),
		_ => None,
	};
	let mut dynamic = DynamicTupleStruct::default();
	if let NamedFields::Tuple(items) = &named.fields {
		for (index, item) in items.iter().enumerate() {
			let nested = tuple_info
				.and_then(|info| info.field_at(index))
				.and_then(|field| field.type_info());
			dynamic.insert_boxed(DataLiteral::to_reflect(
				item, nested, registry, resolver,
			)?);
		}
	}
	dynamic.set_represented_type(field_info);
	Ok(Box::new(dynamic))
}
