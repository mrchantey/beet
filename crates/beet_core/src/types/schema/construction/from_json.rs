//! Parse a JSON Schema document (or beet's compact prop-map shorthand) into a
//! [`ValueSchema`], the inverse of the JSON Schema form a `ValueSchema` produces.
//!
//! Two surfaces are accepted, disambiguated by structure:
//!
//! - **Standard JSON Schema**: an object with a `type` keyword (`"object"`,
//!   `"array"`, `"string"`, `"integer"`, `"number"`, `"boolean"`, `"null"`), or
//!   `properties`/`items`/`$ref`. `object` reads `properties`, the `required`
//!   array, and `additionalProperties`; `array` reads `items`, `minItems`,
//!   `maxItems`, and `uniqueItems`.
//! - **Compact shorthand**: an object mapping each prop name directly to a
//!   descriptor (a primitive name string, or an object with per-field
//!   `required`/`optional`/`items`/`type`).
#[cfg(feature = "json")]
mod enabled {
	use crate::prelude::*;
	use serde_json::Map;
	use serde_json::Value as Json;

	impl ValueSchema {
		/// Build a [`ValueSchema`] from a parsed JSON Schema [`Json`] value.
		pub fn from_json_value(value: &Json) -> Result<ValueSchema> {
			match value {
				// a bare string names a primitive or references another schema by name.
				Json::String(name) => Ok(primitive_or_reference(name)),
				// JSON Schema `true` accepts anything, `false` accepts nothing.
				Json::Bool(true) => Ok(ValueSchema::Any),
				Json::Bool(false) => Ok(ValueSchema::Null),
				Json::Object(map) => object_to_schema(map),
				other => {
					bevybail!("unsupported JSON schema descriptor: {other}")
				}
			}
		}
	}

	/// Build a schema from an object, dispatching on its `type` keyword or, absent
	/// one, the structural keywords it carries, falling back to the prop-map
	/// shorthand.
	fn object_to_schema(map: &Map<String, Json>) -> Result<ValueSchema> {
		let mut schema = match map.get("type").and_then(Json::as_str) {
			Some("object") => struct_schema(map)?,
			Some("array") => list_schema(map)?,
			Some(name) => primitive_or_reference(name),
			None => {
				if map.contains_key("properties") {
					struct_schema(map)?
				} else if map.contains_key("items") {
					list_schema(map)?
				} else if let Some(reference) =
					map.get("$ref").and_then(Json::as_str)
				{
					ValueSchema::Ref(SchemaRef::Name(SmolStr::from(strip_ref(
						reference,
					))))
				} else {
					// the compact shorthand: this object is a map of prop -> descriptor.
					shorthand_schema(map)?
				}
			}
		};
		// `optional`/`nullable` wrap the base schema so a missing or null value passes.
		let optional = flag(map, "optional") || flag(map, "nullable");
		if optional {
			schema = ValueSchema::Optional(Box::new(schema));
		}
		Ok(schema)
	}

	/// A standard `{"type":"object", "properties":{..}, "required":[..]}` struct.
	fn struct_schema(map: &Map<String, Json>) -> Result<ValueSchema> {
		let required: HashSet<&str> = map
			.get("required")
			.and_then(Json::as_array)
			.map(|items| items.iter().filter_map(Json::as_str).collect())
			.unwrap_or_default();
		let properties = map.get("properties").and_then(Json::as_object);
		let fields = properties
			.into_iter()
			.flatten()
			.map(|(key, descriptor)| {
				Ok(NamedFieldSchema {
					key: SmolStr::from(key.as_str()),
					required: required.contains(key.as_str()),
					label: None,
					description: None,
					on_missing: on_missing(descriptor)?,
					write: write_policy(descriptor)?,
					schema: ValueSchema::from_json_value(descriptor)?,
				})
			})
			.collect::<Result<Vec<_>>>()?;
		// an authored struct is closed unless `additionalProperties` says otherwise.
		let allow_additional = map
			.get("additionalProperties")
			.and_then(Json::as_bool)
			.unwrap_or(false);
		Ok(ValueSchema::Struct(StructSchema {
			name: None,
			description: None,
			allow_additional,
			fields,
		}))
	}

	/// A standard `{"type":"array", "items":..}` list with the size/uniqueness
	/// constraints.
	fn list_schema(map: &Map<String, Json>) -> Result<ValueSchema> {
		let item = match map.get("items") {
			Some(items) => ValueSchema::from_json_value(items)?,
			None => ValueSchema::Any,
		};
		Ok(ValueSchema::List(ListSchema {
			item: Box::new(item),
			min_items: usize_field(map, "minItems"),
			max_items: usize_field(map, "maxItems"),
			unique: flag(map, "uniqueItems"),
		}))
	}

	/// The compact shorthand: an object mapping each prop name to a descriptor, with
	/// a per-field `required` flag read off each descriptor.
	fn shorthand_schema(map: &Map<String, Json>) -> Result<ValueSchema> {
		let fields = map
			.iter()
			.map(|(key, descriptor)| {
				Ok(NamedFieldSchema {
					key: SmolStr::from(key.as_str()),
					required: descriptor
						.as_object()
						.map(|fields| flag(fields, "required"))
						.unwrap_or(false),
					label: None,
					description: None,
					on_missing: on_missing(descriptor)?,
					write: write_policy(descriptor)?,
					schema: ValueSchema::from_json_value(descriptor)?,
				})
			})
			.collect::<Result<Vec<_>>>()?;
		Ok(ValueSchema::Struct(StructSchema {
			name: None,
			description: None,
			allow_additional: false,
			fields,
		}))
	}

	/// The primitive a descriptor name declares, or a composable
	/// [`SchemaRef::Name`] to another schema when the word names no primitive.
	///
	/// The accepted primitive names are the shared vocabulary (see
	/// [`ValueSchema::primitive_by_name`]); only the fallback is local, since
	/// here an unknown word is a reference rather than an error.
	fn primitive_or_reference(name: &str) -> ValueSchema {
		ValueSchema::primitive_by_name(name).unwrap_or_else(|| {
			ValueSchema::Ref(SchemaRef::Name(SmolStr::from(name)))
		})
	}

	/// Read a field descriptor's commit-time resolution policy: `"default"` names a
	/// literal to backfill, `"computed"` a script to run. Absent means the field
	/// declares no resolution, so a commit that would leave it required-but-absent
	/// is rejected.
	fn on_missing(descriptor: &Json) -> Result<Option<OnMissing>> {
		let Some(map) = descriptor.as_object() else {
			return Ok(None);
		};
		if let Some(script) = map.get("computed").and_then(Json::as_str) {
			return Ok(Some(OnMissing::Computed {
				script: script.to_string(),
			}));
		}
		map.get("default")
			.map(|value| Value::from_serde(value).map(OnMissing::Default))
			.transpose()
	}

	/// A field's `"write"` keyword: the [`WritePolicy`] its controls write
	/// under, by variant name (`"Blur"`), absent for the kind's default.
	fn write_policy(descriptor: &Json) -> Result<Option<WritePolicy>> {
		descriptor
			.as_object()
			.and_then(|map| map.get("write"))
			.map(|policy| {
				Value::from_serde(policy)?.into_serde::<WritePolicy>()
			})
			.transpose()
	}

	/// Strip the `#/$defs/` (or `#/definitions/`) prefix off a `$ref`, leaving the
	/// referenced schema name.
	fn strip_ref(reference: &str) -> &str {
		reference.rsplit('/').next().unwrap_or(reference)
	}

	/// Read a boolean keyword, defaulting to `false` when absent or non-boolean.
	fn flag(map: &Map<String, Json>, key: &str) -> bool {
		map.get(key).and_then(Json::as_bool).unwrap_or(false)
	}

	/// Read an unsigned-integer keyword as a `usize`, if present.
	fn usize_field(map: &Map<String, Json>, key: &str) -> Option<usize> {
		map.get(key)
			.and_then(Json::as_u64)
			.map(|value| value as usize)
	}

	#[cfg(test)]
	mod test {
		use crate::prelude::*;

		fn parse(json: &str) -> Result<ValueSchema> {
			ValueSchema::from_json_value(&serde_json::from_str(json)?)
		}

		#[crate::test]
		fn shorthand_struct() {
			let ValueSchema::Struct(schema) = parse(
				r#"{ "label": "string", "count": { "type": "i64", "required": true } }"#,
			)
			.unwrap() else {
				panic!("expected struct");
			};
			schema.fields.len().xpect_eq(2);
			let count = schema
				.fields
				.iter()
				.find(|field| field.key == "count")
				.unwrap();
			count.required.xpect_true();
			matches!(count.schema, ValueSchema::I64(_)).xpect_true();
		}

		/// A `write` keyword declares the field's policy; absent, the kind's
		/// default applies at the control.
		#[crate::test]
		fn write_keyword() {
			let ValueSchema::Struct(schema) = parse(
				r#"{ "label": { "type": "string", "write": "Blur" }, "note": "string" }"#,
			)
			.unwrap() else {
				panic!("expected struct");
			};
			schema.fields[0].write.xpect_eq(Some(WritePolicy::Blur));
			schema.fields[1].write.xpect_none();
		}

		#[crate::test]
		fn standard_object() {
			let ValueSchema::Struct(schema) = parse(
				r#"{
				"type": "object",
				"required": ["name"],
				"properties": {
					"name": { "type": "string" },
					"tags": { "type": "array", "items": { "type": "string" } }
				}
			}"#,
			)
			.unwrap() else {
				panic!("expected struct");
			};
			schema.fields.len().xpect_eq(2);
			let name = schema
				.fields
				.iter()
				.find(|field| field.key == "name")
				.unwrap();
			name.required.xpect_true();
			let tags = schema
				.fields
				.iter()
				.find(|field| field.key == "tags")
				.unwrap();
			matches!(tags.schema, ValueSchema::List(_)).xpect_true();
		}

		#[crate::test]
		fn reference_via_ref() {
			parse(r##"{ "$ref": "#/$defs/TodoItem" }"##)
				.unwrap()
				.xpect_eq(ValueSchema::reference("TodoItem"));
		}
	}
}
