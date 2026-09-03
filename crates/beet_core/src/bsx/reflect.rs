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
use bevy::reflect::array::DynamicArray;
use bevy::reflect::enums::DynamicEnum;
use bevy::reflect::enums::DynamicVariant;
use bevy::reflect::enums::VariantInfo;
use bevy::reflect::list::DynamicList;
use bevy::reflect::structs::DynamicStruct;
use bevy::reflect::tuple::DynamicTuple;
use bevy::reflect::tuple_struct::DynamicTupleStruct;
use core::any::TypeId;
use core::time::Duration;

/// Resolves a `$name` entity reference to a concrete (possibly forward-mapped)
/// [`Entity`], threaded through nested literals so a spread component's
/// `Entity`-typed field resolves through the one entity model.
pub(crate) type EntityResolver<'a> = &'a mut dyn FnMut(&str) -> Entity;

impl DataLiteral {
	/// Resolve this literal to a reflected value against `field_info` (the target
	/// field's [`TypeInfo`], when known), looking nested types up in `registry` and
	/// resolving any nested `$name` through `resolver`.
	pub fn to_reflect(
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
			let inner = DataLiteral::to_reflect(
				literal,
				Some(some_info),
				registry,
				resolver,
			)?;
			let mut tuple = DynamicTuple::default();
			tuple.insert_boxed(inner);
			let mut option =
				DynamicEnum::new("Some", DynamicVariant::Tuple(tuple));
			option.set_represented_type(field_info);
			return Ok(Box::new(option));
		}
		// a `Name` target builds via `Name::new` from its single string, whether
		// authored as a bare scalar (`name: "x"`) or the tuple form (`<Name("x")/>`,
		// `{Name("x")}`); its hashed inner field cannot be reflect-built field-by-field.
		if let Some(info) = field_info
			&& info.type_id() == TypeId::of::<Name>()
			&& let Some(string) = name_literal_str(literal)
		{
			return scalar_to_reflect(&Value::Str(string.into()), field_info);
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

/// The string a `Name` literal carries: a bare string scalar (`"x"`), or the
/// single-string tuple form (`Name("x")`). `None` for any other shape, so it
/// falls through to the ordinary path.
fn name_literal_str(literal: &DataLiteral) -> Option<&str> {
	match literal {
		DataLiteral::Scalar(Value::Str(string)) => Some(string.as_str()),
		DataLiteral::Enum(named) => match &named.fields {
			NamedFields::Tuple(items) if items.len() == 1 => match &items[0] {
				DataLiteral::Scalar(Value::Str(string)) => {
					Some(string.as_str())
				}
				_ => None,
			},
			_ => None,
		},
		_ => None,
	}
}

/// Look up a registered type by short type path, the [`reflect_ext`] resolver
/// under the name this module's callers use.
pub(crate) fn registration_by_name<'a>(
	registry: &'a TypeRegistry,
	name: &str,
) -> Option<&'a TypeRegistration> {
	reflect_ext::registration_by_name(registry, name)
}

/// The [`registration_by_name`] match's [`TypeInfo`], for callers that only need
/// the type info (eg attribute field coercion).
pub(crate) fn type_info_by_name(
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
pub(crate) fn enum_variant_registration<'a>(
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
	// A numeric string parses too (the quoted twin of the bare-number form), so
	// a markup `port="0"` authors a numeric field directly.
	let as_f64 = match value {
		Value::Uint(uint) => Some(*uint as f64),
		Value::Int(int) => Some(*int as f64),
		Value::Float(float) => Some(*float),
		Value::Str(string) => string.as_str().trim().parse::<f64>().ok(),
		_ => None,
	};
	if let (Some(number), Some(TypeInfo::Opaque(opaque))) = (as_f64, field_info)
	{
		if let Some(reflected) = cast_number(number, opaque.type_id()) {
			return Ok(reflected);
		}
	}

	// a number targeting a single-field tuple-struct wrapping a scalar (a newtype like
	// `LinearVelocity(f32)`) builds that newtype from the bare number, so `<SetDrive
	// linear=60>` authors a typed velocity directly. The inner field's type id drives
	// the cast, mirroring the opaque branch above.
	if let (Some(number), Some(TypeInfo::TupleStruct(info))) =
		(as_f64, field_info)
		&& info.field_len() == 1
		&& let Some(field) = info.field_at(0)
		&& let Some(cast) = cast_number(number, field.type_id())
	{
		let mut dynamic = DynamicTupleStruct::default();
		dynamic.insert_boxed(cast);
		dynamic.set_represented_type(field_info);
		return Ok(Box::new(dynamic));
	}

	// a string targeting a one-string-field struct builds that struct from the
	// string, so a LABEL REFERENCE authors as the label it is: `<EnsureSecret
	// secret="db-password"/>`. A type like `SecretRef` exists so one composition
	// owns a name both ends of a reference compose, and wrapping a label is all
	// it does; without this the string patch misses, `from_reflect` keeps the
	// default, and a block silently points at the empty label. The named twin of
	// the bare-number newtype cast above.
	if let (Value::Str(string), Some(TypeInfo::Struct(info))) =
		(value, field_info)
		&& info.field_len() == 1
		&& let Some(field) = info.field_at(0)
		&& is_string_target(field.type_id())
	{
		let mut dynamic = DynamicStruct::default();
		dynamic.insert_boxed(
			field.name(),
			string_to_reflect(string, field.type_id()),
		);
		dynamic.set_represented_type(field_info);
		return Ok(Box::new(dynamic));
	}

	// a non-numeric string targeting a numeric field errors rather than silently
	// falling through to `String` (whose `from_reflect` miss would keep the
	// target's default, eg a `port="nope"` leaving the default port)
	if let (Value::Str(string), None, Some(info)) = (value, as_f64, field_info)
		&& is_numeric_target(info)
	{
		bevybail!(
			"invalid number {string:?}: expected a numeric string for a `{}` field",
			info.type_path()
		);
	}

	// a human duration string targeting a `Duration` field, so a markup
	// `<EndInDuration duration="50ms"/>` authors a delay directly. A malformed value
	// (a non-string, or a missing/unknown unit) errors rather than silently falling
	// through to a value that cannot apply. `Duration` is `core`, so no_std-safe.
	if let Some(info) = field_info
		&& info.type_id() == TypeId::of::<Duration>()
	{
		let Some(duration) = coerce_duration(value) else {
			bevybail!(
				"invalid duration {value:?}: expected a unit-suffixed string like \"50ms\" or \"1s\""
			);
		};
		return Ok(Box::new(duration));
	}

	// a `"true"`/`"false"` string targeting a `bool` field coerces to the bool, so a
	// markup `<RouteSidebar home="false"/>` authors a flag directly (mirroring the
	// duration string above). Any other string errors rather than silently applying
	// `false`.
	if let (Value::Str(string), Some(info)) = (value, field_info)
		&& info.type_id() == TypeId::of::<bool>()
	{
		let parsed = match string.as_str().trim() {
			"true" => true,
			"false" => false,
			other => {
				bevybail!(
					"invalid bool {other:?}: expected \"true\" or \"false\""
				)
			}
		};
		return Ok(Box::new(parsed));
	}

	// a single pattern targeting a `GlobFilter` field is the one-entry form of
	// the list coercion below, so `read="guestbook.*"` and `read=["guestbook.*"]`
	// both author an allowlist.
	if let (Value::Str(pattern), Some(info)) = (value, field_info)
		&& info.type_id() == TypeId::of::<GlobFilter>()
	{
		return glob_filter([pattern.as_str()])
			.map(|filter| Box::new(filter) as Box<dyn PartialReflect>);
	}

	// a bare string targeting a `ValueSchema` field names the shape the field
	// accepts, so `<DynamicComponent name=".." schema="u64"/>` declares what a
	// runtime component means. `ValueSchema` is reflect-opaque, so this is the
	// only way markup can build one short of a rust expression.
	if let (Value::Str(source), Some(info)) = (value, field_info)
		&& info.type_id() == TypeId::of::<ValueSchema>()
	{
		return value_schema(source.as_str())
			.map(|schema| Box::new(schema) as Box<dyn PartialReflect>);
	}

	// a string targeting a `SmolStr` field coerces to `SmolStr`, mirroring the
	// numeric cast above (the natural reflect type of a string is `String`).
	if let (Value::Str(string), Some(TypeInfo::Opaque(opaque))) =
		(value, field_info)
		&& opaque.type_id() == TypeId::of::<SmolStr>()
	{
		return Ok(Box::new(SmolStr::new(string)));
	}

	// a string targeting a `Cow<'static, str>` field coerces to an owned `Cow`, so
	// a tuple/struct literal carrying a string (eg `<Log::Message("hi")/>`, whose
	// variant field is `Cow<'static, str>`) reflect-applies instead of panicking on
	// the `String`->`Cow` mismatch.
	if let (Value::Str(string), Some(opaque)) = (value, field_info)
		&& opaque.type_id() == TypeId::of::<alloc::borrow::Cow<'static, str>>()
	{
		return Ok(Box::new(alloc::borrow::Cow::<'static, str>::Owned(
			string.to_string(),
		)));
	}

	// a string targeting a `Name` coerces via `Name::new`, so `<Name("Malenia")/>`
	// and a `name: "x"` field both reflect-construct a real `Name` (its hashed
	// inner field cannot be built field-by-field from a plain string).
	if let (Value::Str(string), Some(info)) = (value, field_info)
		&& info.type_id() == TypeId::of::<Name>()
	{
		return Ok(Box::new(Name::new(string.to_string())));
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
	// rather than through a thin string-prop template adapter. `AbsPathBuf`/`WsPathBuf`
	// live in the std-only `path_utils`, so the coercion is std-gated — a no_std
	// (embedded) build has no filesystem paths to resolve.
	#[cfg(feature = "std")]
	if let (Value::Str(string), Some(info)) = (value, field_info)
		&& info.type_id() == TypeId::of::<AbsPathBuf>()
	{
		return Ok(Box::new(WsPathBuf::new(string.as_str()).into_abs()));
	}

	// a hex string targeting a `Color` field coerces through `Srgba::hex`, so a
	// markup `<Theme primary="#006c4f"/>` spells a colour the way every design
	// tool does rather than as a four-float struct literal. Checked ahead of the
	// enum branch below because `Color` IS an enum, and a malformed value errors
	// rather than silently keeping the default.
	#[cfg(feature = "bevy_color")]
	if let (Value::Str(string), Some(info)) = (value, field_info)
		&& info.type_id() == TypeId::of::<bevy::color::Color>()
	{
		let Ok(srgba) = bevy::color::Srgba::hex(string.as_str()) else {
			bevybail!(
				"invalid color {string:?}: expected a hex string like \"#006c4f\""
			);
		};
		return Ok(Box::new(bevy::color::Color::Srgba(srgba)));
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

	// a string targeting an enum field that names no unit variant errors rather
	// than falling through to `String` (whose `from_reflect` miss would keep
	// the field's DEFAULT, so a mistyped variant name would read as a working
	// declaration — eg a `records="identity-only"` leaving `MailRecords::All`
	// in place, which on a cutover-staged mail domain is the difference
	// between proving an identity and taking the domain's mail). `Option` and
	// `Cow` targets never reach here: both are unwrapped by earlier branches.
	if let (Value::Str(string), Some(TypeInfo::Enum(enum_info))) =
		(value, field_info)
	{
		bevybail!(
			"`{string}` names no unit variant of `{}`; expected one of: {}",
			enum_info.type_path(),
			enum_info
				.iter()
				.filter(|variant| matches!(variant, VariantInfo::Unit(_)))
				.map(|variant| variant.name())
				.collect::<Vec<_>>()
				.join(", ")
		);
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

/// Whether `type_id` is one of the string types a markup attribute's text
/// lands in directly.
fn is_string_target(type_id: TypeId) -> bool {
	type_id == TypeId::of::<SmolStr>() || type_id == TypeId::of::<String>()
}

/// Reflect `string` as whichever of the [`is_string_target`] types `type_id`
/// names.
fn string_to_reflect(string: &str, type_id: TypeId) -> Box<dyn PartialReflect> {
	match type_id == TypeId::of::<SmolStr>() {
		true => Box::new(SmolStr::new(string)),
		false => Box::new(string.to_string()),
	}
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

/// Whether a target [`TypeInfo`] is a numeric scalar (a [`cast_number`] id) or
/// a single-field tuple-struct newtype wrapping one.
fn is_numeric_target(info: &TypeInfo) -> bool {
	match info {
		TypeInfo::Opaque(opaque) => {
			cast_number(0.0, opaque.type_id()).is_some()
		}
		TypeInfo::TupleStruct(info) if info.field_len() == 1 => info
			.field_at(0)
			.is_some_and(|field| cast_number(0.0, field.type_id()).is_some()),
		_ => false,
	}
}

/// Coerce a scalar [`Value`] to a [`Duration`] from a unit-suffixed string
/// (eg `"50ms"`, `"1s"`). The unit is required; a bare number carries no unit and
/// is rejected, so a duration is never silently assumed to be milliseconds.
fn coerce_duration(value: &Value) -> Option<Duration> {
	match value {
		Value::Str(string) => Duration::from_human_str(string.as_str()),
		_ => None,
	}
}

/// Build a [`DynamicList`] (or a [`DynamicArray`] for an array-typed field, eg
/// `host: [u8; 4]`) from items, recursing per the collection's item info.
fn list_to_reflect(
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
				DataLiteral::to_reflect(literal, nested, registry, resolver)?,
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
			dynamic.insert_boxed(DataLiteral::to_reflect(
				item, nested, registry, resolver,
			)?);
		}
	}
	dynamic.set_represented_type(field_info);
	Ok(Box::new(dynamic))
}

/// A [`GlobFilter`] over `patterns`, as includes.
///
/// The markup form of a filter is the allowlist a human writes; an exclude
/// needs the struct literal (`{read:{exclude:[..]}}`), which reflects normally.
fn glob_filter<'a>(
	patterns: impl IntoIterator<Item = &'a str>,
) -> Result<GlobFilter> {
	let mut filter = GlobFilter::default();
	for pattern in patterns {
		// `GlobFilter::include` panics on a malformed pattern, and a markup
		// attribute is authored input, so it is validated into an error first.
		GlobFilter::parse_glob_pattern(pattern).map_err(|err| {
			bevyhow!("invalid glob pattern {pattern:?}: {err}")
		})?;
		filter.include(pattern);
	}
	filter.xok()
}

/// A [`ValueSchema`] from the one word a human means, or from a JSON Schema.
///
/// The markup form of a schema is its kind: `schema="u64"`. A shape with more
/// to say than a kind is written as a JSON Schema object, which is the schema
/// language a document author is likeliest to already know, and anything
/// richer than that is a rust expression.
fn value_schema(source: &str) -> Result<ValueSchema> {
	let source = source.trim();
	if source.starts_with('{') {
		return json_schema(source);
	}
	match source {
		"any" => ValueSchema::Any,
		"null" => ValueSchema::Null,
		"bool" => ValueSchema::Bool(default()),
		"i64" => ValueSchema::I64(default()),
		"u64" => ValueSchema::U64(default()),
		"f64" => ValueSchema::F64(default()),
		"string" => ValueSchema::String(default()),
		"bytes" => ValueSchema::Bytes(default()),
		other => bevybail!(
			"unknown schema {other:?}: expected one of \"any\", \"null\", \
\"bool\", \"i64\", \"u64\", \"f64\", \"string\", \"bytes\", or a JSON Schema object"
		),
	}
	.xok()
}

/// A [`ValueSchema`] from one JSON Schema descriptor.
///
/// Not [`ValueSchema::from_json_schema`], which reads its top level as a
/// template's *prop block*: a component's schema is one descriptor, so
/// `{"type":"integer"}` must mean an integer rather than a prop named `type`.
#[cfg(feature = "json")]
fn json_schema(source: &str) -> Result<ValueSchema> {
	ValueSchema::from_json_value(&serde_json::from_str(source)?)
}

/// Parsing a JSON schema requires the `json` feature.
#[cfg(not(feature = "json"))]
fn json_schema(_source: &str) -> Result<ValueSchema> {
	bevybail!("parsing a JSON schema requires the `json` feature")
}

#[cfg(test)]
mod test {
	use super::*;
	use bevy::reflect::FromReflect;
	use bevy::reflect::Typed;

	/// A `GlobFilter` field takes the allowlist a human writes, as a list or as
	/// a bare string, because its patterns are private and reflect cannot build
	/// them field by field.
	#[crate::test]
	fn coerces_patterns_to_a_glob_filter() {
		#[derive(Reflect, PartialEq, Debug, Default)]
		struct Exposure {
			read: GlobFilter,
		}
		let expected = GlobFilter::default().with_include("guestbook.*");
		resolve::<Exposure>(DataLiteral::Enum(NamedLiteral {
			name: "Exposure".into(),
			fields: NamedFields::Struct(vec![(
				"read".into(),
				DataLiteral::List(vec![DataLiteral::Scalar(Value::Str(
					"guestbook.*".into(),
				))]),
			)]),
		}))
		.xpect_eq(Exposure {
			read: expected.clone(),
		});
		resolve::<Exposure>(DataLiteral::Enum(NamedLiteral {
			name: "Exposure".into(),
			fields: NamedFields::Struct(vec![(
				"read".into(),
				DataLiteral::Scalar(Value::Str("guestbook.*".into())),
			)]),
		}))
		.xpect_eq(Exposure { read: expected });
	}

	/// A malformed pattern errors rather than panicking inside the glob
	/// validator, since a markup attribute is authored input.
	#[crate::test]
	fn a_malformed_glob_pattern_names_itself() {
		#[derive(Reflect, PartialEq, Debug, Default)]
		struct Exposure {
			read: GlobFilter,
		}
		let registry = TypeRegistry::default();
		let mut resolver = |_: &str| Entity::PLACEHOLDER;
		DataLiteral::to_reflect(
			&DataLiteral::Scalar(Value::Str("[".into())),
			Exposure::type_info()
				.as_struct()
				.unwrap()
				.field("read")
				.unwrap()
				.type_info(),
			&registry,
			&mut resolver,
		)
		.unwrap_err()
		.to_string()
		.xpect_contains("invalid glob pattern");
	}

	/// A `ValueSchema` field takes the one word a human means, because the type
	/// is reflect-opaque and cannot be built field by field.
	#[crate::test]
	fn coerces_a_kind_to_a_value_schema() {
		#[derive(Reflect, PartialEq, Debug, Default)]
		struct Declaration {
			schema: ValueSchema,
		}
		for (source, expected) in [
			("any", ValueSchema::Any),
			("u64", ValueSchema::U64(default())),
			("bytes", ValueSchema::Bytes(default())),
		] {
			resolve::<Declaration>(schema_literal(source))
				.xpect_eq(Declaration { schema: expected });
		}
	}

	/// A shape with more to say than a kind is written as a JSON Schema, which
	/// is the schema language a document author is likeliest to already know.
	#[crate::test]
	fn coerces_a_json_schema_to_a_value_schema() {
		#[derive(Reflect, PartialEq, Debug, Default)]
		struct Declaration {
			schema: ValueSchema,
		}
		resolve::<Declaration>(schema_literal(
			r#"{"type":"array","items":{"type":"integer"}}"#,
		))
		.schema
		.xpect_eq(
			super::json_schema(
				r#"{"type":"array","items":{"type":"integer"}}"#,
			)
			.unwrap(),
		);
	}

	/// A misspelled kind names the ones that exist rather than becoming a
	/// wildcard that quietly validates nothing.
	#[crate::test]
	fn an_unknown_schema_names_the_kinds() {
		super::value_schema("uint64")
			.unwrap_err()
			.to_string()
			.xpect_contains("unknown schema")
			.xpect_contains("\"u64\"");
	}

	/// One `schema:` field, as a struct literal targeting it.
	fn schema_literal(source: &str) -> DataLiteral {
		DataLiteral::Enum(NamedLiteral {
			name: "Declaration".into(),
			fields: NamedFields::Struct(vec![(
				"schema".into(),
				DataLiteral::Scalar(Value::Str(source.into())),
			)]),
		})
	}

	fn resolve<T: FromReflect + Typed>(literal: DataLiteral) -> T {
		let registry = TypeRegistry::default();
		let mut resolver = |_: &str| Entity::PLACEHOLDER;
		let reflected = DataLiteral::to_reflect(
			&literal,
			Some(T::type_info()),
			&registry,
			&mut resolver,
		)
		.unwrap();
		T::from_reflect(reflected.as_ref()).unwrap()
	}

	/// A bare number coerces into a single-field tuple-struct newtype (eg
	/// `LinearVelocity(f32)`), so `<SetDrive linear=60>` builds the typed wrapper from a
	/// plain attribute. The stored field takes the number directly, in whatever unit
	/// the newtype stores.
	#[crate::test]
	fn coerces_number_to_newtype() {
		#[derive(Reflect, PartialEq, Debug)]
		struct Speed(f32);
		resolve::<Speed>(DataLiteral::Scalar(Value::Float(60.0)))
			.xpect_eq(Speed(60.0));
		resolve::<Speed>(DataLiteral::Scalar(Value::Int(90)))
			.xpect_eq(Speed(90.0));
	}

	/// A plain string coerces into a one-string-field struct, ie a label
	/// reference (`SecretRef`), so `<EnsureSecret secret="db-password"/>`
	/// authors the reference as the label it wraps.
	///
	/// REGRESSION: without the coercion the `String` patch missed the struct
	/// field, `from_reflect` fell back to the type's default, and the block
	/// pointed at the EMPTY label — a deploy that composes `/app/stage/` and
	/// reads someone else's parameter, silently.
	#[crate::test]
	fn coerces_string_to_label_ref() {
		#[derive(Reflect, PartialEq, Debug, Default)]
		struct SecretRef {
			label: SmolStr,
		}
		#[derive(Reflect, PartialEq, Debug, Default)]
		struct Owned {
			label: String,
		}
		resolve::<SecretRef>(DataLiteral::Scalar(Value::Str(
			"db-password".into(),
		)))
		.xpect_eq(SecretRef {
			label: "db-password".into(),
		});
		resolve::<Owned>(DataLiteral::Scalar(Value::Str("net".into())))
			.xpect_eq(Owned {
				label: "net".into(),
			});
	}

	/// ..but only a struct that wraps ONE string: a second field means the
	/// string cannot say which one it is, so the coercion must not guess.
	#[crate::test]
	fn does_not_coerce_string_to_a_wider_struct() {
		#[derive(Reflect, PartialEq, Debug, Default)]
		struct Pair {
			label: SmolStr,
			other: SmolStr,
		}
		let registry = TypeRegistry::default();
		let mut resolver = |_: &str| Entity::PLACEHOLDER;
		let reflected = DataLiteral::to_reflect(
			&DataLiteral::Scalar(Value::Str("net".into())),
			Some(Pair::type_info()),
			&registry,
			&mut resolver,
		)
		.unwrap();
		Pair::from_reflect(reflected.as_ref()).xpect_none();
	}

	/// A nested struct literal names only the fields it cares about, the rest
	/// coming from the target's `Default` — in a struct-typed field and in a
	/// list item alike.
	///
	/// REGRESSION: the partial `DynamicStruct` reached `FromReflect`, which
	/// needs every field, so `mailboxes={[{localpart:"probe"}]}` produced an
	/// EMPTY list and the declaration silently lost its mailboxes.
	#[crate::test]
	fn completes_a_nested_struct_over_its_default() {
		#[derive(Reflect, PartialEq, Debug)]
		struct Item {
			label: SmolStr,
			admin: bool,
		}
		impl Default for Item {
			fn default() -> Self {
				Self {
					label: "none".into(),
					admin: true,
				}
			}
		}
		#[derive(Reflect, PartialEq, Debug, Default)]
		struct Host {
			one: Item,
			many: Vec<Item>,
		}
		// the `Default` data is what the fill reads, so it must be registered
		let mut registry = TypeRegistry::default();
		registry.register::<Item>();
		registry
			.register_type_data::<Item, bevy::reflect::std_traits::ReflectDefault>(
			);
		let mut resolver = |_: &str| Entity::PLACEHOLDER;
		let literal = DataLiteral::Enum(NamedLiteral {
			name: "Host".into(),
			fields: NamedFields::Struct(vec![
				(
					"one".into(),
					DataLiteral::Struct(vec![(
						"label".into(),
						DataLiteral::Scalar(Value::Str("solo".into())),
					)]),
				),
				(
					"many".into(),
					DataLiteral::List(vec![DataLiteral::Struct(vec![(
						"label".into(),
						DataLiteral::Scalar(Value::Str("probe".into())),
					)])]),
				),
			]),
		});
		let reflected = DataLiteral::to_reflect(
			&literal,
			Some(Host::type_info()),
			&registry,
			&mut resolver,
		)
		.unwrap();
		Host::from_reflect(reflected.as_ref())
			.unwrap()
			.xpect_eq(Host {
				one: Item {
					label: "solo".into(),
					admin: true,
				},
				many: vec![Item {
					label: "probe".into(),
					admin: true,
				}],
			});
	}

	/// An enum's struct variant has no default to complete from, so a literal
	/// that omits a field is an error naming it rather than a value that
	/// quietly resolves to the target's default.
	#[crate::test]
	fn a_partial_enum_variant_is_an_error() {
		#[derive(Reflect, PartialEq, Debug, Default)]
		enum Provider {
			#[default]
			None,
			Zone {
				authority: SmolStr,
				id: SmolStr,
			},
		}
		let complete = DataLiteral::Enum(NamedLiteral {
			name: "Zone".into(),
			fields: NamedFields::Struct(vec![
				(
					"authority".into(),
					DataLiteral::Scalar(Value::Str("beet.org".into())),
				),
				("id".into(), DataLiteral::Scalar(Value::Str("z1".into()))),
			]),
		});
		resolve::<Provider>(complete).xpect_eq(Provider::Zone {
			authority: "beet.org".into(),
			id: "z1".into(),
		});

		let partial = DataLiteral::Enum(NamedLiteral {
			name: "Zone".into(),
			fields: NamedFields::Struct(vec![(
				"authority".into(),
				DataLiteral::Scalar(Value::Str("beet.org".into())),
			)]),
		});
		let registry = TypeRegistry::default();
		let mut resolver = |_: &str| Entity::PLACEHOLDER;
		DataLiteral::to_reflect(
			&partial,
			Some(Provider::type_info()),
			&registry,
			&mut resolver,
		)
		.unwrap_err()
		.to_string()
		.xpect_contains("\"id\"");
	}

	/// A `[a, b, ..]` literal fills an array-typed field (eg an `HttpServer`'s
	/// `host: [u8; 4]` from `{HttpServer{host:[0,0,0,0]}}`), not just a `Vec`.
	#[crate::test]
	fn coerces_list_to_array_field() {
		#[derive(Reflect, PartialEq, Debug)]
		struct Server {
			host: [u8; 4],
			ports: Vec<u16>,
		}
		resolve::<Server>(DataLiteral::Enum(NamedLiteral {
			name: "Server".into(),
			fields: NamedFields::Struct(vec![
				(
					"host".into(),
					DataLiteral::List(vec![
						DataLiteral::Scalar(Value::Int(0)),
						DataLiteral::Scalar(Value::Int(0)),
						DataLiteral::Scalar(Value::Int(0)),
						DataLiteral::Scalar(Value::Int(0)),
					]),
				),
				(
					"ports".into(),
					DataLiteral::List(vec![DataLiteral::Scalar(Value::Int(
						8337,
					))]),
				),
			]),
		}))
		.xpect_eq(Server {
			host: [0, 0, 0, 0],
			ports: vec![8337],
		});
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
	#[crate::test]
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
	#[crate::test]
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
	#[cfg(feature = "std")]
	#[crate::test]
	fn coerces_string_to_abs_path() {
		resolve::<AbsPathBuf>(DataLiteral::Scalar(Value::str("assets")))
			.xpect_eq(WsPathBuf::new("assets").into_abs());
	}

	/// A unit-suffixed string coerces to its duration, so a markup `duration="1s"`
	/// authors an `EndInDuration` delay. The unit is required: a bare number or an
	/// unknown unit does not parse, and a malformed value targeting a `Duration`
	/// field is a hard error rather than a silent miss.
	#[crate::test]
	fn coerces_to_duration() {
		resolve::<Duration>(DataLiteral::Scalar(Value::str("250ms")))
			.xpect_eq(Duration::from_millis(250));
		resolve::<Duration>(DataLiteral::Scalar(Value::str("2s")))
			.xpect_eq(Duration::from_secs(2));
		resolve::<Duration>(DataLiteral::Scalar(Value::str("1h")))
			.xpect_eq(Duration::from_secs(60 * 60));
		resolve::<Duration>(DataLiteral::Scalar(Value::str("7d")))
			.xpect_eq(Duration::from_secs(7 * 24 * 60 * 60));
		// the unit is required
		Duration::from_human_str("50").xpect_none();
		Duration::from_human_str("50years").xpect_none();
		coerce_duration(&Value::Uint(50)).xpect_none();
		// a malformed value targeting a `Duration` field errors, rather than
		// silently falling through to a value that cannot apply
		let registry = TypeRegistry::default();
		let mut resolver = |_: &str| Entity::PLACEHOLDER;
		DataLiteral::to_reflect(
			&DataLiteral::Scalar(Value::Uint(50)),
			Some(Duration::type_info()),
			&registry,
			&mut resolver,
		)
		.is_err()
		.xpect_true();
	}

	/// A `"true"`/`"false"` string attribute coerces to a `bool` field, so a markup
	/// `home="false"` authors a flag; a non-bool string is a hard error.
	#[crate::test]
	fn coerces_to_bool() {
		resolve::<bool>(DataLiteral::Scalar(Value::str("true"))).xpect_eq(true);
		resolve::<bool>(DataLiteral::Scalar(Value::str("false")))
			.xpect_eq(false);
		let registry = TypeRegistry::default();
		let mut resolver = |_: &str| Entity::PLACEHOLDER;
		DataLiteral::to_reflect(
			&DataLiteral::Scalar(Value::str("yes")),
			Some(bool::type_info()),
			&registry,
			&mut resolver,
		)
		.is_err()
		.xpect_true();
	}

	/// A numeric string coerces to a numeric field, the quoted twin of the bare
	/// number, so a markup `<HttpServer port="0"/>` authors an `Option<u16>`
	/// port; a non-numeric string targeting a numeric field is a hard error
	/// rather than a silent miss (`from_reflect` would keep the default).
	#[crate::test]
	fn coerces_string_to_number() {
		resolve::<u16>(DataLiteral::Scalar(Value::str("0"))).xpect_eq(0);
		resolve::<f32>(DataLiteral::Scalar(Value::str("1.5"))).xpect_eq(1.5);
		resolve::<Option<u16>>(DataLiteral::Scalar(Value::str("8080")))
			.xpect_eq(Some(8080));
		#[derive(Reflect, PartialEq, Debug)]
		struct Speed(f32);
		resolve::<Speed>(DataLiteral::Scalar(Value::str("60")))
			.xpect_eq(Speed(60.0));
		let registry = TypeRegistry::default();
		let mut resolver = |_: &str| Entity::PLACEHOLDER;
		DataLiteral::to_reflect(
			&DataLiteral::Scalar(Value::str("nope")),
			Some(u16::type_info()),
			&registry,
			&mut resolver,
		)
		.is_err()
		.xpect_true();
	}

	/// A string names a unit variant of an enum field (`records="IdentityOnly"`),
	/// and a string naming NO variant is an error rather than a fallthrough.
	///
	/// REGRESSION for the second half: the miss fell through to a `String`
	/// patch, `from_reflect` kept the field's default, and a mistyped variant
	/// read as a working declaration — for a `MailRecords` field that is
	/// `All`, ie a cutover-staged mail domain publishing the very records the
	/// stage exists to withhold.
	#[beet_core::test]
	fn a_mistyped_variant_name_errors() {
		#[derive(Debug, Default, PartialEq, Reflect)]
		enum Records {
			#[default]
			All,
			IdentityOnly,
		}
		resolve::<Records>(DataLiteral::Scalar(Value::str("IdentityOnly")))
			.xpect_eq(Records::IdentityOnly);
		let registry = TypeRegistry::default();
		let mut resolver = |_: &str| Entity::PLACEHOLDER;
		DataLiteral::to_reflect(
			&DataLiteral::Scalar(Value::str("identity-only")),
			Some(Records::type_info()),
			&registry,
			&mut resolver,
		)
		.xpect_err();
	}

	/// A hex string coerces to a `Color` field, bare or wrapped in an `Option`, so
	/// a markup `<Theme primary="#006c4f"/>` authors a colour directly. A value
	/// that is not a colour errors rather than silently keeping the default.
	#[cfg(feature = "bevy_color")]
	#[beet_core::test]
	fn coerces_hex_to_color() {
		use bevy::color::Color;
		use bevy::color::Srgba;
		resolve::<Color>(DataLiteral::Scalar(Value::str("#006c4f")))
			.xpect_eq(Color::Srgba(Srgba::hex("#006c4f").unwrap()));
		resolve::<Option<Color>>(DataLiteral::Scalar(Value::str("#f028a8")))
			.xpect_eq(Some(Color::Srgba(Srgba::hex("#f028a8").unwrap())));
		DataLiteral::to_reflect(
			&DataLiteral::Scalar(Value::str("beetroot")),
			Some(Color::type_info()),
			&TypeRegistry::default(),
			&mut |_: &str| Entity::PLACEHOLDER,
		)
		.is_err()
		.xpect_true();
	}

	/// A string coerces to a `Cow<'static, str>` field, so a tuple literal carrying
	/// a string (eg `<Log::Message("hi")/>`, whose variant field is a `Cow`)
	/// reflect-applies instead of panicking on the `String`->`Cow` mismatch.
	#[crate::test]
	fn coerces_to_cow_str() {
		resolve::<alloc::borrow::Cow<'static, str>>(DataLiteral::Scalar(
			Value::str("hi"),
		))
		.xpect_eq(alloc::borrow::Cow::Borrowed("hi"));
	}

	#[crate::test]
	fn wraps_scalar_into_option() {
		resolve::<Option<String>>(DataLiteral::Scalar(Value::str("beet")))
			.xpect_eq(Some("beet".to_string()));
		resolve::<Option<u32>>(DataLiteral::Scalar(Value::Uint(7)))
			.xpect_eq(Some(7));
	}

	#[crate::test]
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
	#[crate::test]
	fn qualified_unit_variant_resolves() {
		resolve::<Emphasis>(DataLiteral::Enum(NamedLiteral {
			name: "Emphasis::High".into(),
			fields: NamedFields::Unit,
		}))
		.xpect_eq(Emphasis::High);
	}
}
