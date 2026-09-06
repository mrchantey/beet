//! Dependent schema references bound from an enclosing struct value.
use crate::prelude::*;

impl ValueSchema {
	/// Whether this schema, or any schema within it up to the next struct
	/// scope, names a field of its enclosing struct.
	///
	/// The cheap check that skips [`bind`](Self::bind) entirely for the schemas
	/// that need no binding, which is nearly all of them. Public because the
	/// widget layer asks it too: a form over a struct whose fields bind needs
	/// the struct's value in hand before it can dispatch them.
	pub fn binds_a_field(&self) -> bool {
		match self {
			Self::Ref(SchemaRef::AtField(_)) => true,
			// a nested struct opens its own scope, so its fields bind against it
			Self::Struct(_) => false,
			Self::Optional(inner) => inner.binds_a_field(),
			Self::List(schema) => schema.item.binds_a_field(),
			Self::Map(schema) => schema.value.binds_a_field(),
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
	/// the value at that key describes, `scope` being the nearest enclosing
	/// struct value.
	///
	/// The recursion stops at a nested [`ValueSchema::Struct`], which opens its
	/// own scope and binds its own fields when the walk reaches it. A key the
	/// scope has no value for stays unbound, deferring to a wildcard exactly as
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
			Self::Map(schema) => Self::Map(MapSchema {
				value: Box::new(schema.value.bind(scope)),
			}),
			Self::Tuple(schema) => Self::Tuple(TupleSchema {
				name: schema.name.clone(),
				fields: schema
					.fields
					.iter()
					.map(|field| UnnamedFieldSchema {
						schema: field.schema.bind(scope),
						..field.clone()
					})
					.collect(),
			}),
			Self::Enum(schema) => Self::Enum(EnumSchema {
				name: schema.name.clone(),
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
			}),
			// a struct opens its own scope, and every other kind names nothing
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

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// The self-describing pair, now written in the schema language rather than
	/// hardcoded at the document root: `value` is whatever `schema` says.
	fn pair() -> ValueSchema {
		ValueSchema::Struct(StructSchema {
			name: Some("Pair".into()),
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
			allow_additional: false,
			fields: vec![
				NamedFieldSchema::new("schema", ValueSchema::meta()),
				NamedFieldSchema::new("value", ValueSchema::at_field("schema")),
			],
		});
		let outer = ValueSchema::Struct(StructSchema {
			name: None,
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
}
