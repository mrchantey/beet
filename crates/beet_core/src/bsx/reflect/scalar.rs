//! Scalar coercion: a scalar [`Value`] to the target field's concrete type.
//!
//! A number casts to the field's registered scalar id, a string lands in
//! whichever string type the field names, and the domain coercions a bare
//! string reaches (a [`Duration`], a [`GlobFilter`], a [`ValueSchema`]) resolve
//! here too, so the one markup spelling of each stays in one place.

use crate::prelude::*;
use bevy::reflect::PartialReflect;
use bevy::reflect::TypeInfo;
use bevy::reflect::enums::DynamicEnum;
use bevy::reflect::enums::DynamicVariant;
use bevy::reflect::enums::VariantInfo;
use bevy::reflect::structs::DynamicStruct;
use bevy::reflect::tuple_struct::DynamicTupleStruct;
use core::any::TypeId;
use core::time::Duration;

/// Coerce a scalar [`Value`] to the field's concrete type, falling through to
/// its natural reflect type when there is no field info to coerce against.
pub(super) fn scalar_to_reflect(
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

	// a `YYYY-MM-DD` string targeting a `Timestamp` field coerces to midnight UTC
	// on that date, so a markup `{PageMeta{created:"2026-08-28"}}` authors a
	// publication date directly (mirroring the duration string above). Any other
	// string errors rather than silently landing on the epoch.
	if let (Value::Str(string), Some(info)) = (value, field_info)
		&& info.type_id() == TypeId::of::<Timestamp>()
	{
		let Some(timestamp) = Timestamp::parse_date(string) else {
			bevybail!(
				"invalid date {string:?}: expected a `YYYY-MM-DD` string like \"2026-08-28\""
			);
		};
		return Ok(Box::new(timestamp));
	}

	// a non-numeric string targeting a numeric field errors rather than silently
	// falling through to `String` (whose `from_reflect` miss would keep the
	// target's default, eg a `port="nope"` leaving the default port). Runs after
	// the domain coercions above because a domain newtype is numeric-shaped:
	// `Timestamp` wraps an `i64`, and its date string is not a number.
	if let (Value::Str(string), None, Some(info)) = (value, as_f64, field_info)
		&& is_numeric_target(info)
	{
		bevybail!(
			"invalid number {string:?}: expected a numeric string for a `{}` field",
			info.type_path()
		);
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

	// a string targeting a `GlobPattern` builds one validated pattern, the item
	// form of the filter coercions: a `GlobFilter` struct literal writes its
	// lists as plain strings, `{filter:{exclude:["blog/**"]}}`.
	if let (Value::Str(pattern), Some(info)) = (value, field_info)
		&& info.type_id() == TypeId::of::<GlobPattern>()
	{
		return glob_pattern(pattern.as_str())
			.map(|pattern| Box::new(pattern) as Box<dyn PartialReflect>);
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
pub(super) fn coerce_duration(value: &Value) -> Option<Duration> {
	match value {
		Value::Str(string) => Duration::from_human_str(string.as_str()),
		_ => None,
	}
}

/// A [`GlobFilter`] over `patterns`, as includes.
///
/// The markup form of a filter is the allowlist a human writes; an exclude
/// needs the struct literal (`{read:{exclude:[..]}}`), which reflects normally
/// now that each pattern coerces from its string.
pub(super) fn glob_filter<'a>(
	patterns: impl IntoIterator<Item = &'a str>,
) -> Result<GlobFilter> {
	let mut filter = GlobFilter::default();
	for pattern in patterns {
		glob_pattern(pattern)?;
		filter.include(pattern);
	}
	filter.xok()
}

/// One validated [`GlobPattern`].
///
/// `GlobPattern::new` panics on a malformed pattern, and a markup attribute is
/// authored input, so it is validated into an error first.
fn glob_pattern(pattern: &str) -> Result<GlobPattern> {
	GlobFilter::parse_glob_pattern(pattern)
		.map_err(|err| bevyhow!("invalid glob pattern {pattern:?}: {err}"))?;
	GlobPattern::new(pattern).xok()
}

/// A [`ValueSchema`] from the one word a human means, or from a JSON Schema.
///
/// The markup form of a schema is its kind: `schema="u64"`. A shape with more
/// to say than a kind is written as a JSON Schema object, which is the schema
/// language a document author is likeliest to already know, and anything
/// richer than that is a rust expression.
pub(super) fn value_schema(source: &str) -> Result<ValueSchema> {
	let source = source.trim();
	if source.starts_with('{') {
		return json_schema(source);
	}
	// unlike a JSON Schema descriptor, an unrecognized word here is an authoring
	// typo rather than a reference to a schema by name, so it is named as one.
	ValueSchema::primitive_by_name(source).ok_or_else(|| {
		bevyhow!(
			"unknown schema {source:?}: expected one of {}, or a JSON Schema object",
			ValueSchema::PRIMITIVE_NAMES
		)
	})
}

/// A [`ValueSchema`] from one JSON Schema descriptor.
///
/// Not [`ValueSchema::from_json_schema`], which reads its top level as a
/// template's *prop block*: a component's schema is one descriptor, so
/// `{"type":"integer"}` must mean an integer rather than a prop named `type`.
#[cfg(feature = "json")]
pub(super) fn json_schema(source: &str) -> Result<ValueSchema> {
	ValueSchema::from_json_value(&serde_json::from_str(source)?)
}

/// Parsing a JSON schema requires the `json` feature.
#[cfg(not(feature = "json"))]
pub(super) fn json_schema(_source: &str) -> Result<ValueSchema> {
	bevybail!("parsing a JSON schema requires the `json` feature")
}
