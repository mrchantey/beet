//! Field schema traversal and compatibility checking.
use crate::prelude::*;

impl ValueSchema {
	/// Resolve the schema of a nested field by `path`.
	///
	/// The dual of [`Document::get_field_ref`](crate::prelude::Document):
	/// descends into struct fields, map values, list items, tuple elements and
	/// an externally tagged enum's payload (keyed by its variant name).
	/// [`ValueSchema::Any`] swallows the remaining path and matches anything,
	/// as does a [`ValueSchema::Ref`] nothing in hand can resolve.
	pub fn get_field_schema(
		&self,
		path: &[FieldSegment],
	) -> Result<&ValueSchema> {
		self.get_field_schema_in(SchemaResolver::default(), path)
	}

	/// [`get_field_schema`](Self::get_field_schema), descending through a
	/// [`ValueSchema::Ref`] that `resolver` can resolve, so a field of a
	/// composed authored schema is reachable.
	pub fn get_field_schema_in<'a>(
		&'a self,
		resolver: SchemaResolver<'a>,
		path: &[FieldSegment],
	) -> Result<&'a ValueSchema> {
		let mut current = self;
		let mut remaining = path;
		while let Some(segment) = remaining.first() {
			current = match current {
				// `Any` matches the rest of the path
				ValueSchema::Any => return Ok(current),
				// a reference descends into its target, or swallows the rest of
				// the path like `Any` while it is still arriving
				ValueSchema::Ref(SchemaRef::Name(name)) => {
					match resolver.schema(name) {
						Some(target) => target,
						None => return Ok(current),
					}
				}
				// an optional descends into its inner schema for the same segment
				ValueSchema::Optional(inner) => inner,
				_ => {
					remaining = &remaining[1..];
					match (current, segment) {
						(
							ValueSchema::Struct(schema),
							FieldSegment::ObjectKey(key),
						) => {
							&schema
								.fields
								.iter()
								.find(|field| field.key == *key)
								.ok_or_else(|| {
									bevyhow!("schema has no field `{key}`")
								})?
								.schema
						}
						// a keyed map's entry is whatever its key names
						(
							ValueSchema::Map(schema),
							FieldSegment::ObjectKey(key),
						) => schema.entry_schema(resolver, key)?,
						// an enum is externally tagged, so its payload sits
						// under the variant name the value itself carries: a
						// schema document's `Struct.fields` is this hop then a
						// struct one.
						(
							ValueSchema::Enum(schema),
							FieldSegment::ObjectKey(key),
						) => schema
							.variants
							.iter()
							.find(|variant| variant.name == *key)
							.and_then(|variant| variant.payload.as_ref())
							.ok_or_else(|| {
								bevyhow!(
									"enum schema has no variant `{key}` carrying a payload"
								)
							})?,
						(
							ValueSchema::List(schema),
							FieldSegment::ArrayIndex(_),
						) => schema.item.as_ref(),
						(
							ValueSchema::Tuple(schema),
							FieldSegment::ArrayIndex(index),
						) => {
							&schema
								.fields
								.get(*index)
								.ok_or_else(|| {
									bevyhow!(
										"tuple schema has no element {index}"
									)
								})?
								.schema
						}
						(schema, segment) => bevybail!(
							"cannot resolve segment `{segment}` against schema `{schema:?}`"
						),
					}
				}
			};
		}
		Ok(current)
	}

	/// Assert this schema is exactly `other`, naming both on mismatch.
	///
	/// Strict equality, unlike [`matches`](Self::matches): the token layer
	/// identifies a value by the schema it declares, so a near-miss is an error
	/// rather than a coercion.
	pub fn assert_eq(&self, other: &ValueSchema) -> Result<&Self> {
		match self == other {
			true => self.xok(),
			false => bevybail!(
				"Schema Mismatch\nExpected: `{other}`\nReceived: `{self}`"
			),
		}
	}

	/// Assert this schema is the one naming `T` by its type path
	/// ([`type_ref`](Self::type_ref)), the identity a token declares.
	pub fn assert_eq_ty<T: TypePath>(&self) -> Result<&Self> {
		self.assert_eq(&Self::type_ref::<T>())
	}

	/// Whether this schema is compatible with `other`, treating
	/// [`ValueSchema::Any`] and an unresolved [`ValueSchema::Ref`] on either
	/// side as a wildcard.
	pub fn matches(&self, other: &ValueSchema) -> bool {
		match (self, other) {
			// an unresolved reference or `Any` is a wildcard on either side
			(ValueSchema::Any | ValueSchema::Ref(_), _)
			| (_, ValueSchema::Any | ValueSchema::Ref(_)) => true,
			// an optional matches its bare inner and another optional's inner, so a
			// typed write of `T` validates against an `Option<T>` field
			(ValueSchema::Optional(inner), other)
			| (other, ValueSchema::Optional(inner)) => inner.matches(other),
			(left, right) => left == right,
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

	/// An enum's payload is reached by its variant name, the key the externally
	/// tagged value itself carries. This is what makes a *schema* document's
	/// own fields addressable: `Struct.fields` is the list of a struct schema's
	/// fields, which is what a schema editor binds.
	#[crate::test]
	fn get_field_schema_walks_an_enum_payload() {
		let meta = ValueSchema::meta();
		matches!(
			meta.get_field_schema(&[
				FieldSegment::key("Struct"),
				FieldSegment::key("fields")
			])
			.unwrap(),
			ValueSchema::List(_)
		)
		.xpect_true();
		// a unit variant carries no payload to descend into
		meta.get_field_schema(&[
			FieldSegment::key("Any"),
			FieldSegment::key("nope"),
		])
		.unwrap_err()
		.to_string()
		.xpect_contains("Any");
	}

	/// A keyed map's entry is the schema its key names, both mid-path and as the
	/// path's end, so a typed write into a component map is checked against
	/// the component's own schema.
	#[crate::test]
	fn get_field_schema_follows_a_keyed_map_entry() {
		let mut registry = SchemaRegistry::default();
		registry.register_type::<UserProfile>();
		let resolver = SchemaResolver::default().with_schemas(&registry);
		let schema = ValueSchema::Map(MapSchema::Keyed);
		schema
			.get_field_schema_in(resolver, &[
				FieldSegment::key("UserProfile"),
				FieldSegment::key("age"),
			])
			.unwrap()
			.xpect_eq(ValueSchema::U64(default()));
		schema
			.get_field_schema_in(resolver, &[FieldSegment::key("UserProfile")])
			.unwrap()
			.xpect_eq(ValueSchema::of::<UserProfile>());
		// an unregistered key is an error naming it
		schema
			.get_field_schema_in(resolver, &[
				FieldSegment::key("Nope"),
				FieldSegment::key("age"),
			])
			.unwrap_err()
			.to_string()
			.xpect_contains("`Nope`");
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
