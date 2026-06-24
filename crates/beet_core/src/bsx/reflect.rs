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
use bevy::reflect::TypeRegistration;
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
///
/// A generic type's short path keeps its arguments (eg `Repeat<()>`), so a bare
/// `{Repeat}` spread or `<Repeat>` tag misses the exact lookup; it then falls
/// back to the unique generic instantiation whose base name matches (the `<`
/// boundary guards against prefix collisions like `Repeat` vs `RepeatTimes`).
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
	let mut matches = registry.iter().filter(|registration| {
		let short = registration.type_info().type_path_table().short_path();
		short.len() > name.len()
			&& short.starts_with(name)
			&& short.as_bytes()[name.len()] == b'<'
	});
	let first = matches.next()?;
	matches.next().is_none().then_some(first)
}

/// The [`registration_by_name`] match's [`TypeInfo`], for callers that only need
/// the type info (eg attribute field coercion).
pub fn type_info_by_name(
	registry: &TypeRegistry,
	name: &str,
) -> Option<&'static TypeInfo> {
	registration_by_name(registry, name).map(|reg| reg.type_info())
}

/// Resolve a `Type::Variant` spread name (eg `SteerTarget::Entity`) to the
/// *enum's* registration, so a `{SteerTarget::Entity($cheese)}` spread builds
/// the variant through [`enum_to_reflect`] (which reduces the qualified name to
/// its last segment). `None` when the prefix is not a registered enum carrying
/// that variant, so a genuine miss still falls through to the unknown-name path.
pub fn enum_variant_registration<'a>(
	registry: &'a TypeRegistry,
	name: &str,
) -> Option<&'a TypeRegistration> {
	let (type_name, variant) = name.rsplit_once("::")?;
	let registration = registration_by_name(registry, type_name)?;
	let TypeInfo::Enum(enum_info) = registration.type_info() else {
		return None;
	};
	enum_info.variant(variant).is_some().then_some(registration)
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
	if let (Some(number), Some(TypeInfo::Opaque(opaque))) = (as_f64, field_info)
	{
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

	// a string targeting an `AbsPathBuf` field is treated as workspace-relative and
	// joined onto the workspace root, mirroring `AbsPathBuf`'s workspace-relative
	// serde. This lets eg `<FsStore path="assets"/>` take a string attribute directly,
	// rather than through a thin string-prop template adapter.
	if let (Value::Str(string), Some(info)) = (value, field_info)
		&& info.type_id() == TypeId::of::<AbsPathBuf>()
	{
		return Ok(Box::new(WsPathBuf::new(string.as_str()).into_abs()));
	}

	// a string targeting an enum field coerces to that unit variant by name, so a
	// markup attribute `kind="User"` resolves to `ActorKind::User` (the quoted
	// twin of the `{Foo{kind:User}}` spread's bare-variant form).
	if let (Value::Str(string), Some(TypeInfo::Enum(enum_info))) =
		(value, field_info)
		&& matches!(
			enum_info.variant(string.as_str()),
			Some(VariantInfo::Unit(_))
		) {
		return Ok(Box::new(DynamicEnum::new(
			string.as_str(),
			DynamicVariant::Unit,
		)));
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
fn cast_number(
	number: f64,
	type_id: TypeId,
) -> Option<Box<dyn PartialReflect>> {
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

	/// A generic marker whose registered short path keeps its argument
	/// (`GenericMarker<u32>`), to exercise base-name resolution.
	#[derive(Reflect)]
	struct GenericMarker<T: Reflect>(
		#[reflect(ignore)] core::marker::PhantomData<T>,
	);

	/// A bare base name resolves to the sole generic instantiation, so a
	/// `{Repeat}` spread / `<Repeat>` tag finds `Repeat<()>` despite the argument
	/// kept in its short path. Ambiguity (more than one) resolves to nothing
	/// rather than guessing.
	#[beet_core::test]
	fn generic_resolves_by_base_name() {
		let mut registry = TypeRegistry::default();
		registry.register::<GenericMarker<u32>>();
		type_info_by_name(&registry, "GenericMarker")
			.unwrap()
			.type_path()
			.xpect_eq(GenericMarker::<u32>::type_info().type_path());
		// the exact short path still resolves; an unknown name does not
		type_info_by_name(&registry, "GenericMarker<u32>").xpect_some();
		type_info_by_name(&registry, "Nope").xpect_none();
		// a second instantiation makes the bare name ambiguous
		registry.register::<GenericMarker<bool>>();
		type_info_by_name(&registry, "GenericMarker").xpect_none();
	}

	/// A fully-qualified type path resolves a type whose short path is ambiguous
	/// (two registered `Dup`s), where the bare short name resolves to nothing.
	#[beet_core::test]
	fn qualified_type_path_disambiguates() {
		mod outer {
			#[derive(bevy::prelude::Reflect)]
			pub struct Dup;
		}
		#[derive(Reflect)]
		struct Dup;

		let mut registry = TypeRegistry::default();
		registry.register::<Dup>();
		registry.register::<outer::Dup>();
		// the bare short name is ambiguous, so it resolves to nothing
		type_info_by_name(&registry, "Dup").xpect_none();
		// each fully-qualified path resolves unambiguously
		type_info_by_name(&registry, Dup::type_info().type_path())
			.unwrap()
			.type_path()
			.xpect_eq(Dup::type_info().type_path());
		type_info_by_name(&registry, outer::Dup::type_info().type_path())
			.unwrap()
			.type_path()
			.xpect_eq(outer::Dup::type_info().type_path());
	}

	/// A string attribute targeting an `AbsPathBuf` field coerces workspace-relative,
	/// so `<FsStore path="assets"/>` resolves under the workspace root (the seam that
	/// replaced the `MountFsStore` string-prop adapter).
	#[beet_core::test]
	fn coerces_string_to_abs_path() {
		resolve::<AbsPathBuf>(DataLiteral::Scalar(Value::str("assets")))
			.xpect_eq(WsPathBuf::new("assets").into_abs());
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
