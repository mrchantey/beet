//! Literal-to-reflected-value resolution, with type inference.
//!
//! A [`DataLiteral`] becomes a `Box<dyn PartialReflect>`, inferring its concrete
//! type from the target field's [`TypeInfo`]: a `{x:0,y:0,z:2}` on a `Vec3` field
//! builds a `Vec3`, `Center` infers the enum variant, and `0` coerces to `0.0f32`
//! when the field is `f32`. Every dynamic value calls `set_represented_type` with
//! the target's `'static` `TypeInfo`, so `from_reflect`/`apply` resolve the
//! concrete type downstream.
//!
//! The dispatch and the registry lookups live here; the coercions each arm
//! reaches are [`super::scalar`] and [`super::composite`].

use super::composite::*;
use super::scalar::*;
use crate::prelude::*;
use bevy::reflect::PartialReflect;
use bevy::reflect::TypeInfo;
use bevy::reflect::TypeRegistration;
use bevy::reflect::TypeRegistry;
use bevy::reflect::enums::DynamicEnum;
use bevy::reflect::enums::DynamicVariant;
use bevy::reflect::tuple::DynamicTuple;

/// Resolves a `$name` entity reference to a concrete (possibly forward-mapped)
/// [`Entity`], threaded through nested literals so a spread component's
/// `Entity`-typed field resolves through the one entity model.
pub(in crate::bsx) type EntityResolver<'a> = &'a mut dyn FnMut(&str) -> Entity;

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
		if let Some(some_info) =
			field_info.and_then(reflect_ext::option_some_inner)
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
		// a named tuple literal wrapping ONE scalar (`<Name("x")/>`, `{Name("x")}`)
		// reduces to that scalar for a target whose type carries its own authored
		// spelling, so the parser sees the string the markup writes: a `Name`'s
		// hashed inner field cannot be reflect-built field-by-field.
		if let Some(info) = field_info
			&& LiteralParser::get(info.type_id()).is_some()
			&& let Some(value) = tuple_scalar(literal)
		{
			return scalar_to_reflect(value, field_info);
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

/// Whether a literal already names an `Option` variant (`Some`/`None`).
fn is_option_literal(literal: &DataLiteral) -> bool {
	matches!(literal, DataLiteral::Enum(named) if named.name == "Some" || named.name == "None")
}

/// The scalar a single-item named tuple literal wraps (`Name("x")`). `None` for
/// any other shape, which falls through to the ordinary path.
fn tuple_scalar(literal: &DataLiteral) -> Option<&Value> {
	let DataLiteral::Enum(named) = literal else {
		return None;
	};
	match &named.fields {
		NamedFields::Tuple(items) if items.len() == 1 => match &items[0] {
			DataLiteral::Scalar(value) => Some(value),
			_ => None,
		},
		_ => None,
	}
}

/// Look up a registered type by short type path, the [`reflect_ext`] resolver
/// under the name this module's callers use.
pub(in crate::bsx) fn registration_by_name<'a>(
	registry: &'a TypeRegistry,
	name: &str,
) -> Option<&'a TypeRegistration> {
	reflect_ext::registration_by_name(registry, name)
}

/// Resolve a `Type::Variant` spread name (eg `SteerTarget::Entity`) to the
/// *enum's* registration, so a `{SteerTarget::Entity($cheese)}` spread builds
/// the variant through [`enum_to_reflect`] (which reduces the qualified name to
/// its last segment). `None` when the prefix is not a registered enum carrying
/// that variant, so a genuine miss still falls through to the unknown-name path.
pub(in crate::bsx) fn enum_variant_registration<'a>(
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

#[cfg(test)]
mod test {
	use super::*;
	use bevy::reflect::FromReflect;
	use bevy::reflect::Typed;

	/// A `GlobFilter` field takes the allowlist a human writes, as a list or as
	/// a bare string: the shorthand for the common case, an allowlist.
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

	/// The full form: a `GlobFilter` struct literal names either list, each
	/// pattern coercing from its string, so a denylist authors as directly as an
	/// allowlist (`{filter:{exclude:["blog/**"]}}`).
	#[crate::test]
	fn coerces_a_glob_filter_struct_literal() {
		#[derive(Reflect, PartialEq, Debug, Default)]
		struct Discovery {
			filter: GlobFilter,
		}
		resolve::<Discovery>(DataLiteral::Enum(NamedLiteral {
			name: "Discovery".into(),
			fields: NamedFields::Struct(vec![(
				"filter".into(),
				DataLiteral::Enum(NamedLiteral {
					name: "GlobFilter".into(),
					fields: NamedFields::Struct(vec![(
						"exclude".into(),
						DataLiteral::List(vec![DataLiteral::Scalar(
							Value::Str("blog/**".into()),
						)]),
					)]),
				}),
			)]),
		}))
		.xpect_eq(Discovery {
			filter: GlobFilter::default().with_exclude("blog/**"),
		});
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
	#[cfg(feature = "json")]
	#[crate::test]
	fn coerces_a_json_schema_to_a_value_schema() {
		#[derive(Reflect, PartialEq, Debug, Default)]
		struct Declaration {
			schema: ValueSchema,
		}
		let source = r#"{"type":"array","items":{"type":"integer"}}"#;
		resolve::<Declaration>(schema_literal(source))
			.schema
			.xpect_eq(
				ValueSchema::from_json_value(
					&serde_json::from_str(source).unwrap(),
				)
				.unwrap(),
			);
	}

	/// A misspelled kind names the ones that exist rather than becoming a
	/// wildcard that quietly validates nothing.
	#[crate::test]
	fn an_unknown_schema_names_the_kinds() {
		resolve_err::<ValueSchema>(DataLiteral::Scalar(Value::str("uint64")))
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

	/// The message a literal that cannot resolve against `T` reports.
	fn resolve_err<T: Typed>(literal: DataLiteral) -> String {
		DataLiteral::to_reflect(
			&literal,
			Some(T::type_info()),
			&TypeRegistry::default(),
			&mut |_: &str| Entity::PLACEHOLDER,
		)
		.unwrap_err()
		.to_string()
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
	/// string cannot say which one it is, so the coercion must not guess, and
	/// must say so, since a `String` fallthrough would miss in `from_reflect`
	/// and leave the target at its default.
	#[crate::test]
	fn does_not_coerce_string_to_a_wider_struct() {
		#[derive(Reflect, PartialEq, Debug, Default)]
		struct Pair {
			label: SmolStr,
			other: SmolStr,
		}
		resolve_err::<Pair>(DataLiteral::Scalar(Value::Str("net".into())))
			.xpect_contains("cannot author")
			.xpect_contains("Pair");
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
		registration_by_name(&registry, "GenericMarker")
			.unwrap()
			.type_info()
			.type_path()
			.xpect_eq(GenericMarker::<u32>::type_info().type_path());
		// the exact short path still resolves; an unknown name does not
		registration_by_name(&registry, "GenericMarker<u32>").xpect_some();
		registration_by_name(&registry, "Nope").xpect_none();
		// a second instantiation makes the bare name ambiguous
		registry.register::<GenericMarker<bool>>();
		registration_by_name(&registry, "GenericMarker").xpect_none();
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
		registration_by_name(&registry, "Dup").xpect_none();
		// each fully-qualified path resolves unambiguously
		registration_by_name(&registry, Dup::type_info().type_path())
			.unwrap()
			.type_info()
			.type_path()
			.xpect_eq(Dup::type_info().type_path());
		registration_by_name(&registry, outer::Dup::type_info().type_path())
			.unwrap()
			.type_info()
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
		// ..and case-insensitively, since a markup attribute is prose: a human
		// writes `records="identityonly"`, the variant is `IdentityOnly`
		resolve::<Records>(DataLiteral::Scalar(Value::str("identityonly")))
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

	/// Two variants that differ only by case are an ambiguity a case-insensitive
	/// match cannot resolve, so it refuses rather than picking the one declared
	/// first. An exact spelling still resolves each of them.
	#[beet_core::test]
	fn an_ambiguous_variant_case_errors() {
		#[derive(Debug, Default, PartialEq, Reflect)]
		#[allow(non_camel_case_types)]
		enum Casing {
			#[default]
			Draft,
			draft,
		}
		resolve::<Casing>(DataLiteral::Scalar(Value::str("draft")))
			.xpect_eq(Casing::draft);
		let registry = TypeRegistry::default();
		let mut resolver = |_: &str| Entity::PLACEHOLDER;
		DataLiteral::to_reflect(
			&DataLiteral::Scalar(Value::str("DRAFT")),
			Some(Casing::type_info()),
			&registry,
			&mut resolver,
		)
		.unwrap_err()
		.to_string()
		.xpect_contains("spell one exactly");
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
	/// a string (eg `<LogLike::Message("hi")/>`, whose variant field is a `Cow`)
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
