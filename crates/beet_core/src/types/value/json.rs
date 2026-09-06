//! JSON conversion for [`Value`].
use crate::prelude::*;

impl Value {
	/// Converts from a [`serde_json::Value`].
	pub fn from_json(json: serde_json::Value) -> Self { json_to_value(json) }

	/// Converts into a [`serde_json::Value`].
	pub fn into_json(self) -> serde_json::Value { value_to_json(self) }

	/// Converts into a pretty-printed JSON string.
	pub fn to_string_pretty(&self) -> Result<String> {
		serde_json::to_string_pretty(self)?.xok()
	}
}

/// Converts a [`serde_json::Value`] to a [`Value`], preserving numeric precision.
///
/// Maps JSON types directly: null→Null, bool→Bool, numbers try u64 then i64 then f64,
/// string→Str, array→List, object→Map.
fn json_to_value(json: serde_json::Value) -> Value {
	match json {
		serde_json::Value::Null => Value::Null,
		serde_json::Value::Bool(b) => Value::Bool(b),
		serde_json::Value::Number(n) => {
			if let Some(u) = n.as_u64() {
				Value::Uint(u)
			} else if let Some(i) = n.as_i64() {
				Value::Int(i)
			} else if let Some(f) = n.as_f64() {
				Value::Float(f)
			} else {
				Value::Null
			}
		}
		serde_json::Value::String(s) => Value::Str(s.into()),
		serde_json::Value::Array(arr) => {
			Value::List(arr.into_iter().map(json_to_value).collect())
		}
		serde_json::Value::Object(obj) => Value::Map(
			obj.into_iter()
				.map(|(k, v)| (SmolStr::from(k), json_to_value(v)))
				.collect(),
		),
	}
}

/// Converts a [`Value`] to a [`serde_json::Value`].
///
/// Bytes are encoded as a JSON array of integers.
fn value_to_json(value: Value) -> serde_json::Value {
	match value {
		Value::Null => serde_json::Value::Null,
		Value::Bool(b) => serde_json::Value::Bool(b),
		Value::Int(i) => serde_json::Value::Number(i.into()),
		Value::Uint(u) => serde_json::Value::Number(u.into()),
		Value::Float(f) => serde_json::Number::from_f64(f)
			.map(serde_json::Value::Number)
			.unwrap_or(serde_json::Value::Null),
		Value::Bytes(bytes) => serde_json::Value::Array(
			bytes
				.into_iter()
				.map(|b| serde_json::Value::Number(b.into()))
				.collect(),
		),
		Value::Str(s) => serde_json::Value::String(s.into()),
		Value::List(list) => serde_json::Value::Array(
			list.into_iter().map(value_to_json).collect(),
		),
		Value::Map(map) => serde_json::Value::Object(
			map.into_iter()
				.map(|(k, v)| (k.into(), value_to_json(v)))
				.collect(),
		),
	}
}

impl From<serde_json::Value> for Value {
	fn from(json: serde_json::Value) -> Self { json_to_value(json) }
}

impl From<Value> for serde_json::Value {
	fn from(value: Value) -> Self { value_to_json(value) }
}

#[cfg(test)]
mod test {
	use super::*;

	#[crate::test]
	fn preserves_json_representation() {
		Value::Bytes(vec![0, 255])
			.into_json()
			.xpect_eq(serde_json::json!([0, 255]));
		Value::Float(f64::INFINITY)
			.into_json()
			.xpect_eq(serde_json::Value::Null);

		let value = Value::from_json(serde_json::json!({
			"negative": -1,
			"positive": 1
		}));
		value.get("negative").unwrap().xpect_eq(Value::Int(-1));
		value.get("positive").unwrap().xpect_eq(Value::Uint(1));
	}
}
