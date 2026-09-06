//! Validation entrypoints, recursive walk and errors for [`ValueSchema`].
use super::scalar_constraints::ApplyConstraints;
use super::scalar_constraints::ApplyFuture;
use crate::prelude::*;

/// An error produced by validating a [`Value`] against a [`ValueSchema`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ValidationError {
	/// The path within the root value where the error occurred.
	pub path: FieldPath,
	/// A human readable description of what failed.
	pub message: SmolStr,
}

impl ValidationError {
	/// Create a new validation error.
	pub fn new(path: FieldPath, message: impl Into<SmolStr>) -> Self {
		Self {
			path,
			message: message.into(),
		}
	}
}

impl core::fmt::Display for ValidationError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		if self.path.is_empty() {
			write!(f, "{}", self.message)
		} else {
			write!(f, "{}: {}", self.path, self.message)
		}
	}
}

impl ValueSchema {
	/// Validate (and possibly mutate) `value` against this schema.
	///
	/// Returns the list of [`ValidationError`]s collected; an empty list means
	/// the value is valid. A [`ValueSchema::Ref`] is a wildcard here,
	/// since nothing in hand can resolve it; use
	/// [`validate_in`](Self::validate_in) to follow references.
	pub async fn validate(&self, value: &mut Value) -> Vec<ValidationError> {
		self.validate_in(SchemaResolver::default(), value).await
	}

	/// [`validate`](Self::validate), resolving each [`ValueSchema::Ref`]
	/// against `resolver` where the walk meets it.
	///
	/// Lazy rather than eager, so a self-recursive schema (a composed authored
	/// schema, the meta-schema) descends exactly as far as the data does
	/// instead of being expanded ahead of time.
	pub async fn validate_in(
		&self,
		resolver: SchemaResolver<'_>,
		value: &mut Value,
	) -> Vec<ValidationError> {
		let path = FieldPath::default();
		self.apply_in(resolver, &path, value).await
	}

	/// Validate `value`, collecting every error into one [`Result`] naming
	/// `subject` (the document, field or commit the value belongs to).
	///
	/// The read backstop: data must never observably violate its schema, so a
	/// document that diverged outside the editor fails loudly here rather than
	/// being silently patched. [`OnMissing`] policies deliberately play no part;
	/// they belong to [`SchemaCommit`].
	pub async fn assert_valid(
		&self,
		subject: &str,
		value: &mut Value,
	) -> Result {
		self.assert_valid_in(SchemaResolver::default(), subject, value)
			.await
	}

	/// [`assert_valid`](Self::assert_valid), resolving references against
	/// `resolver`.
	pub async fn assert_valid_in(
		&self,
		resolver: SchemaResolver<'_>,
		subject: &str,
		value: &mut Value,
	) -> Result {
		self.validate_in(resolver, value).await.xmap(|errors| {
			match errors.is_empty() {
				true => OK,
				false => bevybail!(
					"{subject} does not match its schema:\n{}",
					errors
						.iter()
						.map(ToString::to_string)
						.collect::<Vec<_>>()
						.join("\n")
				),
			}
		})
	}

	/// The walk every validation entrypoint runs, carrying the `resolver` a
	/// [`ValueSchema::Ref`] resolves through.
	fn apply_in<'a>(
		&'a self,
		resolver: SchemaResolver<'a>,
		path: &'a FieldPath,
		value: &'a mut Value,
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
				ValueSchema::Entity(schema) => {
					validate_entity(schema, path, value).await
				}
				ValueSchema::Struct(schema) => {
					validate_struct(resolver, schema, path, value).await
				}
				ValueSchema::Tuple(schema) => {
					validate_tuple(resolver, schema, path, value).await
				}
				ValueSchema::List(schema) => {
					validate_list(resolver, schema, path, value).await
				}
				ValueSchema::Map(schema) => {
					validate_map(resolver, schema, path, value).await
				}
				ValueSchema::Enum(schema) => {
					validate_enum(resolver, schema, path, value).await
				}
				ValueSchema::Optional(inner) => {
					// a null satisfies an optional; anything else validates as the
					// inner schema.
					if matches!(value, Value::Null) {
						Vec::new()
					} else {
						inner.apply_in(resolver, path, value).await
					}
				}
				// a reference the resolver answers is followed here rather than
				// expanded ahead of time; one it cannot (unregistered, still
				// arriving, or a cycle) is a wildcard, so validation defers. An
				// `AtField` never reaches here: the struct holding it binds it
				// before descending, which is the only scope it resolves against.
				ValueSchema::Ref(schema_ref) => {
					match resolver.follow(schema_ref) {
						Some(target) => {
							target.apply_in(resolver, path, value).await
						}
						None => Vec::new(),
					}
				}
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
	let Value::Int(mut number) = *value else {
		// allow coercion from Uint that fits
		if let Value::Uint(unsigned) = *value
			&& let Ok(signed) = i64::try_from(unsigned)
		{
			let mut number = signed;
			let errors = schema.apply(path, &mut number).await;
			*value = Value::Int(number);
			return errors;
		}
		return type_mismatch(path, "i64", value);
	};
	let errors = schema.apply(path, &mut number).await;
	*value = Value::Int(number);
	errors
}

async fn validate_u64(
	schema: &U64Schema,
	path: &FieldPath,
	value: &mut Value,
) -> Vec<ValidationError> {
	let Value::Uint(mut number) = *value else {
		if let Value::Int(signed) = *value
			&& let Ok(unsigned) = u64::try_from(signed)
		{
			let mut number = unsigned;
			let errors = schema.apply(path, &mut number).await;
			*value = Value::Uint(number);
			return errors;
		}
		return type_mismatch(path, "u64", value);
	};
	let errors = schema.apply(path, &mut number).await;
	*value = Value::Uint(number);
	errors
}

async fn validate_f64(
	schema: &F64Schema,
	path: &FieldPath,
	value: &mut Value,
) -> Vec<ValidationError> {
	let mut number = match *value {
		Value::Float(float) => float,
		Value::Int(signed) => signed as f64,
		Value::Uint(unsigned) => unsigned as f64,
		_ => return type_mismatch(path, "f64", value),
	};
	let errors = schema.apply(path, &mut number).await;
	*value = Value::Float(number);
	errors
}

async fn validate_string(
	schema: &StringSchema,
	path: &FieldPath,
	value: &mut Value,
) -> Vec<ValidationError> {
	let Value::Str(string) = value else {
		return type_mismatch(path, "string", value);
	};
	schema.apply(path, string).await
}

async fn validate_bytes(
	schema: &BytesSchema,
	path: &FieldPath,
	value: &mut Value,
) -> Vec<ValidationError> {
	// a transport with no byte type (json, and so every script host) carries
	// bytes as a list of numbers, so the destination restores the type rather
	// than the wire announcing it.
	if let Value::List(items) = value
		&& let Some(bytes) = as_bytes(items)
	{
		*value = Value::Bytes(bytes);
	}
	let Value::Bytes(bytes) = value else {
		return type_mismatch(path, "bytes", value);
	};
	schema.apply(path, bytes).await
}

/// `items` as bytes, when every one of them is a byte-sized integer.
fn as_bytes(items: &[Value]) -> Option<Vec<u8>> {
	items
		.iter()
		.map(|item| match item {
			Value::Uint(byte) => u8::try_from(*byte).ok(),
			Value::Int(byte) => u8::try_from(*byte).ok(),
			_ => None,
		})
		.collect()
}

/// An entity reference is a node key: the generation-stripped index the
/// surrounding document keys its nodes by, so it reads as an unsigned integer.
/// That the key names a live node is checked where a world is in hand (the
/// build path's entity map), not here.
async fn validate_entity(
	schema: &EntitySchema,
	path: &FieldPath,
	value: &mut Value,
) -> Vec<ValidationError> {
	let Value::Uint(mut key) = *value else {
		// an entity key written as a signed integer (every json number parses
		// signed) coerces when it fits.
		if let Value::Int(signed) = *value
			&& let Ok(unsigned) = u64::try_from(signed)
		{
			let mut key = unsigned;
			let errors = schema.apply(path, &mut key).await;
			*value = Value::Uint(key);
			return errors;
		}
		return type_mismatch(path, "entity", value);
	};
	let errors = schema.apply(path, &mut key).await;
	*value = Value::Uint(key);
	errors
}

async fn validate_struct(
	resolver: SchemaResolver<'_>,
	schema: &StructSchema,
	path: &FieldPath,
	value: &mut Value,
) -> Vec<ValidationError> {
	let Value::Map(map) = value else {
		return type_mismatch(path, "struct", value);
	};
	let mut errors = Vec::new();
	// a field described by a sibling is bound here, while the whole struct is
	// still in hand: this is the only scope a `SchemaRef::AtField` resolves
	// against, and binding before the descent is what keeps the walk itself
	// value-independent.
	let bound = schema
		.fields
		.iter()
		.map(|field| {
			field.schema.binds_a_field().then(|| field.schema.bind(map))
		})
		.collect::<Vec<_>>();
	for (field, bound) in schema.fields.iter().zip(bound.iter()) {
		let field_schema = bound.as_ref().unwrap_or(&field.schema);
		match map.0.get_mut(field.key.as_str()) {
			Some(child) => {
				let sub = path.with_pushed(field.key.clone());
				errors
					.extend(field_schema.apply_in(resolver, &sub, child).await);
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
		let allowed: HashSet<&str> = schema
			.fields
			.iter()
			.map(|field| field.key.as_str())
			.collect();
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
	resolver: SchemaResolver<'_>,
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
	for (index, (field, child)) in
		schema.fields.iter().zip(list.iter_mut()).enumerate()
	{
		let sub = path.with_pushed(index);
		errors.extend(field.schema.apply_in(resolver, &sub, child).await);
	}
	errors
}

async fn validate_list(
	resolver: SchemaResolver<'_>,
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
	for (index, child) in list.iter_mut().enumerate() {
		let sub = path.with_pushed(index);
		errors.extend(schema.item.apply_in(resolver, &sub, child).await);
	}
	errors
}

async fn validate_map(
	resolver: SchemaResolver<'_>,
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
		errors.extend(schema.value.apply_in(resolver, &sub, child).await);
	}
	errors
}

async fn validate_enum(
	resolver: SchemaResolver<'_>,
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
		if schema.variants.iter().any(|variant_schema| {
			variant_schema.payload.is_none()
				&& variant_schema.name.as_str() == variant
		}) {
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
		.find(|variant| variant.name.as_str() == key.as_str())
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
	payload_schema.apply_in(resolver, &sub, payload).await
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

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
	async fn validate_struct_missing_field() {
		let schema = ValueSchema::of::<UserProfile>();
		let mut value = value!({
			"name": "Alice",
		});
		let errors = schema.validate(&mut value).await;
		errors.len().xpect_eq(1);
		errors[0].path.to_string().xpect_eq("age");
	}

	#[crate::test]
	async fn validate_struct_ok() {
		let schema = ValueSchema::of::<UserProfile>();
		let mut value = value!({
			"name": "Alice",
			"age": 30u64,
		});
		let errors = schema.validate(&mut value).await;
		errors.is_empty().xpect_true();
	}

	#[crate::test]
	async fn validate_struct_wrong_type() {
		let schema = ValueSchema::of::<UserProfile>();
		let mut value = value!({
			"name": "Alice",
			"age": "thirty",
		});
		let errors = schema.validate(&mut value).await;
		errors.len().xpect_eq(1);
		errors[0].path.to_string().xpect_eq("age");
	}

	#[crate::test]
	async fn validate_list_unique() {
		let schema = ValueSchema::List(ListSchema {
			item: Box::new(ValueSchema::I64(I64Schema::default())),
			min_items: None,
			max_items: None,
			unique: true,
		});
		let mut value = value!([1, 2, 2]);
		let errors = schema.validate(&mut value).await;
		errors
			.iter()
			.any(|error| error.message.contains("unique"))
			.xpect_true();
	}

	#[crate::test]
	async fn validate_enum_unit() {
		let schema = ValueSchema::of::<Status>();
		let mut value = value!("Active");
		let errors = schema.validate(&mut value).await;
		errors.is_empty().xpect_true();
	}

	#[crate::test]
	async fn validate_enum_unknown_variant() {
		let schema = ValueSchema::of::<Status>();
		let mut value = value!("Nope");
		let errors = schema.validate(&mut value).await;
		errors.len().xpect_eq(1);
	}

	/// A unit variant given as the qualified `EnumName::Variant` form (the Rust
	/// path an author reaches for in markup) validates and is normalized to the
	/// bare variant name so reflect deserialization downstream succeeds.
	#[crate::test]
	async fn validate_enum_qualified_unit() {
		let schema = ValueSchema::of::<Status>();
		let mut value = value!("Status::Active");
		schema.validate(&mut value).await.is_empty().xpect_true();
		value.xpect_eq(value!("Active"));
	}

	#[crate::test]
	async fn optional_field_accepts_null_or_value() {
		// an `Option<String>` field validates a present string, a null, and an
		// absent field, but rejects a wrong-typed present value.
		let schema = ValueSchema::of::<UserProfile>();
		// present and well typed
		schema
			.validate(
				&mut value!({ "name": "A", "age": 1u64, "email": "a@b.c" }),
			)
			.await
			.is_empty()
			.xpect_true();
		// explicit null is accepted by the optional
		schema
			.validate(&mut value!({ "name": "A", "age": 1u64, "email": null }))
			.await
			.is_empty()
			.xpect_true();
		// a present but wrong-typed value still fails
		schema
			.validate(&mut value!({ "name": "A", "age": 1u64, "email": 42 }))
			.await
			.is_empty()
			.xpect_false();
	}

	#[crate::test]
	async fn any_matches_everything() {
		let schema = ValueSchema::Any;
		schema
			.validate(&mut value!("anything"))
			.await
			.is_empty()
			.xpect_true();
		schema
			.validate(&mut value!(42))
			.await
			.is_empty()
			.xpect_true();
	}

	/// An [`Entity`] field is its own schema kind, not a number: a UI dispatches
	/// a node picker on it, and the serde layer routes it through the entity map.
	#[crate::test]
	async fn entity_is_its_own_kind() {
		let schema = ValueSchema::of::<Entity>();
		schema.clone().xpect_eq(ValueSchema::Entity(default()));
		// a node key validates, and a signed json number coerces to one
		schema
			.validate(&mut value!(3u64))
			.await
			.is_empty()
			.xpect_true();
		let mut signed = value!(3);
		schema.validate(&mut signed).await.is_empty().xpect_true();
		signed.xpect_eq(value!(3u64));
		// anything that is not a key does not
		schema
			.validate(&mut value!("some-entity"))
			.await
			.is_empty()
			.xpect_false();
	}
}
