//! [`MultiMapReflectExt`]: a string-keyed [`MultiMap`] parsed into a reflected
//! type, the typed read behind every request param.
//!
//! The walk over `T`'s [`TypeInfo`] is the one a route's `--help` documents,
//! so what the help says a flag is, the parse enforces:
//!
//! - a field name is its kebab-case key: `help_format` reads `--help-format`,
//!   and a snake_case key matches too;
//! - `bool` is a flag, present (bare, or `true`/`1`/`yes`/`on`) or absent
//!   (`false`); an explicit `false`/`0`/`no`/`off` is accepted too;
//! - `Option<T>` is optional, `None` when its key is absent, so an unset knob is
//!   distinguishable from one set to the type's default (`--port=0` is
//!   `Some(0)`, no `--port` at all is `None`);
//! - `Vec<T>` collects every value under its key, `--tag=a --tag=b` is
//!   `["a", "b"]`, and is empty when absent;
//! - every other field is required, and an absent one errors naming its flag;
//! - a present value parses through its type's [`LiteralParser`] entry, so a
//!   type authors identically here and in markup: `Duration`, `Timestamp`,
//!   `StoreUri`, `GlobFilter`, and whatever a downstream crate registers;
//! - a single-field newtype is transparent and reads its parent's key; a
//!   nested multi-field struct is flattened into the parent's namespace.
//!
//! The walk supplies every field, so `T` needs no `Default`, and a field whose
//! type has no authored form is an error naming the type, never a silent
//! default.
//!
//! ```
//! # use beet_core::prelude::*;
//! #[derive(Debug, PartialEq, Reflect)]
//! struct Params {
//! 	name: String,
//! 	verbose: bool,
//! 	tags: Vec<String>,
//! 	limit: Option<u32>,
//! }
//!
//! let params = CliArgs::parse("--name=x --verbose --tags=a --tags=b")
//! 	.params
//! 	.parse_reflect::<Params>()
//! 	.unwrap();
//! assert_eq!(params, Params {
//! 	name: "x".into(),
//! 	verbose: true,
//! 	tags: vec!["a".into(), "b".into()],
//! 	limit: None,
//! });
//! ```

use crate::prelude::*;
use bevy::reflect::FromReflect;
use bevy::reflect::PartialReflect;
use bevy::reflect::TypeInfo;
use bevy::reflect::Typed;
use bevy::reflect::enums::DynamicEnum;
use bevy::reflect::enums::DynamicVariant;
use bevy::reflect::list::DynamicList;
use bevy::reflect::structs::DynamicStruct;
use bevy::reflect::structs::StructInfo;
use bevy::reflect::tuple::DynamicTuple;
use bevy::reflect::tuple::TupleInfo;
use bevy::reflect::tuple_struct::DynamicTupleStruct;
use bevy::reflect::tuple_struct::TupleStructInfo;
use core::any::TypeId;
use core::hash::Hash;
use heck::ToKebabCase;
use heck::ToSnakeCase;

/// Parsing a string-keyed [`MultiMap`] into a reflected type, per the
/// [module](self) contract.
///
/// Implemented for any key/value that borrows as `str`, ie both
/// `MultiMap<String, String>` and `MultiMap<SmolStr, SmolStr>`.
#[extend::ext(name=MultiMapReflectExt)]
pub impl<K: AsRef<str> + Eq + Hash, V: AsRef<str>> MultiMap<K, V> {
	/// Build a whole `T` from the map: an absent optional field takes its
	/// empty form and an absent required one is an error naming its flag.
	fn parse_reflect<T: FromReflect + Typed>(&self) -> Result<T> {
		let normalized = normalize_snake_case(self);
		let type_info = T::type_info();
		let dynamic = build_dynamic_from_type_info(
			&normalized,
			type_info,
			None,
			Absent::Complete,
		)?
		.ok_or_else(|| {
			bevyhow!("cannot build `{}` from params", type_info.type_path())
		})?;
		T::from_reflect(dynamic.as_partial_reflect()).ok_or_else(|| {
			bevyhow!(
				"failed to convert dynamic type to {}",
				type_info.type_path()
			)
		})
	}

	/// Apply only the fields present in the map over `base`, leaving absent
	/// ones untouched: the presence-aware sibling of
	/// [`parse_reflect`](Self::parse_reflect), for a value with its own
	/// declared defaults, ie a markup-authored component that request params
	/// override field-by-field. Keys normalize identically.
	fn apply_reflect<T: Reflect + Typed>(&self, base: &mut T) -> Result {
		let normalized = normalize_snake_case(self);
		if let Some(dynamic) = build_dynamic_from_type_info(
			&normalized,
			T::type_info(),
			None,
			Absent::Skip,
		)? {
			base.try_apply(dynamic.as_partial_reflect())?;
		}
		Ok(())
	}
}

/// What the walk does with a field whose key is absent from the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Absent {
	/// The whole value is being built: an optional kind takes its empty form
	/// (`false`, `None`, `[]`) and a required one errors naming its flag.
	Complete,
	/// The field is left out of the dynamic, so the base it applies over keeps
	/// its own value.
	Skip,
}

/// Normalize kebab-case keys to snake_case for reflection lookup, preserving
/// empty value lists (flags with no value).
fn normalize_snake_case<K: AsRef<str> + Eq + Hash, V: AsRef<str>>(
	map: &MultiMap<K, V>,
) -> MultiMap<String, String> {
	let mut normalized = MultiMap::<String, String>::new();
	for (key, values) in map.iter_all() {
		let snake_key = key.as_ref().to_snake_case();
		if values.is_empty() {
			normalized.insert_key(snake_key);
		} else {
			for value in values {
				normalized
					.insert(snake_key.clone(), value.as_ref().to_string());
			}
		}
	}
	normalized
}

/// Build a dynamic value for `type_info`, a struct, a newtype under
/// `field_prefix`, or a tuple.
fn build_dynamic_from_type_info(
	map: &MultiMap<String, String>,
	type_info: &TypeInfo,
	field_prefix: Option<&str>,
	absent: Absent,
) -> Result<Option<Box<dyn PartialReflect>>> {
	match type_info {
		TypeInfo::Struct(info) => build_dynamic_struct(map, info, absent),
		TypeInfo::TupleStruct(info) => {
			build_dynamic_tuple_struct(map, info, field_prefix, absent)
		}
		TypeInfo::Tuple(info) => {
			build_dynamic_tuple(map, info, field_prefix, absent)
		}
		other => {
			bevybail!(
				"unsupported type kind for params: {}\nSupported types are Struct, TupleStruct and Tuple",
				other.kind()
			)
		}
	}
}

/// A struct, each field under its own key. Always `Some`, empty under
/// [`Absent::Skip`] when nothing matched.
fn build_dynamic_struct(
	map: &MultiMap<String, String>,
	info: &StructInfo,
	absent: Absent,
) -> Result<Option<Box<dyn PartialReflect>>> {
	let mut dynamic = DynamicStruct::default();
	for field in info.iter() {
		if let Some(value) = build_field_value(
			map,
			field.name(),
			field.type_id(),
			field.type_info(),
			absent,
		)? {
			dynamic.insert_boxed(field.name(), value);
		}
	}
	Ok(Some(Box::new(dynamic)))
}

/// A single-field newtype, transparent over its inner value under the parent's
/// key. `None` under [`Absent::Skip`] when that key is absent.
fn build_dynamic_tuple_struct(
	map: &MultiMap<String, String>,
	info: &TupleStructInfo,
	field_prefix: Option<&str>,
	absent: Absent,
) -> Result<Option<Box<dyn PartialReflect>>> {
	if info.field_len() != 1 {
		bevybail!(
			"multi-field tuple struct `{}` has no authored form",
			info.type_path()
		);
	}
	let Some(prefix) = field_prefix else {
		bevybail!("top level tuple structs not supported");
	};
	let field = info
		.field_at(0)
		.ok_or_else(|| bevyhow!("tuple struct field at index 0 not found"))?;
	build_field_value(map, prefix, field.type_id(), field.type_info(), absent)?
		.map(|value| {
			let mut dynamic = DynamicTupleStruct::default();
			dynamic.insert_boxed(value);
			Box::new(dynamic) as Box<dyn PartialReflect>
		})
		.xok()
}

/// A tuple, each element under its index as a key (or all under the prefix).
/// An element cannot be skipped without shifting the rest, so an absent one is
/// an error even under [`Absent::Skip`].
fn build_dynamic_tuple(
	map: &MultiMap<String, String>,
	info: &TupleInfo,
	field_prefix: Option<&str>,
	absent: Absent,
) -> Result<Option<Box<dyn PartialReflect>>> {
	let mut dynamic = DynamicTuple::default();
	for (index, field) in info.iter().enumerate() {
		let field_name = field_prefix
			.map(str::to_string)
			.unwrap_or_else(|| index.to_string());
		let value = build_field_value(
			map,
			&field_name,
			field.type_id(),
			field.type_info(),
			absent,
		)?
		.ok_or_else(|| {
			bevyhow!("missing required param `--{field_name}` for tuple")
		})?;
		dynamic.insert_boxed(value);
	}
	Ok(Some(Box::new(dynamic)))
}

/// One field's value, per the [module](self) contract: `Some` when the field
/// is present or takes its empty form, `None` when it is left out under
/// [`Absent::Skip`].
fn build_field_value(
	map: &MultiMap<String, String>,
	field_name: &str,
	field_type_id: TypeId,
	field_type_info: Option<&TypeInfo>,
	absent: Absent,
) -> Result<Option<Box<dyn PartialReflect>>> {
	// an `Option<T>` field is authored exactly as its `T`, its optionality
	// being whether the key is there at all
	let option_inner = field_type_info.and_then(reflect_ext::option_some_inner);
	let (leaf_id, leaf_info) = match option_inner {
		Some(inner) => (inner.type_id(), Some(inner)),
		None => (field_type_id, field_type_info),
	};
	let wrap = |value: Box<dyn PartialReflect>| match option_inner {
		Some(_) => wrap_some(value),
		None => value,
	};

	// a flag is present or absent, not parsed: a bare `--watch` is `true`
	if leaf_id == TypeId::of::<bool>()
		&& let Some(flag) = parse_bool_field(map, field_name)?
	{
		return Ok(Some(wrap(Box::new(flag))));
	}

	// the field type's own authored spelling, over the values under its key
	// (one value a `Value::Str`, a repeated key a `Value::List`). A parser
	// that declines, or a type with no entry, falls to the structural rules.
	if let Some(values) = map.get_vec(field_name) {
		if let Some(parsed) = parse_leaf_literal(values, leaf_id)? {
			return Ok(Some(wrap(parsed)));
		}
		// a list-typed field takes EVERY value under its key, each parsed into
		// the item type, so `--tag a --tag b` is `["a","b"]`
		if let Some(TypeInfo::List(list_info)) = leaf_info
			&& let Some(item_info) = list_info.item_info()
		{
			return parse_list_field(values, field_name, item_info)
				.map(wrap)
				.map(Some);
		}
	}

	// a struct or newtype has no key of its own: a newtype reads its parent's
	// key, a struct's fields are flattened into the parent's namespace
	if option_inner.is_none()
		&& let Some(info) = leaf_info
		&& matches!(
			info,
			TypeInfo::Struct(_) | TypeInfo::TupleStruct(_) | TypeInfo::Tuple(_)
		) {
		return build_dynamic_from_type_info(
			map,
			info,
			Some(field_name),
			absent,
		);
	}

	// present, but nothing above knew how to read it: a bare key on anything
	// but a flag wants its value, a valued key a parser for its type
	if let Some(values) = map.get_vec(field_name) {
		return match values.is_empty() {
			true => missing_value(field_name, leaf_info),
			false => unsupported_field(field_name, leaf_info),
		};
	}
	// absent: the empty form of an optional kind, or the missing flag
	match absent {
		Absent::Skip => Ok(None),
		Absent::Complete if option_inner.is_some() => Ok(Some(Box::new(
			DynamicEnum::new("None", DynamicVariant::Unit),
		))),
		Absent::Complete if leaf_id == TypeId::of::<bool>() => {
			Ok(Some(Box::new(false)))
		}
		Absent::Complete if matches!(leaf_info, Some(TypeInfo::List(_))) => {
			Ok(Some(Box::new(DynamicList::default())))
		}
		Absent::Complete => bevybail!(
			"missing required param `--{}`",
			field_name.to_kebab_case()
		),
	}
}

/// Parse the values under one key through `type_id`'s [`LiteralParser`], per
/// the seam contract: one value is a [`Value::Str`], a repeated key a
/// [`Value::List`], and a type with no entry declines to the structural rules.
fn parse_leaf_literal(
	values: &[String],
	type_id: TypeId,
) -> Result<Option<Box<dyn PartialReflect>>> {
	match values {
		[] => Ok(None),
		[one] => LiteralParser::parse_type(type_id, &Value::str(one.as_str())),
		many => LiteralParser::parse_type(
			type_id,
			&Value::List(
				many.iter()
					.map(|value| Value::str(value.as_str()))
					.collect(),
			),
		),
	}
}

/// Every value under a list field's key, each parsed as one item: through the
/// item type's [`LiteralParser`], or for a newtype item through its inner
/// type.
fn parse_list_field(
	values: &[String],
	field_name: &str,
	item_info: &TypeInfo,
) -> Result<Box<dyn PartialReflect>> {
	let mut list = DynamicList::default();
	for value in values {
		let mut item_map = MultiMap::<String, String>::new();
		item_map.insert(field_name.to_string(), value.clone());
		let item = build_field_value(
			&item_map,
			field_name,
			item_info.type_id(),
			Some(item_info),
			Absent::Complete,
		)?
		.ok_or_else(|| bevyhow!("missing list item"))?;
		list.push_box(item);
	}
	Ok(Box::new(list))
}

/// The error a bare `--key` on a non-flag field reports, naming the value it
/// wants: `--generate` on an `Option<usize>` is `--generate=<usize>`, never
/// "unsupported type".
fn missing_value<T>(
	field_name: &str,
	type_info: Option<&TypeInfo>,
) -> Result<T> {
	let kind = type_info
		.map(|info| info.type_path_table().short_path())
		.unwrap_or("value");
	let key = field_name.to_kebab_case();
	bevybail!("--{key} needs a value, ie --{key}=<{kind}>")
}

/// The error a field with no authored form reports, naming the type so the
/// remedy (a [`LiteralParser`] for it) is obvious.
fn unsupported_field<T>(
	field_name: &str,
	type_info: Option<&TypeInfo>,
) -> Result<T> {
	let type_path = type_info
		.map(TypeInfo::type_path)
		.unwrap_or("an untyped field");
	bevybail!(
		"unsupported type for `--{}`: no authored form for `{type_path}`, expected a bool, a nested struct, or a type carrying a `LiteralParser`",
		field_name.to_kebab_case()
	)
}

/// Wrap a parsed leaf in an `Option`'s `Some` variant.
fn wrap_some(value: Box<dyn PartialReflect>) -> Box<dyn PartialReflect> {
	let mut tuple = DynamicTuple::default();
	tuple.insert_boxed(value);
	Box::new(DynamicEnum::new("Some", DynamicVariant::Tuple(tuple)))
}

/// A flag's value: `None` when its key is absent, `true` for a bare key, else
/// the spelled `true`/`1`/`yes`/`on` or `false`/`0`/`no`/`off`.
fn parse_bool_field(
	map: &MultiMap<String, String>,
	field_name: &str,
) -> Result<Option<bool>> {
	match map.get_vec(field_name) {
		None => Ok(None),
		Some(values) if values.is_empty() => Ok(Some(true)),
		Some(values) => match values[0].to_lowercase().as_str() {
			"true" | "1" | "yes" | "on" | "" => Ok(Some(true)),
			"false" | "0" | "no" | "off" => Ok(Some(false)),
			other => bevybail!(
				"invalid bool for `--{}`: '{other}', expected true/false",
				field_name.to_kebab_case()
			),
		},
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// Parse `args` (a CLI string) into `T`.
	fn parse<T: FromReflect + Typed>(args: &str) -> Result<T> {
		CliArgs::parse(args).params.parse_reflect()
	}

	/// The message parsing `args` into `T` fails with.
	fn message<T: FromReflect + Typed>(args: &str) -> String {
		match parse::<T>(args) {
			Ok(_) => panic!("expected an error"),
			Err(err) => err.to_string(),
		}
	}

	#[derive(Debug, PartialEq, Reflect)]
	struct Params {
		name: String,
		verbose: bool,
		tags: Vec<String>,
		limit: Option<u32>,
	}

	/// Each kind reads its argv shape, and an absent optional kind takes its
	/// empty form with no `Default` on the type.
	#[crate::test]
	fn parses_each_kind() {
		parse::<Params>("--name=x --verbose --tags=a --tags=b --limit=3")
			.unwrap()
			.xpect_eq(Params {
				name: "x".into(),
				verbose: true,
				tags: vec!["a".into(), "b".into()],
				limit: Some(3),
			});
		parse::<Params>("--name=x").unwrap().xpect_eq(Params {
			name: "x".into(),
			verbose: false,
			tags: Vec::new(),
			limit: None,
		});
	}

	/// A required field is one that is neither a flag, optional nor a list,
	/// and its absence names the flag the user forgot.
	#[crate::test]
	fn missing_required_names_the_flag() {
		#[derive(Debug, Reflect)]
		#[allow(dead_code)]
		struct Out {
			out_dir: String,
		}
		message::<Out>("").xpect_contains("missing required param `--out-dir`");
	}

	/// A flag is spelled every way a shell user writes one, and anything else
	/// is an error rather than `false`.
	#[crate::test]
	fn flag_spellings() {
		#[derive(Debug, Reflect)]
		struct Flag {
			watch: bool,
		}
		for spelling in ["--watch", "--watch=true", "--watch=1", "--watch=on"] {
			parse::<Flag>(spelling).unwrap().watch.xpect_true();
		}
		for spelling in ["--watch=false", "--watch=0", "--watch=off"] {
			parse::<Flag>(spelling).unwrap().watch.xpect_false();
		}
		message::<Flag>("--watch=maybe").xpect_contains("invalid bool");
	}

	/// An `Option<T>` leaf separates "unset" from "set to the type's default":
	/// `--port=0` (an OS-assigned port) is a selection, no `--port` is not. A
	/// bare `--tls` keeps the flag shape through the `Option`.
	#[crate::test]
	fn optional_leaves() {
		#[derive(Debug, PartialEq, Reflect)]
		struct Knobs {
			port: Option<u16>,
			tls: Option<bool>,
		}
		parse::<Knobs>("--port=0 --tls").unwrap().xpect_eq(Knobs {
			port: Some(0),
			tls: Some(true),
		});
		parse::<Knobs>("").unwrap().xpect_eq(Knobs {
			port: None,
			tls: None,
		});
		// malformed errors exactly as the unwrapped leaf does
		message::<Knobs>("--port=nope").xpect_contains("invalid number");
	}

	/// A leaf parses through its type's `LiteralParser`, so a domain type
	/// authors here exactly as in markup and a malformed value is an error.
	#[crate::test]
	fn leaves_parse_through_the_literal_table() {
		#[derive(Debug, PartialEq, Reflect)]
		struct Domain {
			timeout: Duration,
			store: Option<StoreUri>,
		}
		parse::<Domain>("--timeout=30s --store=s3://bucket")
			.unwrap()
			.xpect_eq(Domain {
				timeout: Duration::from_secs(30),
				store: Some(StoreUri::parse("s3://bucket").unwrap()),
			});
		message::<Domain>("--timeout=30").xpect_contains("invalid duration");
	}

	/// Keys match kebab-case (the flag) and snake_case (the field) alike.
	#[crate::test]
	fn kebab_and_snake_keys() {
		#[derive(Debug, Reflect)]
		struct Kebab {
			max_retry_count: u32,
		}
		parse::<Kebab>("--max-retry-count=5")
			.unwrap()
			.max_retry_count
			.xpect_eq(5);
		parse::<Kebab>("--max_retry_count=5")
			.unwrap()
			.max_retry_count
			.xpect_eq(5);
	}

	#[derive(Debug, PartialEq, Reflect)]
	struct Database {
		host: String,
		port: Option<u16>,
	}

	#[derive(Debug, PartialEq, Reflect)]
	struct AppConfig {
		app_name: String,
		database: Database,
	}

	/// A nested struct's fields are flattened into the parent's namespace, and
	/// its required fields are as required as the parent's.
	#[crate::test]
	fn nested_structs_flatten() {
		parse::<AppConfig>("--app-name=app --host=db")
			.unwrap()
			.xpect_eq(AppConfig {
				app_name: "app".into(),
				database: Database {
					host: "db".into(),
					port: None,
				},
			});
		message::<AppConfig>("--app-name=app")
			.xpect_contains("missing required param `--host`");
	}

	#[derive(Debug, PartialEq, Reflect)]
	struct Label(String);

	#[derive(Debug, PartialEq, Reflect)]
	struct Enabled(bool);

	#[derive(Debug, PartialEq, Reflect)]
	struct Newtypes {
		label: Label,
		enabled: Enabled,
		labels: Vec<Label>,
	}

	/// A single-field newtype is transparent, reading its parent's key, alone
	/// or as a list item.
	#[crate::test]
	fn newtypes_are_transparent() {
		parse::<Newtypes>("--label=x --enabled --labels=a --labels=b")
			.unwrap()
			.xpect_eq(Newtypes {
				label: Label("x".into()),
				enabled: Enabled(true),
				labels: vec![Label("a".into()), Label("b".into())],
			});
		message::<Newtypes>("").xpect_contains("`--label`");
	}

	/// A tuple reads each element under its index.
	#[crate::test]
	fn parses_tuple() {
		parse::<(String, u32)>("--0=first --1=2")
			.unwrap()
			.xpect_eq(("first".to_string(), 2));
	}

	/// A field whose type has no authored form names that type, never a
	/// silent default.
	#[crate::test]
	fn unsupported_type_names_itself() {
		#[derive(Debug, Reflect)]
		#[allow(dead_code)]
		struct Opaque;
		#[derive(Debug, Reflect)]
		#[allow(dead_code)]
		struct Holder {
			value: Option<Opaque>,
		}
		message::<Holder>("--value=x")
			.xpect_contains("unsupported type for `--value`")
			.xpect_contains("Opaque");
	}

	/// A bare key on anything but a flag says it needs a value, naming the
	/// flag and the type it wants, rather than calling the type unsupported.
	#[crate::test]
	fn bare_key_names_the_missing_value() {
		#[derive(Debug, Reflect)]
		#[allow(dead_code)]
		struct Mint {
			generate: Option<usize>,
			note: String,
		}
		message::<Mint>("--generate --note=x")
			.xpect_contains("--generate needs a value, ie --generate=<usize>");
		message::<Mint>("--note")
			.xpect_contains("--note needs a value, ie --note=<String>");
	}

	/// `apply_reflect` writes only the present fields over a base with its own
	/// declared values, so an absent required field is no error there.
	#[crate::test]
	fn apply_reflect_leaves_absent_fields() {
		#[derive(Debug, Reflect)]
		struct Config {
			host: String,
			port: u16,
			enabled: bool,
		}
		let mut base = Config {
			host: "preset".to_string(),
			port: 8080,
			enabled: true,
		};
		CliArgs::parse("--host=localhost")
			.params
			.apply_reflect(&mut base)
			.unwrap();
		base.host.xpect_eq("localhost".to_string());
		base.port.xpect_eq(8080);
		base.enabled.xpect_true();
	}
}
