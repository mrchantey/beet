//! Literal-to-reflected-value resolution, with type inference.
//!
//! A [`DataLiteral`] becomes a `Box<dyn PartialReflect>`, inferring its concrete
//! type from the target field's [`TypeInfo`]: a `{x:0,y:0,z:2}` on a `Vec3` field
//! builds a `Vec3`, `Center` infers the enum variant, and `0` coerces to `0.0f32`
//! when the field is `f32`. Every dynamic value calls `set_represented_type` with
//! the target's `'static` `TypeInfo`, so `from_reflect`/`apply` resolve the
//! concrete type downstream.

use super::ast::*;
use crate::prelude::*;
use bevy::reflect::PartialReflect;
use bevy::reflect::TypeInfo;
use bevy::reflect::TypeRegistry;
use bevy::reflect::enums::DynamicEnum;
use bevy::reflect::enums::DynamicVariant;
use bevy::reflect::enums::VariantInfo;
use bevy::reflect::list::DynamicList;
use bevy::reflect::structs::DynamicStruct;
use bevy::reflect::tuple::DynamicTuple;
use bevy::reflect::tuple_struct::DynamicTupleStruct;
use core::any::TypeId;

/// Resolves a `$name` entity reference to a concrete (possibly forward-mapped)
/// [`Entity`], threaded through nested literals so a spread component's
/// `Entity`-typed field resolves through the one entity model.
pub type EntityResolver<'a> = &'a mut dyn FnMut(&str) -> Entity;

/// Resolve a literal to a reflected value against `field_info` (the target
/// field's [`TypeInfo`], when known), looking nested types up in `registry` and
/// resolving any nested `$name` through `resolver`.
pub fn literal_to_reflect(
	literal: &DataLiteral,
	field_info: Option<&'static TypeInfo>,
	registry: &TypeRegistry,
	resolver: EntityResolver,
) -> Result<Box<dyn PartialReflect>> {
	// an `Option<T>` target wraps a plain value into `Some`: `title="x"` on an
	// `Option<String>` field resolves to `Some("x")`. An explicit `Some`/`None`
	// literal falls through to the ordinary enum path.
	if let Some(some_info) = option_some_inner(field_info)
		&& !is_option_literal(literal)
	{
		let inner =
			literal_to_reflect(literal, Some(some_info), registry, resolver)?;
		let mut tuple = DynamicTuple::default();
		tuple.insert_boxed(inner);
		let mut option = DynamicEnum::new("Some", DynamicVariant::Tuple(tuple));
		option.set_represented_type(field_info);
		return Ok(Box::new(option));
	}
	match literal {
		DataLiteral::Scalar(value) => scalar_to_reflect(value, field_info),
		DataLiteral::List(items) => {
			list_to_reflect(items, field_info, registry, resolver)
		}
		DataLiteral::Struct(fields) => {
			struct_to_reflect(fields, field_info, registry, resolver)
		}
		DataLiteral::Enum(named) => {
			enum_to_reflect(named, field_info, registry, resolver)
		}
		DataLiteral::EntityRef(name) => Ok(Box::new(resolver(name))),
	}
}

/// The `Some` variant's inner [`TypeInfo`] when `field_info` is an
/// `Option<T>` enum, else `None`.
fn option_some_inner(
	field_info: Option<&'static TypeInfo>,
) -> Option<&'static TypeInfo> {
	let TypeInfo::Enum(info) = field_info? else {
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

/// Whether a literal already names an `Option` variant (`Some`/`None`).
fn is_option_literal(literal: &DataLiteral) -> bool {
	matches!(literal, DataLiteral::Enum(named) if named.name == "Some" || named.name == "None")
}

/// Look up a registered type's `'static` [`TypeInfo`] by short type path.
pub fn type_info_by_name(
	registry: &TypeRegistry,
	name: &str,
) -> Option<&'static TypeInfo> {
	registry
		.get_with_short_type_path(name)
		.map(|registration| registration.type_info())
}

/// Coerce a scalar [`Value`] to the field's concrete type, falling through to
/// its natural reflect type when there is no field info to coerce against.
fn scalar_to_reflect(
	value: &Value,
	field_info: Option<&'static TypeInfo>,
) -> Result<Box<dyn PartialReflect>> {
	// numeric coercion: read as f64 then cast to the field's concrete type id.
	let as_f64 = match value {
		Value::Uint(uint) => Some(*uint as f64),
		Value::Int(int) => Some(*int as f64),
		Value::Float(float) => Some(*float),
		_ => None,
	};
	if let (Some(number), Some(TypeInfo::Opaque(opaque))) = (as_f64, field_info) {
		if let Some(reflected) = cast_number(number, opaque.type_id()) {
			return Ok(reflected);
		}
	}

	// a string targeting a `SmolStr` field coerces to `SmolStr`, mirroring the
	// numeric cast above (the natural reflect type of a string is `String`).
	if let (Value::Str(string), Some(TypeInfo::Opaque(opaque))) =
		(value, field_info)
		&& opaque.type_id() == TypeId::of::<SmolStr>()
	{
		return Ok(Box::new(SmolStr::new(string)));
	}

	// a string targeting a `SmolPath` field coerces to a logical path, so a markup
	// `src="assets"` resolves to a `SmolPath` (a tuple struct, hence checked by
	// `type_id` rather than the opaque branch above).
	if let (Value::Str(string), Some(info)) = (value, field_info)
		&& info.type_id() == TypeId::of::<SmolPath>()
	{
		return Ok(Box::new(SmolPath::new(string.as_str())));
	}

	// otherwise the value's natural reflect type.
	let reflected: Box<dyn PartialReflect> = match value {
		Value::Bool(b) => Box::new(*b),
		Value::Int(int) => Box::new(*int),
		Value::Uint(uint) => Box::new(*uint),
		Value::Float(float) => Box::new(*float),
		Value::Str(string) => Box::new(string.to_string()),
		other => bevybail!("cannot reflect scalar value `{other:?}`"),
	};
	Ok(reflected)
}

/// Cast a number to a registered scalar type by its [`TypeId`].
fn cast_number(number: f64, type_id: TypeId) -> Option<Box<dyn PartialReflect>> {
	if type_id == TypeId::of::<f32>() {
		Some(Box::new(number as f32))
	} else if type_id == TypeId::of::<f64>() {
		Some(Box::new(number))
	} else if type_id == TypeId::of::<i8>() {
		Some(Box::new(number as i8))
	} else if type_id == TypeId::of::<i16>() {
		Some(Box::new(number as i16))
	} else if type_id == TypeId::of::<i32>() {
		Some(Box::new(number as i32))
	} else if type_id == TypeId::of::<i64>() {
		Some(Box::new(number as i64))
	} else if type_id == TypeId::of::<u8>() {
		Some(Box::new(number as u8))
	} else if type_id == TypeId::of::<u16>() {
		Some(Box::new(number as u16))
	} else if type_id == TypeId::of::<u32>() {
		Some(Box::new(number as u32))
	} else if type_id == TypeId::of::<u64>() {
		Some(Box::new(number as u64))
	} else if type_id == TypeId::of::<usize>() {
		Some(Box::new(number as usize))
	} else {
		None
	}
}

/// Build a [`DynamicList`] from items, recursing per the list's item info.
fn list_to_reflect(
	items: &[DataLiteral],
	field_info: Option<&'static TypeInfo>,
	registry: &TypeRegistry,
	resolver: EntityResolver,
) -> Result<Box<dyn PartialReflect>> {
	let item_info = match field_info {
		Some(TypeInfo::List(info)) => info.item_info(),
		Some(TypeInfo::Array(info)) => info.item_info(),
		_ => None,
	};
	let mut list = DynamicList::default();
	for item in items {
		list.push_box(literal_to_reflect(item, item_info, registry, resolver)?);
	}
	list.set_represented_type(field_info);
	Ok(Box::new(list))
}

/// Build a [`DynamicStruct`] from named fields, recursing per field info.
fn struct_to_reflect(
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
			literal_to_reflect(literal, nested, registry, resolver)?,
		);
	}
	dynamic.set_represented_type(field_info);
	Ok(Box::new(dynamic))
}

/// Build a named literal (`Name`, `Name(..)`, `Name { .. }`) to a reflected
/// value, dispatching on the target's [`TypeInfo`]: a struct/tuple-struct target
/// (a component spread) builds a [`DynamicStruct`]/[`DynamicTupleStruct`], an
/// enum (or unknown) target builds a [`DynamicEnum`].
fn enum_to_reflect(
	named: &NamedLiteral,
	field_info: Option<&'static TypeInfo>,
	registry: &TypeRegistry,
	resolver: EntityResolver,
) -> Result<Box<dyn PartialReflect>> {
	match field_info {
		Some(TypeInfo::Struct(_)) => {
			return named_struct_to_reflect(named, field_info, registry, resolver);
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
				tuple.insert_boxed(literal_to_reflect(
					item, nested, registry, resolver,
				)?);
			}
			DynamicVariant::Tuple(tuple)
		}
		(NamedFields::Struct(struct_fields), variant) => {
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
					literal_to_reflect(literal, nested, registry, resolver)?,
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

/// Build a [`DynamicStruct`] from a named literal targeting a struct component,
/// eg a `{MyComponent{foo:"bar"}}` spread. Unit/tuple forms become an empty
/// patch over default.
fn named_struct_to_reflect(
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
				literal_to_reflect(literal, nested, registry, resolver)?,
			);
		}
	}
	dynamic.set_represented_type(field_info);
	Ok(Box::new(dynamic))
}

/// Build a [`DynamicTupleStruct`] from a named literal targeting a tuple-struct
/// component, eg `{Wrapper(1, 2)}`.
fn named_tuple_struct_to_reflect(
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
			dynamic.insert_boxed(literal_to_reflect(
				item, nested, registry, resolver,
			)?);
		}
	}
	dynamic.set_represented_type(field_info);
	Ok(Box::new(dynamic))
}

#[cfg(test)]
mod test {
	use super::*;
	use bevy::reflect::FromReflect;
	use bevy::reflect::Typed;

	fn resolve<T: FromReflect + Typed>(literal: DataLiteral) -> T {
		let registry = TypeRegistry::default();
		let mut resolver = |_: &str| Entity::PLACEHOLDER;
		let reflected = literal_to_reflect(
			&literal,
			Some(T::type_info()),
			&registry,
			&mut resolver,
		)
		.unwrap();
		T::from_reflect(reflected.as_ref()).unwrap()
	}

	#[beet_core::test]
	fn wraps_scalar_into_option() {
		resolve::<Option<String>>(DataLiteral::Scalar(Value::str("beet")))
			.xpect_eq(Some("beet".to_string()));
		resolve::<Option<u32>>(DataLiteral::Scalar(Value::Uint(7)))
			.xpect_eq(Some(7));
	}

	#[beet_core::test]
	fn explicit_none_passes_through() {
		resolve::<Option<String>>(DataLiteral::Enum(NamedLiteral {
			name: "None".into(),
			fields: NamedFields::Unit,
		}))
		.xpect_eq(None);
	}

	#[derive(Debug, Default, PartialEq, Reflect)]
	enum Emphasis {
		#[default]
		Low,
		High,
	}

	/// A qualified unit-variant path (`Emphasis::High`) resolves to its variant,
	/// not the enum default, the bug that left a `<Link variant=ButtonVariant::Outlined>`
	/// rendering filled.
	#[beet_core::test]
	fn qualified_unit_variant_resolves() {
		resolve::<Emphasis>(DataLiteral::Enum(NamedLiteral {
			name: "Emphasis::High".into(),
			fields: NamedFields::Unit,
		}))
		.xpect_eq(Emphasis::High);
	}
}
