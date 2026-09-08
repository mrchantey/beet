//! The [`LiteralParser`] entries beet owns, seeded into the process table.
//!
//! Each entry is the one place a type's authored spelling lives: what shapes it
//! accepts, what it rejects, and what it declines to the structural path.

use crate::prelude::*;
use bevy::reflect::PartialReflect;
use bevy::reflect::TypePath;
use core::any::TypeId;
use core::time::Duration;

/// The entries beet owns, built once for [`LiteralParser`]'s table.
///
/// Built directly rather than through [`LiteralParser::register`], which would
/// re-enter the table it is initializing.
pub(super) fn builtins() -> HashMap<TypeId, LiteralParser> {
	let mut table = Table::default();
	add_numbers(&mut table);
	add_strings(&mut table);
	add_domain(&mut table);
	table.0
}

/// The table under construction, so each entry reads as the type it is for and
/// the parser that spells it.
#[derive(Default)]
struct Table(HashMap<TypeId, LiteralParser>);

impl Table {
	/// `T` parses through `parse`.
	fn add<T: 'static + Send + Sync + PartialReflect>(
		&mut self,
		parse: impl 'static + Send + Sync + Fn(&Value) -> Result<Option<T>>,
	) -> &mut Self {
		self.0.insert(TypeId::of::<T>(), LiteralParser::new(parse));
		self
	}

	/// `T` parses through `parse`, and is WRITTEN as `hint` says, for a surface
	/// that names the value rather than showing it (see
	/// [`LiteralParser::with_hint`]).
	fn add_hinted<T: 'static + Send + Sync + PartialReflect>(
		&mut self,
		hint: &'static str,
		parse: impl 'static + Send + Sync + Fn(&Value) -> Result<Option<T>>,
	) -> &mut Self {
		self.0.insert(
			TypeId::of::<T>(),
			LiteralParser::new(parse).with_hint(hint),
		);
		self
	}

	/// `T` parses from a string and declines every other shape, the common form
	/// of a type spelled as one word.
	fn add_str<T: 'static + Send + Sync + PartialReflect>(
		&mut self,
		parse: impl 'static + Send + Sync + Fn(&str) -> T,
	) -> &mut Self {
		self.add(move |value: &Value| match value {
			Value::Str(string) => Ok(Some(parse(string.as_str()))),
			_ => Ok(None),
		})
	}
}

/// One entry per numeric primitive: a `Value` number casts to it, a numeric
/// string parses into it (the quoted twin of the bare-number form, so a markup
/// `port="0"` authors a numeric field), and a non-numeric string errors rather
/// than falling through to a `String` whose `from_reflect` miss would keep the
/// target's default.
macro_rules! add_numbers {
	($table:expr, $($ty:ty),* $(,)?) => {$(
		$table.add(|value: &Value| match value {
			Value::Str(string) => string
				.as_str()
				.trim()
				.parse::<$ty>()
				.map(Some)
				.map_err(|_| bevyhow!(
					"invalid number {string:?}: expected a numeric string for a `{}` field",
					<$ty as TypePath>::type_path()
				)),
			Value::Uint(uint) => Ok(Some(*uint as $ty)),
			Value::Int(int) => Ok(Some(*int as $ty)),
			Value::Float(float) => Ok(Some(*float as $ty)),
			_ => Ok(None),
		});
	)*};
}

fn add_numbers(table: &mut Table) {
	add_numbers!(
		table, f32, f64, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64,
		u128, usize
	);
}

/// The string types a markup attribute's text lands in directly.
fn add_strings(table: &mut Table) {
	table
		.add_str(|string| string.to_string())
		.add_str(|string| SmolStr::new(string))
		// a tuple/struct literal carrying a string (eg `<Log::Message("hi")/>`,
		// whose variant field is a `Cow`) reflect-applies instead of panicking
		// on the `String`->`Cow` mismatch.
		.add_str(|string| {
			alloc::borrow::Cow::<'static, str>::Owned(string.to_string())
		})
		// a `Name`'s hashed inner field cannot be built field-by-field from a
		// plain string, so `<Name("Malenia")/>` builds via `Name::new`.
		.add_str(|string| Name::new(string.to_string()))
		// a logical path, so a markup `src="assets"` resolves to a `SmolPath`
		.add_str(|string| SmolPath::new(string));

	// a url is authored as the string it displays as, and the strict
	// `Url::parse` is what makes a control character in a frontmatter
	// `image_url` a loud error rather than a url nobody meant.
	table.add_hinted("a url, eg \"https://beet.org/blog\"", |value: &Value| {
		match value {
			Value::Str(string) => Url::parse(string.as_str()).map(Some),
			_ => Ok(None),
		}
	});
}

/// The domain types whose authored spelling is a single word rather than their
/// structure.
fn add_domain(table: &mut Table) {
	table
		// a `"true"`/`"false"` string, so a markup `<RouteSidebar home="false"/>`
		// authors a flag. Any other string errors rather than silently applying
		// `false`.
		.add(|value: &Value| {
			let Value::Str(string) = value else {
				return Ok(None);
			};
			match string.as_str().trim() {
				"true" => Ok(Some(true)),
				"false" => Ok(Some(false)),
				other => bevybail!(
					"invalid bool {other:?}: expected \"true\" or \"false\""
				),
			}
		})
		// a unit-suffixed string, so `<EndInDuration duration="50ms"/>` authors
		// a delay. The unit is required, and a malformed value (a non-string,
		// or a missing/unknown unit) errors rather than silently falling
		// through to a value that cannot apply.
		.add_hinted("a unit-suffixed duration, eg \"30s\"", |value: &Value| {
			match value {
				Value::Str(string) => Duration::from_human_str(string.as_str()),
				_ => None,
			}
			.map(Some)
			.ok_or_else(|| bevyhow!(
				"invalid duration {value:?}: expected a unit-suffixed string like \"50ms\" or \"1s\""
			))
		})
		// a `YYYY-MM-DD` string is midnight UTC on that date, so a markup
		// `{PageMeta{created:"2026-08-28"}}` authors a publication date. Any
		// other string errors rather than silently landing on the epoch; a
		// number declines to the newtype cast over its inner `i64`.
		.add_hinted("a `YYYY-MM-DD` date", |value: &Value| {
			let Value::Str(string) = value else {
				return Ok(None);
			};
			Timestamp::parse_date(string).map(Some).ok_or_else(|| bevyhow!(
				"invalid date {string:?}: expected a `YYYY-MM-DD` string like \"2026-08-28\""
			))
		})
		// the markup form of a filter is the allowlist a human writes, one
		// pattern or a list of them: `read="guestbook.*"` and
		// `read=["guestbook.*"]` both author includes. An exclude needs the
		// struct literal (`{read:{exclude:[..]}}`), which declines to the
		// structural walk and reflects normally now that each pattern coerces
		// from its string.
		.add_hinted("a glob pattern, repeatable", |value: &Value| match value {
			Value::Str(pattern) => glob_filter([pattern.as_str()]).map(Some),
			Value::List(items) => items
				.iter()
				.map(|item| match item {
					Value::Str(pattern) => pattern.as_str().xok(),
					other => bevybail!(
						"invalid glob pattern {other:?}: expected a string"
					),
				})
				.collect::<Result<Vec<_>>>()?
				.xmap(glob_filter)
				.map(Some),
			_ => Ok(None),
		})
		// one validated pattern, the item form of the filter above.
		.add_hinted("a glob pattern", |value: &Value| match value {
			Value::Str(pattern) => glob_pattern(pattern.as_str()).map(Some),
			_ => Ok(None),
		});

	// a bare word naming the shape a field accepts, so `<DynamicComponent
	// name=".." schema="u64"/>` declares what a runtime component means.
	// `ValueSchema` is reflect-opaque, so this is the only way markup can build
	// one short of a rust expression, and the vocabulary it reads is the markup
	// attribute's, hence `bsx`.
	#[cfg(feature = "bsx")]
	table.add(|value: &Value| match value {
		Value::Str(source) => value_schema(source.as_str()).map(Some),
		_ => Ok(None),
	});

	// a string targeting an `AbsPathBuf` is workspace-relative, mirroring
	// `AbsPathBuf`'s workspace-relative serde, so `<FsStore path="assets"/>`
	// takes a string attribute directly. `AbsPathBuf`/`WsPathBuf` live in the
	// std-only `path_utils`: a no_std (embedded) build has no filesystem paths
	// to resolve.
	#[cfg(feature = "std")]
	table.add_hinted(
		"a workspace-relative path",
		|value: &Value| match value {
			Value::Str(string) => {
				Ok(Some(WsPathBuf::new(string.as_str()).into_abs()))
			}
			_ => Ok(None),
		},
	);

	// a hex string, so a markup `<Theme primary="#006c4f"/>` spells a colour the
	// way every design tool does rather than as a four-float struct literal. A
	// malformed value errors rather than silently keeping the default.
	#[cfg(feature = "bevy_color")]
	table.add_hinted("a hex colour, eg \"#006c4f\"", |value: &Value| {
		let Value::Str(string) = value else {
			return Ok(None);
		};
		match bevy::color::Srgba::hex(string.as_str()) {
			Ok(srgba) => Ok(Some(bevy::color::Color::Srgba(srgba))),
			Err(_) => bevybail!(
				"invalid color {string:?}: expected a hex string like \"#006c4f\""
			),
		}
	});
}

/// A [`GlobFilter`] over `patterns`, as includes.
fn glob_filter<'a>(
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
/// A shape with more to say than a kind is written as a JSON Schema object,
/// which is the schema language a document author is likeliest to already know,
/// and anything richer than that is a rust expression.
#[cfg(feature = "bsx")]
fn value_schema(source: &str) -> Result<ValueSchema> {
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
#[cfg(all(feature = "bsx", feature = "json"))]
fn json_schema(source: &str) -> Result<ValueSchema> {
	ValueSchema::from_json_value(&serde_json::from_str(source)?)
}

/// Parsing a JSON schema requires the `json` feature.
#[cfg(all(feature = "bsx", not(feature = "json")))]
fn json_schema(_source: &str) -> Result<ValueSchema> {
	bevybail!("parsing a JSON schema requires the `json` feature")
}

#[cfg(test)]
mod test {
	use super::*;
	use bevy::reflect::FromReflect;

	/// Parse `value` into `T` through `T`'s builtin entry.
	fn parse<T: FromReflect>(value: Value) -> Result<Option<T>> {
		Ok(LiteralParser::parse_type(TypeId::of::<T>(), &value)?
			.map(|parsed| T::from_reflect(parsed.as_ref()).unwrap()))
	}

	/// The message a malformed authored value reports.
	fn message<T: 'static>(value: Value) -> String {
		LiteralParser::parse_type(TypeId::of::<T>(), &value)
			.unwrap_err()
			.to_string()
	}

	/// Every seeded entry parses its authored form.
	#[crate::test]
	fn parses_authored_forms() {
		parse::<u16>(Value::str("8080"))
			.unwrap()
			.xpect_eq(Some(8080));
		parse::<u16>(Value::Int(80)).unwrap().xpect_eq(Some(80));
		parse::<f32>(Value::str("1.5")).unwrap().xpect_eq(Some(1.5));
		parse::<bool>(Value::str("false"))
			.unwrap()
			.xpect_eq(Some(false));
		parse::<String>(Value::str("hi"))
			.unwrap()
			.xpect_eq(Some("hi".to_string()));
		parse::<SmolStr>(Value::str("hi"))
			.unwrap()
			.xpect_eq(Some(SmolStr::new("hi")));
		parse::<Name>(Value::str("Malenia"))
			.unwrap()
			.xpect_eq(Some(Name::new("Malenia")));
		parse::<SmolPath>(Value::str("assets/x"))
			.unwrap()
			.xpect_eq(Some(SmolPath::new("assets/x")));
		parse::<Duration>(Value::str("50ms"))
			.unwrap()
			.xpect_eq(Some(Duration::from_millis(50)));
		parse::<Timestamp>(Value::str("2026-08-28"))
			.unwrap()
			.xpect_eq(Timestamp::parse_date("2026-08-28"));
		parse::<GlobFilter>(Value::str("guestbook.*"))
			.unwrap()
			.xpect_eq(Some(GlobFilter::default().with_include("guestbook.*")));
		parse::<GlobFilter>(Value::new_list(["a", "b"]))
			.unwrap()
			.xpect_eq(Some(
				GlobFilter::default().with_include("a").with_include("b"),
			));
		parse::<GlobPattern>(Value::str("*.md"))
			.unwrap()
			.xpect_eq(Some(GlobPattern::new("*.md")));
		#[cfg(feature = "bsx")]
		parse::<ValueSchema>(Value::str("u64"))
			.unwrap()
			.xpect_eq(Some(ValueSchema::U64(default())));
	}

	/// Each entry DECLINES the shape it has nothing to say about, so the
	/// structural path takes over exactly as for a type with no entry.
	#[crate::test]
	fn declines_other_shapes() {
		parse::<u16>(Value::Bool(true)).unwrap().xpect_none();
		parse::<bool>(Value::Bool(true)).unwrap().xpect_none();
		parse::<String>(Value::Int(1)).unwrap().xpect_none();
		// a `Timestamp` number declines to the newtype cast over its inner i64
		parse::<Timestamp>(Value::Int(5)).unwrap().xpect_none();
		// a struct literal targeting a `GlobFilter` builds structurally
		parse::<GlobFilter>(Value::map()).unwrap().xpect_none();
		parse::<GlobPattern>(Value::Int(1)).unwrap().xpect_none();
		#[cfg(feature = "bsx")]
		parse::<ValueSchema>(Value::Int(1)).unwrap().xpect_none();
	}

	/// A type with no entry parses to nothing rather than erroring, so the seam
	/// falls through to its structural rules.
	#[crate::test]
	fn an_unregistered_type_declines() {
		#[derive(Reflect)]
		struct NoParser;
		parse::<NoParser>(Value::str("x")).unwrap().xpect_none();
	}

	/// A malformed input is an ERROR, never a decline: it must not fall through
	/// to a value that cannot apply, leaving the target at its default.
	#[crate::test]
	fn malformed_input_errors() {
		message::<u16>(Value::str("nope")).xpect_contains("invalid number");
		message::<bool>(Value::str("yes")).xpect_contains("invalid bool");
		// the unit is required, and a bare number carries none
		message::<Duration>(Value::str("50"))
			.xpect_contains("invalid duration");
		message::<Duration>(Value::Uint(50)).xpect_contains("invalid duration");
		message::<Timestamp>(Value::str("yesterday"))
			.xpect_contains("invalid date");
		message::<GlobFilter>(Value::str("["))
			.xpect_contains("invalid glob pattern");
		message::<GlobFilter>(Value::new_list([Value::Int(1)]))
			.xpect_contains("invalid glob pattern");
		message::<GlobPattern>(Value::str("["))
			.xpect_contains("invalid glob pattern");
		#[cfg(feature = "bsx")]
		message::<ValueSchema>(Value::str("uint64"))
			.xpect_contains("unknown schema")
			.xpect_contains("\"u64\"");
	}

	/// A registered parser wins over the builtin it replaces, the one way an app
	/// overrides a spelling. Keyed on a local type so the table stays as every
	/// other test expects it.
	#[crate::test]
	fn a_registered_parser_is_found() {
		#[derive(Debug, PartialEq, Reflect)]
		struct Shout(String);
		LiteralParser::register::<Shout>(LiteralParser::new(
			|value: &Value| match value {
				Value::Str(string) => {
					Ok(Some(Shout(string.to_uppercase().into())))
				}
				_ => Ok(None),
			},
		));
		parse::<Shout>(Value::str("hi"))
			.unwrap()
			.xpect_eq(Some(Shout("HI".into())));
	}

	#[cfg(feature = "bevy_color")]
	#[crate::test]
	fn parses_a_hex_color() {
		use bevy::color::Color;
		use bevy::color::Srgba;
		parse::<Color>(Value::str("#006c4f"))
			.unwrap()
			.xpect_eq(Some(Color::Srgba(Srgba::hex("#006c4f").unwrap())));
		parse::<Color>(Value::Int(1)).unwrap().xpect_none();
		parse::<Color>(Value::str("beetroot")).xpect_err();
	}

	#[cfg(feature = "std")]
	#[crate::test]
	fn parses_a_workspace_relative_path() {
		parse::<AbsPathBuf>(Value::str("assets"))
			.unwrap()
			.xpect_eq(Some(WsPathBuf::new("assets").into_abs()));
	}
}
