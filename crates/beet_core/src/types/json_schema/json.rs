//! JSON conversion for [`JsonSchema`].
use crate::prelude::*;

impl From<JsonSchema> for serde_json::Value {
	fn from(schema: JsonSchema) -> Self { schema.into_inner().into_json() }
}

impl From<serde_json::Value> for JsonSchema {
	fn from(json: serde_json::Value) -> Self {
		JsonSchema::from_value(Value::from_json(json))
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[crate::test]
	fn preserves_wrapper_shape() {
		let value = value!({
			"type": "object",
			"properties": {}
		});
		let json =
			serde_json::Value::from(JsonSchema::from_value(value.clone()));
		json.xpect_eq(serde_json::json!({
			"type": "object",
			"properties": {}
		}));
		JsonSchema::from(json).into_inner().xpect_eq(value);
	}
}
