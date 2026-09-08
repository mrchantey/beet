//! Scalar coercion: a scalar [`Value`] to the target field's concrete type.
//!
//! The domain spellings a bare string reaches (a [`Duration`](core::time::Duration),
//! a [`GlobFilter`], a hex [`Color`](bevy::color::Color)) are not here: each is
//! one [`LiteralParser`] entry, looked up by the target's [`TypeId`] per that
//! type's seam contract, so a downstream crate's type authors identically with
//! no arm added here.
//!
//! What stays are the SHAPE-keyed rules, which cannot be expressed per type: a
//! scalar into a single-field newtype, a string into a one-string-field struct,
//! and a string naming a unit enum variant.

use crate::prelude::*;
use bevy::reflect::PartialReflect;
use bevy::reflect::TypeInfo;
use bevy::reflect::enums::DynamicEnum;
use bevy::reflect::enums::DynamicVariant;
use bevy::reflect::enums::EnumInfo;
use bevy::reflect::enums::VariantInfo;
use bevy::reflect::structs::DynamicStruct;
use bevy::reflect::tuple_struct::DynamicTupleStruct;
use core::any::TypeId;

/// Coerce a scalar [`Value`] to the field's concrete type, falling through to
/// its natural reflect type when there is no field info to coerce against.
pub(super) fn scalar_to_reflect(
	value: &Value,
	field_info: Option<&'static TypeInfo>,
) -> Result<Box<dyn PartialReflect>> {
	// the field type's own authored spelling, the one table lookup: `Ok(Some)`
	// is done, `Err` is a real authoring error (a malformed glob, a unit-less
	// duration), `Ok(None)` declines to the shape rules below.
	if let Some(info) = field_info
		&& let Some(reflected) =
			LiteralParser::parse_type(info.type_id(), value)?
	{
		return Ok(reflected);
	}

	// a scalar targeting a single-field tuple-struct newtype (`LinearVelocity(f32)`)
	// parses into the INNER type, so `<SetDrive linear=60>` authors a typed
	// velocity from a plain attribute. A non-numeric string targeting a numeric
	// newtype errors here rather than silently falling through to a `String`
	// whose `from_reflect` miss would keep the target's default.
	if let Some(TypeInfo::TupleStruct(info)) = field_info
		&& info.field_len() == 1
		&& let Some(field) = info.field_at(0)
		&& let Some(inner) = LiteralParser::parse_type(field.type_id(), value)?
	{
		let mut dynamic = DynamicTupleStruct::default();
		dynamic.insert_boxed(inner);
		dynamic.set_represented_type(field_info);
		return Ok(Box::new(dynamic));
	}

	// a string targeting a one-string-field struct builds that struct from the
	// string, so a LABEL REFERENCE authors as the label it is: `<EnsureSecret
	// secret="db-password"/>`. A type like `SecretRef` exists so one composition
	// owns a name both ends of a reference compose, and wrapping a label is all
	// it does; without this the string patch misses, `from_reflect` keeps the
	// default, and a block silently points at the empty label. The named twin of
	// the newtype cast above.
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

	// a string targeting an enum field coerces to that unit variant, so a markup
	// attribute `kind="User"` resolves to `ActorKind::User` (the quoted twin of
	// the `{Foo{kind:User}}` spread's bare-variant form).
	if let (Value::Str(string), Some(TypeInfo::Enum(enum_info))) =
		(value, field_info)
	{
		if let Some(variant) = unit_variant(enum_info, string.as_str())? {
			return Ok(Box::new(DynamicEnum::new(
				variant,
				DynamicVariant::Unit,
			)));
		}
	}

	// a string targeting an enum field that names no unit variant errors rather
	// than falling through to `String` (whose `from_reflect` miss would keep
	// the field's DEFAULT, so a mistyped variant name would read as a working
	// declaration — eg a `records="identity-only"` leaving `MailRecords::All`
	// in place, which on a cutover-staged mail domain is the difference
	// between proving an identity and taking the domain's mail). `Option` and
	// `Cow` targets never reach here: both are unwrapped earlier.
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

	// a string that reached a struct, tuple-struct or opaque target without
	// coercing has no way to become one: the `String` fallthrough below would
	// miss in `from_reflect` and leave the target's DEFAULT in place, so the
	// authoring mistake would read as a working declaration. Naming the type is
	// also what keeps registration honest: an app whose registry names no
	// parser for the type fails here rather than silently defaulting the field.
	if let (Value::Str(string), Some(info)) = (value, field_info)
		&& matches!(
			info,
			TypeInfo::Struct(_)
				| TypeInfo::TupleStruct(_)
				| TypeInfo::Opaque(_)
		) {
		bevybail!(
			"cannot author `{}` from the string {string:?}: it has no literal parser (see `LiteralParser`) and no field it could name",
			info.type_path()
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

/// The unit variant of `enum_info` that `name` spells, or `None` when it spells
/// none.
///
/// An exact match wins outright, so a lookup costs one probe and an enum whose
/// variants differ only by case still resolves each one exactly. Failing that,
/// the match is case-insensitive: a markup attribute is prose, where
/// `visibility="draft"` is what a human writes, while the Rust variant is
/// `Draft`.
///
/// # Errors
/// Errors when a case-insensitive spelling matches more than one variant. There
/// is no right answer there, and quietly picking one would make the declaration
/// mean whichever variant happened to be declared first.
fn unit_variant<'a>(
	enum_info: &'a EnumInfo,
	name: &str,
) -> Result<Option<&'a str>> {
	let is_unit =
		|variant: &&VariantInfo| matches!(variant, VariantInfo::Unit(_));
	if let Some(variant) = enum_info.variant(name)
		&& is_unit(&variant)
	{
		return Ok(Some(variant.name()));
	}
	let matches: Vec<&str> = enum_info
		.iter()
		.filter(is_unit)
		.map(|variant| variant.name())
		.filter(|variant| variant.eq_ignore_ascii_case(name))
		.collect();
	match matches.as_slice() {
		[] => Ok(None),
		[variant] => Ok(Some(variant)),
		ambiguous => bevybail!(
			"`{name}` matches {} variants of `{}` case-insensitively: {}; spell one exactly",
			ambiguous.len(),
			enum_info.type_path(),
			ambiguous.join(", ")
		),
	}
}
