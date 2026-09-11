//! Dependent schemas bound from their enclosing container: an
//! [`AtField`](SchemaRef::AtField) from the nearest struct value, a
//! [`MapSchema::Keyed`] entry from the registry under its key.
use crate::prelude::*;

impl ValueSchema {
	/// Whether this schema, or any schema within it up to the next struct
	/// scope, names a field of its enclosing struct.
	///
	/// The validator asks this to decide whether a struct must bind before
	/// descending, so a plain struct costs nothing on the read path, and the
	/// widget layer asks it too: a form over a struct whose fields bind needs
	/// the struct's value in hand before it can dispatch them.
	pub fn binds_a_field(&self) -> bool {
		match self {
			Self::Ref(SchemaRef::AtField(_)) => true,
			// a nested struct opens its own scope, so its fields bind against it
			Self::Struct(_) => false,
			Self::Optional(inner) => inner.binds_a_field(),
			Self::List(schema) => schema.item.binds_a_field(),
			Self::Map(MapSchema::Uniform { value }) => value.binds_a_field(),
			Self::Tuple(schema) => schema
				.fields
				.iter()
				.any(|field| field.schema.binds_a_field()),
			Self::Enum(schema) => schema.variants.iter().any(|variant| {
				variant
					.payload
					.as_ref()
					.is_some_and(ValueSchema::binds_a_field)
			}),
			_ => false,
		}
	}

	/// Substitute every [`SchemaRef::AtField`] this schema names with the schema
	/// the corresponding field of `scope` describes.
	///
	/// The recursion stops at a nested [`ValueSchema::Struct`], which opens its
	/// own scope and binds its own fields when the walk reaches it. A key the
	/// scope does not hold, or holds something that is not a schema, is left in
	/// place: it resolves to [`ValueSchema::Any`] downstream, deferring exactly as
	/// an unregistered name does, since a document can legitimately be read
	/// before the field describing it has arrived.
	pub fn bind(&self, scope: &Map) -> Self {
		match self {
			Self::Ref(SchemaRef::AtField(key)) => scope
				.0
				.get(key.as_str())
				.and_then(Self::from_described)
				.unwrap_or_else(|| self.clone()),
			Self::Optional(inner) => {
				Self::Optional(Box::new(inner.bind(scope)))
			}
			Self::List(schema) => Self::List(ListSchema {
				item: Box::new(schema.item.bind(scope)),
				..schema.clone()
			}),
			Self::Map(MapSchema::Uniform { value }) => {
				Self::Map(MapSchema::uniform(value.bind(scope)))
			}
			Self::Tuple(schema) => Self::Tuple(TupleSchema {
				fields: schema
					.fields
					.iter()
					.map(|field| UnnamedFieldSchema {
						schema: field.schema.bind(scope),
						..field.clone()
					})
					.collect(),
				..schema.clone()
			}),
			Self::Enum(schema) => Self::Enum(EnumSchema {
				variants: schema
					.variants
					.iter()
					.map(|variant| VariantSchema {
						name: variant.name.clone(),
						payload: variant
							.payload
							.as_ref()
							.map(|payload| payload.bind(scope)),
					})
					.collect(),
				..schema.clone()
			}),
			// a struct opens its own scope, a keyed map's entries resolve
			// through the registry, and every other kind names nothing
			schema => schema.clone(),
		}
	}

	/// Read `value` as the schema it describes: a map as a [`ValueSchema`], a
	/// bare string as a [`SchemaRef::Name`].
	///
	/// The string arm is the discriminator idiom, where the field naming the
	/// shape holds a name rather than a schema
	/// (`{ "kind": "circle", "props": .. }`).
	fn from_described(value: &Value) -> Option<Self> {
		match value {
			Value::Str(name) => Some(Self::reference(name.clone())),
			#[cfg(feature = "serde")]
			Value::Map(_) => value.clone().into_serde().ok(),
			_ => None,
		}
	}
}

impl MapSchema {
	/// The schema of the entry at `key`: a [`Uniform`](Self::Uniform) map's
	/// `value`, or the schema `resolver` holds under the key of a
	/// [`Keyed`](Self::Keyed) map.
	///
	/// The one place a keyed map is resolved, shared by validation, the field
	/// walk, the backfill and a form's map arm, so a map's entries are described
	/// at the schema level and never special-cased in a widget.
	///
	/// # Errors
	///
	/// A keyed entry whose key the registry does not hold: this binary cannot
	/// say what the entry is, exactly as the template loader rejects a component
	/// it has not registered.
	pub fn entry_schema<'a>(
		&'a self,
		resolver: SchemaResolver<'a>,
		key: &str,
	) -> Result<&'a ValueSchema> {
		match self {
			Self::Uniform { value } => Ok(value),
			Self::Keyed => resolver.schema(key).ok_or_else(|| {
				bevyhow!("no schema is registered under the key `{key}`")
			}),
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// The self-describing pair, now written in the schema language rather than
	/// hardcoded at the document root: `value` is whatever `schema` says.
	fn pair() -> ValueSchema {
		ValueSchema::Struct(StructSchema {
			name: Some("Pair".into()),
			description: None,
			allow_additional: false,
			fields: vec![
				NamedFieldSchema::new("schema", ValueSchema::meta()),
				NamedFieldSchema::new("value", ValueSchema::at_field("schema")),
			],
		})
	}

	#[crate::test]
	async fn a_sibling_describes_its_pair() {
		let registry = SchemaRegistry::default();
		let resolver = SchemaResolver::default().with_schemas(&registry);
		let mut valid = value!({
			"schema": { "Bool": {} },
			"value": true
		});
		pair()
			.assert_valid_in(resolver, "pair", &mut valid)
			.await
			.unwrap();

		// the same value under a schema the sibling now rejects
		let mut invalid = value!({
			"schema": { "Bool": {} },
			"value": "nope"
		});
		pair()
			.assert_valid_in(resolver, "pair", &mut invalid)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("value");
	}

	/// A bare string names a registered schema, which is the discriminator
	/// shape: one field says what the other one is.
	#[crate::test]
	async fn a_named_sibling_resolves_through_the_registry() {
		let mut registry = SchemaRegistry::default();
		registry.insert(
			"Circle",
			ValueSchema::Struct(StructSchema {
				name: Some("Circle".into()),
				description: None,
				allow_additional: false,
				fields: vec![NamedFieldSchema::new(
					"radius",
					ValueSchema::U64(default()),
				)],
			}),
		);
		let resolver = SchemaResolver::default().with_schemas(&registry);
		let shape = ValueSchema::Struct(StructSchema {
			name: Some("Shape".into()),
			description: None,
			allow_additional: false,
			fields: vec![
				NamedFieldSchema::new("kind", ValueSchema::String(default())),
				NamedFieldSchema::new("props", ValueSchema::at_field("kind")),
			],
		});
		shape
			.assert_valid_in(
				resolver,
				"shape",
				&mut value!({ "kind": "Circle", "props": { "radius": 3 } }),
			)
			.await
			.unwrap();
		shape
			.assert_valid_in(
				resolver,
				"shape",
				&mut value!({ "kind": "Circle", "props": { "radius": "big" } }),
			)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("radius");
	}

	/// A key the enclosing struct has no value for defers to a wildcard, exactly
	/// as an unregistered name does: a document may be read before the field
	/// describing it has arrived.
	#[crate::test]
	async fn an_unresolvable_key_defers() {
		ValueSchema::Struct(StructSchema {
			name: None,
			description: None,
			allow_additional: true,
			fields: vec![NamedFieldSchema::new(
				"value",
				ValueSchema::at_field("nope"),
			)],
		})
		.assert_valid("deferred", &mut value!({ "value": 7 }))
		.await
		.unwrap();
	}

	/// The scope is the *nearest* enclosing struct, so a nested one binds its
	/// own fields and never reaches past itself into the outer one.
	#[crate::test]
	async fn a_nested_struct_opens_its_own_scope() {
		let inner = ValueSchema::Struct(StructSchema {
			name: None,
			description: None,
			allow_additional: false,
			fields: vec![
				NamedFieldSchema::new("schema", ValueSchema::meta()),
				NamedFieldSchema::new("value", ValueSchema::at_field("schema")),
			],
		});
		let outer = ValueSchema::Struct(StructSchema {
			name: None,
			description: None,
			allow_additional: false,
			fields: vec![
				NamedFieldSchema::new("schema", ValueSchema::meta()),
				NamedFieldSchema::new("inner", inner),
			],
		});
		let registry = SchemaRegistry::default();
		let resolver = SchemaResolver::default().with_schemas(&registry);
		// the inner `value` follows the inner `schema`, not the outer one
		outer
			.assert_valid_in(
				resolver,
				"outer",
				&mut value!({
					"schema": { "Bool": {} },
					"inner": { "schema": { "U64": { "constraints": [] } }, "value": 7 }
				}),
			)
			.await
			.unwrap();
	}

	/// `{ "Circle": .., "Count": .. }`: each entry is whatever the registry
	/// holds under its key, so a keyed map is described once at the schema
	/// level rather than per widget.
	fn keyed_map() -> ValueSchema { ValueSchema::Map(MapSchema::Keyed) }

	fn count_registry() -> SchemaRegistry {
		let mut registry = SchemaRegistry::default();
		registry.insert("Count", ValueSchema::U64(default()));
		registry
	}

	#[crate::test]
	async fn an_entry_key_names_its_schema() {
		let registry = count_registry();
		let resolver = SchemaResolver::default().with_schemas(&registry);
		keyed_map()
			.assert_valid_in(resolver, "map", &mut value!({ "Count": 3 }))
			.await
			.unwrap();
		keyed_map()
			.assert_valid_in(resolver, "map", &mut value!({ "Count": "many" }))
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("Count");
	}

	/// A key the registry does not hold is an error naming the key: this
	/// binary cannot say what the entry is.
	#[crate::test]
	async fn an_unregistered_key_is_an_error() {
		let registry = count_registry();
		let resolver = SchemaResolver::default().with_schemas(&registry);
		keyed_map()
			.assert_valid_in(
				resolver,
				"map",
				&mut value!({ "Nope": "anything" }),
			)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("no schema is registered under the key `Nope`");
	}
}
