//! [`SchemaRef`]: the ways a schema can be named rather than written in place.
use crate::prelude::*;

/// How to find a schema that is not written inline.
///
/// The one vocabulary for "described by", reached through
/// [`ValueSchema::Ref`]. Writing a schema in place needs no arm here, because
/// a [`ValueSchema`] *is* the inline case; every other way of naming one is an
/// arm below.
///
/// Three of them resolve against a [`SchemaResolver`] alone, and are what a
/// document uses to declare its own shape:
///
/// - [`Name`](Self::Name), the one [`SchemaRegistry`] namespace, which holds
///   authored and reflect-derived schemas alike
/// - [`TypePath`](Self::TypePath), a registered Rust type, resolved through the
///   by-name registry first so a hand-authored schema can stand in for what
///   reflection would derive
/// - [`Document`](Self::Document), a schema document in this document's own
///   store, resolved by **location** in the by-location index
///
/// The fourth resolves against the *value* instead.
///
/// # `AtField`, the dependent arm
///
/// [`AtField`](Self::AtField) says "my schema is the one described at this key",
/// naming a field of the nearest enclosing struct value, exactly as
/// [`DocumentPath::Ancestor`] names the nearest enclosing document. It is what
/// lets a self-describing pair be written in the schema language rather than
/// hardcoded:
///
/// ```
/// # use beet_core::prelude::*;
/// // `{ "schema": .., "value": .. }`: the value is whatever the sibling says
/// let pair = ValueSchema::Struct(StructSchema {
///     name: Some("TypedDocument".into()),
///     allow_additional: false,
///     fields: vec![
///         NamedFieldSchema::new("schema", ValueSchema::meta()),
///         NamedFieldSchema::new("value", ValueSchema::at_field("schema")),
///     ],
/// });
/// # let _ = pair;
/// ```
///
/// A nested struct establishes its own scope, so a key always resolves against
/// the innermost struct the field belongs to and never reaches sideways into an
/// unrelated subtree. The value found there is read as a schema: a map as a
/// [`ValueSchema`] (so `{"Bool":{}}` describes a boolean), and a bare string as
/// a [`Name`](Self::Name), which is the discriminator idiom
/// (`{ "kind": "circle", "props": .. }`).
///
/// It yields a *schema*, never a constraint. There is deliberately no way to say
/// "if this field is set then that one is required": that is where a dependent
/// schema stops being resolvable and starts being a rules engine.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[reflect(opaque)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SchemaRef {
	/// A schema registered under this name in the [`SchemaRegistry`], the one
	/// by-name namespace.
	Name(SmolStr),
	/// A registered Rust type, ie `bevy_color::color::Color`.
	TypePath(SmolStr),
	/// A schema document in this document's own store, resolved by location.
	Document(SmolPath),
	/// The schema described by the value at this key of the nearest enclosing
	/// struct.
	AtField(SmolStr),
}

impl SchemaRef {
	/// The identifying path of this reference, for diagnostics.
	pub fn as_str(&self) -> SmolStr {
		match self {
			Self::Name(name) => name.clone(),
			Self::TypePath(path) => path.clone(),
			Self::Document(path) => path.as_str().into(),
			Self::AtField(key) => key.clone(),
		}
	}
}

impl core::fmt::Display for SchemaRef {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::Name(name) => write!(f, "{name}"),
			Self::TypePath(path) => write!(f, "type `{path}`"),
			Self::Document(path) => write!(f, "document `{path}`"),
			Self::AtField(key) => write!(f, "the schema at `{key}`"),
		}
	}
}

impl ValueSchema {
	/// A schema registered under `name`, the by-name arm.
	pub fn reference(name: impl Into<SmolStr>) -> Self {
		Self::Ref(SchemaRef::Name(name.into()))
	}

	/// The schema of a registered Rust type, resolved at runtime.
	pub fn type_ref<T: TypePath>() -> Self {
		Self::Ref(SchemaRef::TypePath(SmolStr::new_static(T::type_path())))
	}

	/// The schema held by the schema document at `path`, resolved by location in
	/// this document's own store.
	pub fn document(path: impl Into<SmolPath>) -> Self {
		Self::Ref(SchemaRef::Document(path.into()))
	}

	/// The schema described by the value at `key` of the nearest enclosing
	/// struct, the dependent arm ([`SchemaRef::AtField`]).
	pub fn at_field(key: impl Into<SmolStr>) -> Self {
		Self::Ref(SchemaRef::AtField(key.into()))
	}

	/// This schema with its outermost [`SchemaRef`] followed, or itself when it
	/// names nothing.
	///
	/// One hop, not a deep expansion: [`SchemaRegistry::resolve`] is the eager
	/// walk, and validation follows a reference lazily where it meets it. A name
	/// or document that has not arrived defers to [`ValueSchema::Any`], exactly
	/// as it does everywhere else, and so does
	/// [`AtField`](SchemaRef::AtField), which only the struct holding it can
	/// answer. A [`TypePath`](SchemaRef::TypePath) is the one arm that errors,
	/// because an unregistered Rust type is a build mistake rather than a late
	/// arrival.
	pub fn resolve(&self, resolver: SchemaResolver<'_>) -> Result<ValueSchema> {
		match self {
			Self::Ref(SchemaRef::Name(name)) => resolver
				.schema(name)
				.cloned()
				.unwrap_or(ValueSchema::Any)
				.xok(),
			Self::Ref(SchemaRef::Document(path)) => resolver
				.located(path)
				.cloned()
				.unwrap_or(ValueSchema::Any)
				.xok(),
			Self::Ref(SchemaRef::TypePath(path)) => resolver.type_schema(path),
			Self::Ref(SchemaRef::AtField(_)) => ValueSchema::Any.xok(),
			schema => schema.clone().xok(),
		}
	}

	/// The reference this schema is, if it is one.
	pub fn as_ref(&self) -> Option<&SchemaRef> {
		match self {
			Self::Ref(schema_ref) => Some(schema_ref),
			_ => None,
		}
	}

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
