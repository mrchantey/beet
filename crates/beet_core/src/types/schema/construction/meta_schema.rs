//! [`ValueSchema::meta`]: the schema of a schema.
//!
//! The keystone closure of schema-as-data: a schema is itself a value, and the
//! value it is has a schema, so the machinery that renders a form over a todo
//! item is the machinery that renders a form over the todo item's *schema*.
//!
//! [`ValueSchema`] reflects as opaque (it is a recursive enum bevy's derive
//! cannot see through), so the meta-schema is hand-authored, and it describes
//! the serde encoding rather than the Rust layout: an externally tagged enum,
//! `"Any"` for a unit variant and `{"Struct": {..}}` for a payload. It is
//! registered under [`ValueSchema`]'s own type path by every
//! [`SchemaRegistry`], where it takes precedence over the (useless) reflect
//! derivation for the same path.
use crate::prelude::*;

impl ValueSchema {
	/// The schema describing a [`ValueSchema`] as data.
	///
	/// Recursive: every nested schema position is a [`ValueSchema::Ref`]
	/// back to this schema's own registration, which validation follows lazily,
	/// so the walk descends as far as the data does.
	///
	/// Adding a variant to [`ValueSchema`] means adding it here;
	/// `meta_schema.rs::describes_every_variant` is the tripwire.
	pub fn meta() -> ValueSchema {
		enumeration("ValueSchema", vec![
			unit_variant("Any"),
			unit_variant("Null"),
			variant("Bool", r#struct("BoolSchema", vec![])),
			variant(
				"I64",
				number("I64Schema", number_constraint("I64", signed())),
			),
			variant(
				"U64",
				number("U64Schema", number_constraint("U64", unsigned())),
			),
			variant(
				"F64",
				number("F64Schema", number_constraint("F64", float())),
			),
			variant("String", string_schema()),
			variant(
				"Bytes",
				r#struct("BytesSchema", vec![optional("max_len", unsigned())]),
			),
			variant("Entity", r#struct("EntitySchema", vec![])),
			variant("Struct", struct_schema()),
			variant("Tuple", tuple_schema()),
			variant("List", list_schema()),
			variant("Map", map_schema()),
			variant("Enum", enum_schema()),
			variant("Optional", meta_ref()),
			variant("Ref", schema_ref_schema()),
		])
	}
}

/// A reference back to the meta-schema, by the full type path that is its
/// identity in the [`SchemaRegistry`].
fn meta_ref() -> ValueSchema {
	ValueSchema::reference(ValueSchema::type_path())
}

/// The [`SchemaRef`] vocabulary: the ways a schema names another rather than
/// writing it in place.
///
/// Adding a variant to [`SchemaRef`] means adding it here;
/// `meta_schema.rs::describes_every_reference` is the tripwire.
fn schema_ref_schema() -> ValueSchema {
	enumeration("SchemaRef", vec![
		variant("Name", string()),
		variant("TypePath", string()),
		variant("Document", string()),
		variant("AtField", string()),
	])
}

// ── The composite payloads ─────────────────────────────────────────────

fn struct_schema() -> ValueSchema {
	r#struct("StructSchema", vec![
		optional("name", string()),
		optional("description", string()),
		field("allow_additional", boolean()),
		field("fields", list(named_field_schema())),
	])
}

fn tuple_schema() -> ValueSchema {
	r#struct("TupleSchema", vec![
		optional("name", string()),
		optional("description", string()),
		field("fields", list(unnamed_field_schema())),
	])
}

fn list_schema() -> ValueSchema {
	r#struct("ListSchema", vec![
		field("item", meta_ref()),
		optional("min_items", unsigned()),
		optional("max_items", unsigned()),
		field("unique", boolean()),
	])
}

/// [`MapSchema`] is the one composite that is itself an enum: every entry one
/// schema, or each entry the schema its key names.
///
/// Adding a variant to [`MapSchema`] means adding it here;
/// `meta_schema.rs::describes_every_map_schema` is the tripwire.
fn map_schema() -> ValueSchema {
	enumeration("MapSchema", vec![
		variant(
			"Uniform",
			r#struct("Uniform", vec![field("value", meta_ref())]),
		),
		unit_variant("Keyed"),
	])
}

fn enum_schema() -> ValueSchema {
	r#struct("EnumSchema", vec![
		optional("name", string()),
		optional("description", string()),
		field("variants", list(variant_schema())),
	])
}

fn named_field_schema() -> ValueSchema {
	r#struct("NamedFieldSchema", vec![
		field("key", string()),
		field("required", boolean()),
		optional("label", string()),
		optional("description", string()),
		optional("on_missing", on_missing()),
		field("schema", meta_ref()),
	])
}

fn unnamed_field_schema() -> ValueSchema {
	r#struct("UnnamedFieldSchema", vec![
		field("required", boolean()),
		optional("description", string()),
		field("schema", meta_ref()),
	])
}

fn variant_schema() -> ValueSchema {
	r#struct("VariantSchema", vec![
		field("name", string()),
		optional("payload", meta_ref()),
	])
}

/// The one per-field resolution policy, shared by the schema and binding layers.
fn on_missing() -> ValueSchema {
	enumeration("OnMissing", vec![
		unit_variant("Error"),
		// a default is a value of the field's own schema, which the sibling
		// `schema` key describes: the dependent arm, and the case it exists for
		variant("Default", ValueSchema::at_field("schema")),
		variant(
			"Computed",
			r#struct("Computed", vec![field("script", string())]),
		),
	])
}

// ── The scalar payloads ────────────────────────────────────────────────

fn string_schema() -> ValueSchema {
	r#struct("StringSchema", vec![
		field("sensitive", boolean()),
		field("multiline", boolean()),
		field("constraints", list(string_constraint())),
	])
}

fn string_constraint() -> ValueSchema {
	enumeration("StringConstraint", vec![
		variant("MinLength", bound("MinLength", unsigned())),
		variant("MaxLength", bound("MaxLength", unsigned())),
		unit_variant("Email"),
	])
}

/// `{ constraints: [..] }`, the shape every numeric schema shares.
fn number(name: &str, constraint: ValueSchema) -> ValueSchema {
	r#struct(name, vec![field("constraints", list(constraint))])
}

/// `Min | Max | Step`, the constraint set every numeric schema shares, over a
/// `value` of the number's own kind.
fn number_constraint(prefix: &str, value: ValueSchema) -> ValueSchema {
	enumeration(&format!("{prefix}Constraint"), vec![
		variant("Min", bound(&format!("{prefix}Min"), value.clone())),
		variant("Max", bound(&format!("{prefix}Max"), value.clone())),
		variant("Step", bound(&format!("{prefix}Step"), value)),
	])
}

/// `{ value, behavior }`, the shape every bound-style constraint shares.
fn bound(name: &str, value: ValueSchema) -> ValueSchema {
	r#struct(name, vec![
		field("value", value),
		field("behavior", constraint_behavior()),
	])
}

fn constraint_behavior() -> ValueSchema {
	enumeration("ConstraintBehavior", vec![
		unit_variant("Error"),
		unit_variant("Mutate"),
	])
}

// ── Authoring shorthands ───────────────────────────────────────────────

fn field(key: &str, schema: ValueSchema) -> NamedFieldSchema {
	NamedFieldSchema::new(key, schema)
}

/// An optional field. Serde writes an absent `Option` as an explicit null, so
/// the field must accept both a missing key and a null value.
fn optional(key: &str, schema: ValueSchema) -> NamedFieldSchema {
	NamedFieldSchema::new(key, ValueSchema::Optional(Box::new(schema)))
		.optional()
}

fn r#struct(name: &str, fields: Vec<NamedFieldSchema>) -> ValueSchema {
	ValueSchema::Struct(StructSchema {
		name: Some(name.into()),
		description: None,
		allow_additional: false,
		fields,
	})
}

fn enumeration(name: &str, variants: Vec<VariantSchema>) -> ValueSchema {
	ValueSchema::Enum(EnumSchema {
		name: Some(name.into()),
		description: None,
		variants,
	})
}

fn variant(name: &str, payload: ValueSchema) -> VariantSchema {
	VariantSchema {
		name: name.into(),
		payload: Some(payload),
	}
}

fn unit_variant(name: &str) -> VariantSchema {
	VariantSchema {
		name: name.into(),
		payload: None,
	}
}

fn list(item: ValueSchema) -> ValueSchema {
	ValueSchema::List(ListSchema {
		item: Box::new(item),
		min_items: None,
		max_items: None,
		unique: false,
	})
}

fn string() -> ValueSchema { ValueSchema::String(StringSchema::default()) }
fn boolean() -> ValueSchema { ValueSchema::Bool(BoolSchema::default()) }
fn signed() -> ValueSchema { ValueSchema::I64(I64Schema::default()) }
fn unsigned() -> ValueSchema { ValueSchema::U64(U64Schema::default()) }
fn float() -> ValueSchema { ValueSchema::F64(F64Schema::default()) }

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// One schema per [`ValueSchema`] variant, in declaration order.
	fn samples() -> Vec<ValueSchema> {
		vec![
			ValueSchema::Any,
			ValueSchema::Null,
			ValueSchema::Bool(default()),
			ValueSchema::I64(I64Schema {
				constraints: vec![I64Constraint::Min(I64Min {
					value: -3,
					behavior: ConstraintBehavior::Error,
				})],
			}),
			ValueSchema::U64(U64Schema {
				constraints: vec![U64Constraint::Step(U64Step {
					value: 2,
					behavior: ConstraintBehavior::Mutate,
				})],
			}),
			ValueSchema::F64(F64Schema {
				constraints: vec![F64Constraint::Max(F64Max {
					value: 1.5,
					behavior: ConstraintBehavior::Error,
				})],
			}),
			ValueSchema::String(StringSchema::default().sensitive().with(
				StringConstraint::MinLength {
					value: 3,
					behavior: ConstraintBehavior::Error,
				},
			)),
			ValueSchema::Bytes(BytesSchema { max_len: Some(8) }),
			ValueSchema::Entity(default()),
			ValueSchema::Struct(StructSchema {
				name: Some("TodoItem".into()),
				description: None,
				allow_additional: false,
				fields: vec![
					NamedFieldSchema::new(
						"label",
						ValueSchema::String(default()),
					),
					NamedFieldSchema::new("done", ValueSchema::Bool(default()))
						.optional()
						.with_on_missing(OnMissing::Default(value!(false))),
				],
			}),
			ValueSchema::Tuple(TupleSchema {
				name: Some("Pair".into()),
				description: None,
				fields: vec![UnnamedFieldSchema {
					required: true,
					description: Some("the first".into()),
					schema: ValueSchema::I64(default()),
				}],
			}),
			ValueSchema::List(ListSchema {
				item: Box::new(ValueSchema::reference("TodoItem")),
				min_items: Some(1),
				max_items: None,
				unique: true,
			}),
			ValueSchema::Map(MapSchema::uniform(ValueSchema::Null)),
			ValueSchema::Enum(EnumSchema {
				name: Some("Status".into()),
				description: None,
				variants: vec![
					VariantSchema {
						name: "Active".into(),
						payload: None,
					},
					VariantSchema {
						name: "Pending".into(),
						payload: Some(ValueSchema::String(default())),
					},
				],
			}),
			ValueSchema::Optional(Box::new(ValueSchema::Bool(default()))),
			ValueSchema::reference("TodoItem"),
		]
	}

	/// One reference per [`SchemaRef`] variant, in declaration order.
	fn references() -> Vec<SchemaRef> {
		vec![
			SchemaRef::Name("TodoItem".into()),
			SchemaRef::TypePath("bevy_color::color::Color".into()),
			SchemaRef::Document("schema/todo.json".into()),
			SchemaRef::AtField("schema".into()),
		]
	}

	/// One map per [`MapSchema`] variant, in declaration order.
	fn map_schemas() -> Vec<MapSchema> {
		vec![MapSchema::uniform(ValueSchema::Any), MapSchema::Keyed]
	}

	/// The variant names an externally tagged enum schema describes.
	fn described_variants(schema: ValueSchema) -> Vec<String> {
		let ValueSchema::Enum(schema) = schema else {
			panic!("an externally tagged enum");
		};
		schema
			.variants
			.iter()
			.map(|variant| variant.name.to_string())
			.collect()
	}

	#[crate::test]
	fn describes_every_variant() {
		samples()
			.iter()
			.map(|schema| schema.variant_name().to_string())
			.collect::<Vec<_>>()
			.xpect_eq(described_variants(ValueSchema::meta()));
	}

	/// The reference vocabulary is described in full too, so a new way of
	/// naming a schema fails here until the meta-schema can read it.
	#[crate::test]
	fn describes_every_reference() {
		references()
			.iter()
			.map(|schema_ref| schema_ref.variant_name().to_string())
			.collect::<Vec<_>>()
			.xpect_eq(described_variants(super::schema_ref_schema()));
	}

	/// And the map vocabulary, so a new kind of map fails here until the
	/// meta-schema can read it.
	#[crate::test]
	fn describes_every_map_schema() {
		map_schemas()
			.iter()
			.map(|map| map.variant_name().to_string())
			.collect::<Vec<_>>()
			.xpect_eq(described_variants(super::map_schema()));
	}

	/// The closure: every schema is a value the meta-schema accepts, and it
	/// deserializes back to the schema it came from.
	#[crate::test]
	async fn a_schema_is_a_value_of_the_meta_schema() {
		let registry = SchemaRegistry::default();
		let resolver = SchemaResolver::default().with_schemas(&registry);
		let referencing = references().into_iter().map(ValueSchema::Ref);
		let maps = map_schemas().into_iter().map(ValueSchema::Map);
		for schema in samples().into_iter().chain(referencing).chain(maps) {
			let mut value = Value::from_serde(&schema).unwrap();
			ValueSchema::meta()
				.assert_valid_in(resolver, schema.variant_name(), &mut value)
				.await
				.unwrap();
			value.into_serde::<ValueSchema>().unwrap().xpect_eq(schema);
		}
	}

	/// The meta-schema is intrinsic to a registry, so a schema document
	/// validates wherever it is read.
	#[crate::test]
	async fn is_registered_by_every_registry() {
		let registry = SchemaRegistry::default();
		let resolver = SchemaResolver::default().with_schemas(&registry);
		resolver
			.schema(ValueSchema::type_path())
			.unwrap()
			.xpect_eq(ValueSchema::meta());
		// and by its short name, the authoring sugar
		resolver.schema("ValueSchema").is_some().xpect_true();
	}

	/// A malformed schema value is rejected, naming the field that broke it.
	#[crate::test]
	async fn rejects_a_malformed_schema() {
		let registry = SchemaRegistry::default();
		let resolver = SchemaResolver::default().with_schemas(&registry);
		// a struct field whose own schema is not a schema
		let mut value = value!({ "Struct": {
			"name": "TodoItem",
			"allow_additional": false,
			"fields": [{
				"key": "label",
				"required": true,
				"label": null,
				"description": null,
				"on_missing": null,
				"schema": "NotAVariant"
			}]
		} });
		ValueSchema::meta()
			.assert_valid_in(resolver, "schema.json", &mut value)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("schema.json")
			.xpect_contains("NotAVariant");
	}
}
