//! [`ValueSchema::default_value`]: the zero of a schema.
use crate::prelude::*;

/// Bound on [`ValueSchema::Ref`] hops while building a default, so a
/// self-recursive schema terminates. Only a reference consumes budget: every
/// cycle in a schema graph passes through one, because a schema written in
/// place is finite by construction.
const MAX_REFERENCE_HOPS: usize = 32;

impl ValueSchema {
	/// The zero of this schema: the value a fresh list item, a newly selected
	/// enum variant or a backfilled field starts as.
	///
	/// A [`ValueSchema::Ref`] resolves to a wildcard here, so use
	/// [`default_value_in`](Self::default_value_in) where a registry is in hand.
	pub fn default_value(&self) -> Value {
		self.default_value_in(SchemaResolver::default())
	}

	/// [`default_value`](Self::default_value), following each
	/// [`ValueSchema::Ref`] through `resolver`.
	///
	/// Total, and valid wherever a schema *has* a zero:
	///
	/// - a scalar is its empty value (`false`, `0`, `""`, no bytes)
	/// - a `Struct` is a map of its required fields' zeros, each field's own
	///   [`OnMissing::Default`] winning where it declares one; an optional field
	///   is absent, which is what optional means
	/// - a `Tuple` is its elements' zeros, in order
	/// - a `List` is empty, or `min_items` copies of the item's zero
	/// - a `Map` is empty, and an `Optional` is null
	/// - an `Enum` is its first variant, carrying that variant's payload zero
	/// - a floor a schema declares is honoured (a numeric `Min`, `min_items`),
	///   so the zero validates against the schema that produced it
	///
	/// Three kinds have no zero and answer [`Value::Null`]: `Any`/`Null`,
	/// an `Entity` (a reference to nothing, which item 18's picker fills), and
	/// a reference that resolves to neither.
	pub fn default_value_in(&self, resolver: SchemaResolver) -> Value {
		self.default_at(resolver, 0)
	}

	/// One arm of the zero, `hops` counting the references followed to reach it.
	fn default_at(&self, resolver: SchemaResolver, hops: usize) -> Value {
		match self {
			Self::Any | Self::Null | Self::Optional(_) | Self::Entity(_) => {
				Value::Null
			}
			Self::Bool(_) => Value::Bool(false),
			Self::I64(schema) => Value::Int(
				schema
					.constraints
					.iter()
					.find_map(|constraint| match constraint {
						I64Constraint::Min(min) => Some(min.value),
						_ => None,
					})
					.unwrap_or_default(),
			),
			Self::U64(schema) => Value::Uint(
				schema
					.constraints
					.iter()
					.find_map(|constraint| match constraint {
						U64Constraint::Min(min) => Some(min.value),
						_ => None,
					})
					.unwrap_or_default(),
			),
			Self::F64(schema) => Value::Float(
				schema
					.constraints
					.iter()
					.find_map(|constraint| match constraint {
						F64Constraint::Min(min) => Some(min.value),
						_ => None,
					})
					.unwrap_or_default(),
			),
			Self::String(_) => Value::str(""),
			Self::Bytes(_) => Value::Bytes(Vec::new()),
			Self::Struct(schema) => schema
				.fields
				.iter()
				.filter(|field| field.required)
				.map(|field| {
					let value = match &field.on_missing {
						Some(OnMissing::Default(value)) => value.clone(),
						_ => field.schema.default_at(resolver, hops),
					};
					(field.key.clone(), value)
				})
				.collect::<Map>()
				.xmap(Value::Map),
			Self::Tuple(schema) => schema
				.fields
				.iter()
				.map(|field| field.schema.default_at(resolver, hops))
				.collect::<Vec<_>>()
				.xmap(Value::List),
			Self::List(schema) => (0..schema.min_items.unwrap_or_default())
				.map(|_| schema.item.default_at(resolver, hops))
				.collect::<Vec<_>>()
				.xmap(Value::List),
			Self::Map(_) => Value::map(),
			Self::Enum(schema) => match schema.variants.first() {
				Some(variant) => variant.default_value(resolver, hops),
				None => Value::Null,
			},
			// a reference is the same zero seen more precisely, so it costs a hop
			// rather than recursing on itself forever
			Self::Ref(schema_ref) if hops < MAX_REFERENCE_HOPS => resolver
				.follow(schema_ref)
				.map(|schema| schema.default_at(resolver, hops + 1))
				.unwrap_or_default(),
			Self::Ref(_) => Value::Null,
		}
	}
}

impl VariantSchema {
	/// This variant as a value: the bare name for a unit variant, else the
	/// externally tagged `{"Name": zero}` its payload's zero fills.
	///
	/// The value an enum control writes when the variant is chosen, and the
	/// enum's own zero when it is the first variant.
	pub fn default_value(
		&self,
		resolver: SchemaResolver,
		hops: usize,
	) -> Value {
		match &self.payload {
			None => Value::Str(self.name.clone()),
			Some(payload) => {
				[(self.name.clone(), payload.default_at(resolver, hops))]
					.into_iter()
					.collect::<Map>()
					.xmap(Value::Map)
			}
		}
	}
}

#[cfg(all(test, feature = "json"))]
mod test {
	use crate::prelude::*;

	#[derive(Reflect)]
	#[allow(dead_code)]
	struct TodoItem {
		label: String,
		done: bool,
		note: Option<String>,
	}

	/// Every scalar's zero, and the two kinds with none.
	#[crate::test]
	fn a_scalar_is_its_empty_value() {
		ValueSchema::Bool(default())
			.default_value()
			.xpect_eq(Value::Bool(false));
		ValueSchema::I64(default())
			.default_value()
			.xpect_eq(Value::Int(0));
		ValueSchema::String(default())
			.default_value()
			.xpect_eq(Value::str(""));
		ValueSchema::Entity(default())
			.default_value()
			.xpect_eq(Value::Null);
		ValueSchema::Optional(Box::new(ValueSchema::Bool(default())))
			.default_value()
			.xpect_eq(Value::Null);
	}

	/// A struct's zero carries its required fields and leaves its optional ones
	/// absent, which is what optional means.
	#[crate::test]
	fn a_struct_carries_its_required_fields() {
		ValueSchema::of::<TodoItem>()
			.default_value()
			.xpect_eq(value!({ "label": "", "done": false }));
	}

	/// A field declaring a resolution declares its own zero, so the value a
	/// commit backfills and the value a fresh row starts with agree.
	#[crate::test]
	fn a_field_policy_wins_over_the_kind() {
		ValueSchema::Struct(StructSchema {
			name: None,
			allow_additional: false,
			fields: vec![
				NamedFieldSchema::new("done", ValueSchema::Bool(default()))
					.with_on_missing(OnMissing::Default(value!(true))),
			],
		})
		.default_value()
		.xpect_eq(value!({ "done": true }));
	}

	/// A floor the schema declares is honoured, so the zero validates against
	/// the schema that produced it rather than failing its own bounds.
	#[crate::test]
	async fn a_declared_floor_is_honoured() {
		let schema = ValueSchema::List(ListSchema {
			item: Box::new(ValueSchema::I64(I64Schema {
				constraints: vec![I64Constraint::Min(I64Min {
					value: 3,
					behavior: default(),
				})],
			})),
			min_items: Some(2),
			..default()
		});
		let mut zero = schema.default_value();
		zero.clone().xpect_eq(value!([3, 3]));
		schema.validate(&mut zero).await.len().xpect_eq(0);
	}

	/// An enum is its first variant, a unit one as its bare name and a
	/// payload-carrying one externally tagged over the payload's own zero.
	#[crate::test]
	fn an_enum_is_its_first_variant() {
		let unit = VariantSchema {
			name: "Error".into(),
			payload: None,
		};
		let payload = VariantSchema {
			name: "Computed".into(),
			payload: Some(ValueSchema::String(default())),
		};
		ValueSchema::Enum(EnumSchema {
			name: None,
			variants: vec![unit.clone(), payload.clone()],
		})
		.default_value()
		.xpect_eq(Value::str("Error"));
		ValueSchema::Enum(EnumSchema {
			name: None,
			variants: vec![payload, unit],
		})
		.default_value()
		.xpect_eq(value!({ "Computed": "" }));
	}

	/// A reference resolves through the registry and answers the zero of what it
	/// names; one that never arrives is null, like every other deferred
	/// indirection.
	#[crate::test]
	fn a_reference_resolves_to_the_zero_it_names() {
		let mut registry = SchemaRegistry::default();
		registry.insert("TodoItem", ValueSchema::of::<TodoItem>());
		let resolver = SchemaResolver::default().with_schemas(&registry);
		ValueSchema::reference("TodoItem")
			.default_value_in(resolver)
			.xpect_eq(value!({ "label": "", "done": false }));
		ValueSchema::reference("NotRegistered")
			.default_value_in(resolver)
			.xpect_eq(Value::Null);
	}

	/// The meta-schema is self-recursive, so its own zero is the case the hop
	/// budget exists for: it terminates, and it is a schema.
	#[crate::test]
	async fn the_meta_schema_has_a_zero() {
		let registry = SchemaRegistry::default();
		let resolver = SchemaResolver::default().with_schemas(&registry);
		let mut zero = ValueSchema::meta().default_value_in(resolver);
		ValueSchema::meta()
			.assert_valid_in(resolver, "the meta-schema's zero", &mut zero)
			.await
			.unwrap();
		zero.into_serde::<ValueSchema>().unwrap().xpect_eq(
			// `Any`, the first variant it declares
			ValueSchema::Any,
		);
	}

	/// A schema whose field names its own type terminates on the hop budget
	/// rather than recursing forever.
	#[crate::test]
	fn a_self_recursive_schema_terminates() {
		let mut registry = SchemaRegistry::default();
		registry.insert(
			"Node",
			ValueSchema::Struct(StructSchema {
				name: Some("Node".into()),
				allow_additional: false,
				fields: vec![NamedFieldSchema::new(
					"child",
					ValueSchema::reference("Node"),
				)],
			}),
		);
		let resolver = SchemaResolver::default().with_schemas(&registry);
		ValueSchema::reference("Node")
			.default_value_in(resolver)
			.is_map()
			.xpect_true();
	}
}
