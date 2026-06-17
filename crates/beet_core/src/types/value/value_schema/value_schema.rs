//! [`ValueSchema`]: an interface-oriented schema for [`Value`]s.
use super::*;
use crate::prelude::*;
use bevy::reflect::TypeInfo;
use bevy::reflect::Typed;

/// An interface-oriented description of a [`Value`]'s shape.
///
/// Used for driving dynamic UIs, performing validation and producing a
/// [`Schema`] (JSON Schema) representation.
#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, Component,
)]
#[reflect(opaque)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ValueSchema {
	/// Matches any value. An escape hatch that disables validation and
	/// type-checking for this field.
	Any,
	/// Always [`Value::Null`].
	Null,
	/// A boolean value.
	Bool(BoolSchema),
	/// A signed 64-bit integer.
	I64(I64Schema),
	/// An unsigned 64-bit integer.
	U64(U64Schema),
	/// A 64-bit float.
	F64(F64Schema),
	/// A string.
	String(StringSchema),
	/// Raw bytes.
	Bytes(BytesSchema),
	/// A struct with named fields.
	Struct(StructSchema),
	/// A fixed-arity tuple (also used for tuple structs).
	Tuple(TupleSchema),
	/// A homogenous sequence (list, array or set).
	List(ListSchema),
	/// A map with string keys.
	Map(MapSchema),
	/// A tagged union.
	Enum(EnumSchema),
	/// An optional value: [`Value::Null`] is accepted, anything else is
	/// validated against the inner schema. This is how an `Option`-typed field
	/// is represented so a missing or null value validates rather than failing.
	Optional(Box<ValueSchema>),
	/// A reference to another template's (or registered type's) schema, resolved
	/// by name against the [`SchemaRegistry`].
	///
	/// This is what makes schemas composable: an `items` array of `TodoItem`
	/// references `TodoItem`'s schema, so schemas form a graph mirroring the
	/// template graph and validate recursively. The name is a registered
	/// template's module path (`path::to::TodoItem`) or a Rust short type path.
	/// Until resolved, validation against it is a wildcard (deferred), since the
	/// referenced schema may resolve asynchronously.
	Reference(SmolStr),
}

impl Default for ValueSchema {
	fn default() -> Self { Self::Null }
}

/// Fallback when the `json` feature is off (the real parser lives in
/// `from_json.rs`): JSON schema parsing is unavailable, so a `bx:schema` block is
/// treated as absent by its `.ok()` callers.
#[cfg(not(feature = "json"))]
impl ValueSchema {
	/// Parsing a JSON schema requires the `json` feature.
	pub fn from_json_schema(_json: &str) -> Result<ValueSchema> {
		bevybail!("parsing a JSON schema requires the `json` feature")
	}
}

impl ValueSchema {
	/// Build a schema for `T` via its bevy reflect type info.
	pub fn of<T: Typed>() -> Self { Self::from_type_info(T::type_info()) }

	/// Build a schema from a bevy reflect [`TypeInfo`].
	pub fn from_type_info(type_info: &TypeInfo) -> Self {
		from_type_info::build(type_info)
	}

	/// Validate (and possibly mutate) `value` against this schema.
	///
	/// Returns the list of [`ValidationError`]s collected; an empty list means
	/// the value is valid.
	pub async fn validate(&self, value: &mut Value) -> Vec<ValidationError> {
		let path = FieldPath::default();
		self.apply(&path, value).await
	}

	/// Resolve the schema of a nested field by `path`.
	///
	/// The dual of [`Document::get_field_ref`](crate::prelude::Document):
	/// descends into struct fields, map values, list items and tuple elements.
	/// [`ValueSchema::Any`] swallows the remaining path and matches anything.
	pub fn get_field_schema(
		&self,
		path: &[FieldSegment],
	) -> Result<&ValueSchema> {
		let mut current = self;
		for (i, segment) in path.iter().enumerate() {
			current = match (current, segment) {
				// Any matches the rest of the path
				(ValueSchema::Any, _) => return Ok(current),
				// an unresolved reference swallows the rest of the path, like Any
				(ValueSchema::Reference(_), _) => return Ok(current),
				// an optional descends into its inner schema for the same segment
				(ValueSchema::Optional(inner), _) => {
					return inner.get_field_schema(&path[i..]);
				}
				(ValueSchema::Struct(schema), FieldSegment::ObjectKey(key)) => {
					&schema
						.fields
						.iter()
						.find(|field| field.key == *key)
						.ok_or_else(|| bevyhow!("schema has no field `{key}`"))?
						.schema
				}
				(ValueSchema::Map(schema), FieldSegment::ObjectKey(_)) => {
					schema.value.as_ref()
				}
				(ValueSchema::List(schema), FieldSegment::ArrayIndex(_)) => {
					schema.item.as_ref()
				}
				(ValueSchema::Tuple(schema), FieldSegment::ArrayIndex(idx)) => {
					&schema
						.fields
						.get(*idx)
						.ok_or_else(|| {
							bevyhow!("tuple schema has no element {idx}")
						})?
						.schema
				}
				(schema, segment) => bevybail!(
					"cannot resolve segment `{segment}` against schema `{schema:?}`"
				),
			};
		}
		Ok(current)
	}

	/// Whether this schema is compatible with `other`, treating
	/// [`ValueSchema::Any`] on either side as a wildcard.
	pub fn matches(&self, other: &ValueSchema) -> bool {
		match (self, other) {
			// an unresolved reference or `Any` is a wildcard on either side
			(ValueSchema::Any | ValueSchema::Reference(_), _) => true,
			(_, ValueSchema::Any | ValueSchema::Reference(_)) => true,
			// an optional matches its bare inner and another optional's inner, so a
			// typed write of `T` validates against an `Option<T>` field
			(ValueSchema::Optional(inner), other)
			| (other, ValueSchema::Optional(inner)) => inner.matches(other),
			(a, b) => a == b,
		}
	}

	/// Assert this schema [`matches`](Self::matches) `other`, reporting the
	/// field `path` on mismatch.
	///
	/// Shared by the `DocumentSchema` field-type checks and the field-local
	/// typed write fast path.
	pub fn assert_matches(
		&self,
		other: &ValueSchema,
		path: &[FieldSegment],
	) -> Result {
		if self.matches(other) {
			Ok(())
		} else {
			bevybail!(
				"Field Schema Mismatch at `{}`\nExpected: `{other:?}`\nReceived: `{self:?}`",
				FieldPath::from(path)
			)
		}
	}
}

impl ApplyConstraints for ValueSchema {
	type Value = Value;
	fn apply<'a>(
		&'a self,
		path: &'a FieldPath,
		value: &'a mut Self::Value,
	) -> ApplyFuture<'a> {
		Box::pin(async move {
			match self {
				ValueSchema::Any => Vec::new(),
				ValueSchema::Null => validate_null(path, value),
				ValueSchema::Bool(_) => validate_bool(path, value),
				ValueSchema::I64(schema) => {
					validate_i64(schema, path, value).await
				}
				ValueSchema::U64(schema) => {
					validate_u64(schema, path, value).await
				}
				ValueSchema::F64(schema) => {
					validate_f64(schema, path, value).await
				}
				ValueSchema::String(schema) => {
					validate_string(schema, path, value).await
				}
				ValueSchema::Bytes(schema) => {
					validate_bytes(schema, path, value).await
				}
				ValueSchema::Struct(schema) => {
					validate_struct(schema, path, value).await
				}
				ValueSchema::Tuple(schema) => {
					validate_tuple(schema, path, value).await
				}
				ValueSchema::List(schema) => {
					validate_list(schema, path, value).await
				}
				ValueSchema::Map(schema) => {
					validate_map(schema, path, value).await
				}
				ValueSchema::Enum(schema) => {
					validate_enum(schema, path, value).await
				}
				ValueSchema::Optional(inner) => {
					// a null satisfies an optional; anything else validates as the
					// inner schema.
					if matches!(value, Value::Null) {
						Vec::new()
					} else {
						inner.apply(path, value).await
					}
				}
				// an unresolved reference is a wildcard: the referenced schema may
				// still be resolving asynchronously, so validation is deferred.
				ValueSchema::Reference(_) => Vec::new(),
			}
		})
	}
}

fn type_mismatch(
	path: &FieldPath,
	expected: &str,
	actual: &Value,
) -> Vec<ValidationError> {
	vec![ValidationError::new(
		path.clone(),
		format!("expected {}, got {}", expected, actual.kind()),
	)]
}

fn validate_null(path: &FieldPath, value: &Value) -> Vec<ValidationError> {
	if matches!(value, Value::Null) {
		Vec::new()
	} else {
		type_mismatch(path, "null", value)
	}
}

fn validate_bool(path: &FieldPath, value: &Value) -> Vec<ValidationError> {
	if matches!(value, Value::Bool(_)) {
		Vec::new()
	} else {
		type_mismatch(path, "bool", value)
	}
}

async fn validate_i64(
	schema: &I64Schema,
	path: &FieldPath,
	value: &mut Value,
) -> Vec<ValidationError> {
	let Value::Int(mut n) = *value else {
		// allow coercion from Uint that fits
		if let Value::Uint(u) = *value
			&& let Ok(i) = i64::try_from(u)
		{
			let mut n = i;
			let errors = schema.apply(path, &mut n).await;
			*value = Value::Int(n);
			return errors;
		}
		return type_mismatch(path, "i64", value);
	};
	let errors = schema.apply(path, &mut n).await;
	*value = Value::Int(n);
	errors
}

async fn validate_u64(
	schema: &U64Schema,
	path: &FieldPath,
	value: &mut Value,
) -> Vec<ValidationError> {
	let Value::Uint(mut n) = *value else {
		if let Value::Int(i) = *value
			&& let Ok(u) = u64::try_from(i)
		{
			let mut n = u;
			let errors = schema.apply(path, &mut n).await;
			*value = Value::Uint(n);
			return errors;
		}
		return type_mismatch(path, "u64", value);
	};
	let errors = schema.apply(path, &mut n).await;
	*value = Value::Uint(n);
	errors
}

async fn validate_f64(
	schema: &F64Schema,
	path: &FieldPath,
	value: &mut Value,
) -> Vec<ValidationError> {
	let mut n = match *value {
		Value::Float(f) => f,
		Value::Int(i) => i as f64,
		Value::Uint(u) => u as f64,
		_ => return type_mismatch(path, "f64", value),
	};
	let errors = schema.apply(path, &mut n).await;
	*value = Value::Float(n);
	errors
}

async fn validate_string(
	schema: &StringSchema,
	path: &FieldPath,
	value: &mut Value,
) -> Vec<ValidationError> {
	let Value::Str(s) = value else {
		return type_mismatch(path, "string", value);
	};
	schema.apply(path, s).await
}

async fn validate_bytes(
	schema: &BytesSchema,
	path: &FieldPath,
	value: &mut Value,
) -> Vec<ValidationError> {
	let Value::Bytes(b) = value else {
		return type_mismatch(path, "bytes", value);
	};
	schema.apply(path, b).await
}

async fn validate_struct(
	schema: &StructSchema,
	path: &FieldPath,
	value: &mut Value,
) -> Vec<ValidationError> {
	let Value::Map(map) = value else {
		return type_mismatch(path, "struct", value);
	};
	let mut errors = Vec::new();
	for field in &schema.fields {
		match map.0.get_mut(field.key.as_str()) {
			Some(child) => {
				let sub = path.with_pushed(field.key.clone());
				errors.extend(field.schema.apply(&sub, child).await);
			}
			None if field.required => {
				errors.push(ValidationError::new(
					path.with_pushed(field.key.clone()),
					format!("missing required field `{}`", field.key),
				));
			}
			None => {}
		}
	}
	if !schema.allow_additional {
		let allowed: HashSet<&str> =
			schema.fields.iter().map(|f| f.key.as_str()).collect();
		for key in map.0.keys() {
			if !allowed.contains(key.as_str()) {
				errors.push(ValidationError::new(
					path.with_pushed(key.clone()),
					format!("unknown field `{}`", key),
				));
			}
		}
	}
	errors
}

async fn validate_tuple(
	schema: &TupleSchema,
	path: &FieldPath,
	value: &mut Value,
) -> Vec<ValidationError> {
	let Value::List(list) = value else {
		return type_mismatch(path, "tuple", value);
	};
	let mut errors = Vec::new();
	if list.len() != schema.fields.len() {
		errors.push(ValidationError::new(
			path.clone(),
			format!(
				"expected tuple of length {}, got {}",
				schema.fields.len(),
				list.len()
			),
		));
		return errors;
	}
	for (idx, (field, child)) in
		schema.fields.iter().zip(list.iter_mut()).enumerate()
	{
		let sub = path.with_pushed(idx);
		errors.extend(field.schema.apply(&sub, child).await);
	}
	errors
}

async fn validate_list(
	schema: &ListSchema,
	path: &FieldPath,
	value: &mut Value,
) -> Vec<ValidationError> {
	let Value::List(list) = value else {
		return type_mismatch(path, "list", value);
	};
	let mut errors = Vec::new();
	if let Some(min) = schema.min_items
		&& list.len() < min
	{
		errors.push(ValidationError::new(
			path.clone(),
			format!("must have at least {} items", min),
		));
	}
	if let Some(max) = schema.max_items
		&& list.len() > max
	{
		errors.push(ValidationError::new(
			path.clone(),
			format!("must have at most {} items", max),
		));
	}
	if schema.unique {
		let mut seen: HashSet<Value> = HashSet::default();
		for item in list.iter() {
			if !seen.insert(item.clone()) {
				errors.push(ValidationError::new(
					path.clone(),
					"items must be unique",
				));
				break;
			}
		}
	}
	for (idx, child) in list.iter_mut().enumerate() {
		let sub = path.with_pushed(idx);
		errors.extend(schema.item.apply(&sub, child).await);
	}
	errors
}

async fn validate_map(
	schema: &MapSchema,
	path: &FieldPath,
	value: &mut Value,
) -> Vec<ValidationError> {
	let Value::Map(map) = value else {
		return type_mismatch(path, "map", value);
	};
	let mut errors = Vec::new();
	for (key, child) in map.0.iter_mut() {
		let sub = path.with_pushed(key.clone());
		errors.extend(schema.value.apply(&sub, child).await);
	}
	errors
}

async fn validate_enum(
	schema: &EnumSchema,
	path: &FieldPath,
	value: &mut Value,
) -> Vec<ValidationError> {
	// Unit variant as bare string. A qualified `EnumName::Variant` (the Rust
	// path form authors reach for) is accepted by its trailing segment and
	// normalized to the bare variant name, so reflect deserialization downstream
	// (which expects the bare name) succeeds.
	if let Value::Str(name) = value {
		let variant = name.rsplit("::").next().unwrap_or(name.as_str());
		if schema
			.variants
			.iter()
			.any(|v| v.payload.is_none() && v.name.as_str() == variant)
		{
			if variant != name.as_str() {
				*value = Value::Str(variant.into());
			}
			return Vec::new();
		}
		return vec![ValidationError::new(
			path.clone(),
			format!("unknown variant `{}`", name),
		)];
	}

	// Otherwise expect `{ "VariantName": payload }`.
	let Value::Map(map) = value else {
		return type_mismatch(path, "enum", value);
	};
	if map.0.len() != 1 {
		return vec![ValidationError::new(
			path.clone(),
			"expected a single-key enum object",
		)];
	}
	let (key, payload) = map.0.iter_mut().next().expect("len == 1");
	let Some(variant) = schema
		.variants
		.iter()
		.find(|v| v.name.as_str() == key.as_str())
	else {
		return vec![ValidationError::new(
			path.clone(),
			format!("unknown variant `{}`", key),
		)];
	};
	let Some(payload_schema) = &variant.payload else {
		return vec![ValidationError::new(
			path.clone(),
			format!("variant `{}` has no payload", key),
		)];
	};
	let sub = path.with_pushed(key.clone());
	payload_schema.apply(&sub, payload).await
}

#[cfg(test)]
mod test {
	use super::*;

	#[derive(Reflect)]
	#[allow(dead_code)]
	struct UserProfile {
		name: String,
		age: u32,
		email: Option<String>,
	}

	#[derive(Reflect)]
	#[allow(dead_code)]
	enum Status {
		Active,
		Banned,
		Pending(String),
	}

	#[crate::test]
	fn primitive_schemas() {
		matches!(ValueSchema::of::<bool>(), ValueSchema::Bool(_)).xpect_true();
		matches!(ValueSchema::of::<i32>(), ValueSchema::I64(_)).xpect_true();
		matches!(ValueSchema::of::<u32>(), ValueSchema::U64(_)).xpect_true();
		matches!(ValueSchema::of::<f32>(), ValueSchema::F64(_)).xpect_true();
		matches!(ValueSchema::of::<String>(), ValueSchema::String(_))
			.xpect_true();
		matches!(ValueSchema::of::<()>(), ValueSchema::Null).xpect_true();
	}

	#[crate::test]
	fn struct_schema_from_type_info() {
		let schema = ValueSchema::of::<UserProfile>();
		let ValueSchema::Struct(s) = schema else {
			panic!("expected struct schema");
		};
		s.fields.len().xpect_eq(3);
		s.fields[0].key.as_str().xpect_eq("name");
		s.fields[0].required.xpect_true();
		// Option<String> is unwrapped to its inner schema
		s.fields[2].key.as_str().xpect_eq("email");
		s.fields[2].required.xpect_false();
	}

	#[crate::test]
	fn enum_schema_from_type_info() {
		let schema = ValueSchema::of::<Status>();
		let ValueSchema::Enum(e) = schema else {
			panic!("expected enum schema");
		};
		e.variants.len().xpect_eq(3);
		e.variants[0].name.as_str().xpect_eq("Active");
		e.variants[0].payload.is_none().xpect_true();
		e.variants[2].name.as_str().xpect_eq("Pending");
		e.variants[2].payload.is_some().xpect_true();
	}

	#[crate::test]
	async fn validate_struct_missing_field() {
		let schema = ValueSchema::of::<UserProfile>();
		let mut value = val!({
			"name": "Alice",
		});
		let errors = schema.validate(&mut value).await;
		errors.len().xpect_eq(1);
		errors[0].path.to_string().xpect_eq("age");
	}

	#[crate::test]
	async fn validate_struct_ok() {
		let schema = ValueSchema::of::<UserProfile>();
		let mut value = val!({
			"name": "Alice",
			"age": 30u64,
		});
		let errors = schema.validate(&mut value).await;
		errors.is_empty().xpect_true();
	}

	#[crate::test]
	async fn validate_struct_wrong_type() {
		let schema = ValueSchema::of::<UserProfile>();
		let mut value = val!({
			"name": "Alice",
			"age": "thirty",
		});
		let errors = schema.validate(&mut value).await;
		errors.len().xpect_eq(1);
		errors[0].path.to_string().xpect_eq("age");
	}

	#[crate::test]
	async fn validate_min_constraint() {
		let schema = ValueSchema::I64(I64Schema {
			constraints: vec![I64Constraint::Min(I64Min {
				value: 10,
				behavior: ConstraintBehavior::Error,
			})],
		});
		let mut value = val!(5);
		let errors = schema.validate(&mut value).await;
		errors.len().xpect_eq(1);
		// no mutation
		value.as_i64().unwrap().xpect_eq(5);
	}

	#[crate::test]
	async fn validate_min_mutate() {
		let schema = ValueSchema::I64(I64Schema {
			constraints: vec![I64Constraint::Min(I64Min {
				value: 10,
				behavior: ConstraintBehavior::Mutate,
			})],
		});
		let mut value = val!(5);
		let errors = schema.validate(&mut value).await;
		errors.is_empty().xpect_true();
		value.as_i64().unwrap().xpect_eq(10);
	}

	#[crate::test]
	async fn validate_string_min_length() {
		let schema = ValueSchema::String(StringSchema::default().with(
			StringConstraint::MinLength {
				value: 3,
				behavior: ConstraintBehavior::Error,
			},
		));
		let mut value = val!("hi");
		let errors = schema.validate(&mut value).await;
		errors.len().xpect_eq(1);
	}

	#[crate::test]
	async fn validate_list_unique() {
		let schema = ValueSchema::List(ListSchema {
			item: Box::new(ValueSchema::I64(I64Schema::default())),
			min_items: None,
			max_items: None,
			unique: true,
		});
		let mut value = val!([1, 2, 2]);
		let errors = schema.validate(&mut value).await;
		errors
			.iter()
			.any(|e| e.message.contains("unique"))
			.xpect_true();
	}

	#[crate::test]
	async fn validate_enum_unit() {
		let schema = ValueSchema::of::<Status>();
		let mut value = val!("Active");
		let errors = schema.validate(&mut value).await;
		errors.is_empty().xpect_true();
	}

	#[crate::test]
	async fn validate_enum_unknown_variant() {
		let schema = ValueSchema::of::<Status>();
		let mut value = val!("Nope");
		let errors = schema.validate(&mut value).await;
		errors.len().xpect_eq(1);
	}

	/// A unit variant given as the qualified `EnumName::Variant` form (the Rust
	/// path an author reaches for in markup) validates and is normalized to the
	/// bare variant name so reflect deserialization downstream succeeds.
	#[crate::test]
	async fn validate_enum_qualified_unit() {
		let schema = ValueSchema::of::<Status>();
		let mut value = val!("Status::Active");
		schema.validate(&mut value).await.is_empty().xpect_true();
		value.xpect_eq(val!("Active"));
	}

	#[crate::test]
	async fn optional_field_accepts_null_or_value() {
		// an `Option<String>` field validates a present string, a null, and an
		// absent field, but rejects a wrong-typed present value.
		let schema = ValueSchema::of::<UserProfile>();
		// present and well typed
		schema
			.validate(&mut val!({ "name": "A", "age": 1u64, "email": "a@b.c" }))
			.await
			.is_empty()
			.xpect_true();
		// explicit null is accepted by the optional
		schema
			.validate(&mut val!({ "name": "A", "age": 1u64, "email": null }))
			.await
			.is_empty()
			.xpect_true();
		// a present but wrong-typed value still fails
		schema
			.validate(&mut val!({ "name": "A", "age": 1u64, "email": 42 }))
			.await
			.is_empty()
			.xpect_false();
	}

	#[crate::test]
	fn optional_schema_built_for_option_field() {
		let schema = ValueSchema::of::<UserProfile>();
		let ValueSchema::Struct(struct_schema) = schema else {
			panic!("expected struct schema");
		};
		// `email: Option<String>` is an Optional wrapper over String.
		let email = &struct_schema.fields[2];
		matches!(email.schema, ValueSchema::Optional(_)).xpect_true();
	}

	#[crate::test]
	async fn any_matches_everything() {
		let schema = ValueSchema::Any;
		schema
			.validate(&mut val!("anything"))
			.await
			.is_empty()
			.xpect_true();
		schema.validate(&mut val!(42)).await.is_empty().xpect_true();
	}

	#[crate::test]
	fn get_field_schema_walks_struct() {
		let schema = ValueSchema::of::<UserProfile>();
		matches!(
			schema
				.get_field_schema(&[FieldSegment::key("name")])
				.unwrap(),
			ValueSchema::String(_)
		)
		.xpect_true();
		matches!(
			schema
				.get_field_schema(&[FieldSegment::key("age")])
				.unwrap(),
			ValueSchema::U64(_)
		)
		.xpect_true();
		schema
			.get_field_schema(&[FieldSegment::key("missing")])
			.is_err()
			.xpect_true();
	}

	#[crate::test]
	fn get_field_schema_walks_list() {
		let schema = ValueSchema::of::<Vec<i64>>();
		matches!(
			schema.get_field_schema(&[FieldSegment::index(0)]).unwrap(),
			ValueSchema::I64(_)
		)
		.xpect_true();
	}

	#[crate::test]
	fn get_field_schema_any_swallows_path() {
		let schema = ValueSchema::Any;
		matches!(
			schema
				.get_field_schema(&[
					FieldSegment::key("a"),
					FieldSegment::index(2)
				])
				.unwrap(),
			ValueSchema::Any
		)
		.xpect_true();
	}
}
