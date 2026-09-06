//! OpenAI strict-mode normalization for [`JsonSchema`].
use crate::prelude::*;

impl JsonSchema {
	/// Sanitizes this schema for OpenAI strict mode in place.
	///
	/// - Adds `"additionalProperties": false` to all object schemas
	/// - Converts `oneOf` to `anyOf`
	/// - Ensures all properties are in `required`
	pub fn sanitize_for_strict_mode(&mut self) -> &mut Self {
		sanitize_value_for_strict_mode(self);
		self
	}
}

fn sanitize_value_for_strict_mode(value: &mut Value) {
	let Value::Map(obj) = value else { return };

	// recurse into nested schemas
	for key in ["properties", "items", "prefixItems", "$defs"] {
		if let Some(nested) = obj.get_mut(key) {
			match nested {
				Value::Map(map) => {
					for val in map.values_mut() {
						sanitize_value_for_strict_mode(val);
					}
				}
				Value::List(arr) => {
					for val in arr.iter_mut() {
						sanitize_value_for_strict_mode(val);
					}
				}
				_ => {}
			}
		}
	}

	// convert oneOf to anyOf (OpenAI strict mode forbids oneOf)
	if let Some(one_of) = obj.remove("oneOf") {
		obj.insert("anyOf", one_of);
	}

	// recurse into anyOf / allOf variants
	for key in ["anyOf", "allOf"] {
		if let Some(Value::List(arr)) = obj.get_mut(key) {
			for val in arr.iter_mut() {
				sanitize_value_for_strict_mode(val);
			}
		}
	}

	// ensure all properties are required (OpenAI strict mode requirement)
	if let Some(Value::Map(props)) = obj.get("properties").ok().cloned() {
		let all_keys: Vec<Value> =
			props.keys().map(|k| Value::Str(k.clone())).collect();
		if !all_keys.is_empty() {
			obj.insert("required", all_keys);
		}
	}

	// add additionalProperties: false to objects without it
	if obj.get("type").and_then(|t| t.as_str()).ok() == Some("object") {
		if !obj.contains_key("additionalProperties") {
			obj.insert("additionalProperties", false);
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[crate::test]
	fn normalizes_openai_strict_mode() {
		let mut schema = JsonSchema::from_value(value!({
			"type": "object",
			"properties": {
				"name": { "type": "string" }
			},
			"oneOf": [{
				"type": "object",
				"properties": {
					"id": { "type": "integer" }
				}
			}]
		}));

		schema.sanitize_for_strict_mode();
		schema
			.get("additionalProperties")
			.unwrap()
			.xpect_eq(Value::Bool(false));
		schema.get("oneOf").is_some().xpect_false();
		let variant = schema
			.get("anyOf")
			.unwrap()
			.as_list()
			.unwrap()
			.first()
			.unwrap();
		variant
			.get("additionalProperties")
			.unwrap()
			.xpect_eq(Value::Bool(false));
		variant
			.get("required")
			.unwrap()
			.as_list()
			.unwrap()
			.contains(&Value::str("id"))
			.xpect_true();
	}
}
