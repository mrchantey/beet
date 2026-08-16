//! Flattens BiDi remote values into plain JSON.
//!
//! `script.evaluate` responses encode results as tagged remote values, eg
//! `{"type": "object", "value": [["count", {"type": "number", "value": 0}]]}`.
//! [`to_json`] collapses that encoding so consumers compare against ordinary
//! [`serde_json::Value`] literals.

use serde_json::Map;
use serde_json::Value;
use serde_json::json;

/// Convert a BiDi remote value into plain JSON.
///
/// Non-serializable values (nodes, functions, promises, values beyond the
/// requested serialization depth) become `null`; the JS number specials
/// (`NaN`, `Infinity`, `-Infinity`) also become `null` since JSON cannot
/// carry them, and `-0` becomes `0`.
pub(crate) fn to_json(remote: &Value) -> Value {
	let ty = remote
		.get("type")
		.and_then(|ty| ty.as_str())
		.unwrap_or_default();
	let value = remote.get("value");
	match (ty, value) {
		("string" | "date", Some(value)) => value.clone(),
		("boolean", Some(value)) => value.clone(),
		("number", Some(value)) => match value.as_str() {
			Some("-0") => json!(0),
			// NaN / Infinity / -Infinity have no JSON representation
			Some(_) => Value::Null,
			None => value.clone(),
		},
		("bigint", Some(value)) => value
			.as_str()
			.and_then(|digits| digits.parse::<i64>().ok())
			.map(|num| json!(num))
			.unwrap_or_else(|| value.clone()),
		("array" | "set" | "nodelist" | "htmlcollection", Some(value)) => value
			.as_array()
			.map(|items| Value::Array(items.iter().map(to_json).collect()))
			.unwrap_or(Value::Null),
		("object" | "map", Some(value)) => value
			.as_array()
			.map(|entries| {
				Value::Object(
					entries
						.iter()
						.filter_map(|entry| entry.as_array())
						.filter_map(|pair| match pair.as_slice() {
							[key, val] => Some((entry_key(key), to_json(val))),
							_ => None,
						})
						.collect::<Map<_, _>>(),
				)
			})
			.unwrap_or(Value::Null),
		("regexp", Some(value)) => {
			let pattern = value
				.pointer("/pattern")
				.and_then(|p| p.as_str())
				.unwrap_or_default();
			let flags = value
				.pointer("/flags")
				.and_then(|f| f.as_str())
				.unwrap_or_default();
			json!(format!("/{pattern}/{flags}"))
		}
		// undefined, null, node, window, function, promise, depth-exceeded
		_ => Value::Null,
	}
}

/// An object entry key is either a plain string or itself a remote value.
fn entry_key(key: &Value) -> String {
	match key {
		Value::String(key) => key.clone(),
		remote => match to_json(remote) {
			Value::String(key) => key,
			other => other.to_string(),
		},
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use beet_core::prelude::*;
	use serde_json::Value;

	#[beet_core::test]
	fn flattens_nested_remote_values() {
		to_json(&json!({
			"type": "object",
			"value": [
				["count", {"type": "number", "value": 0}],
				["flag", {"type": "boolean", "value": false}],
				["status", {"type": "string", "value": "pending"}],
				["items", {"type": "array", "value": [
					{"type": "number", "value": 1},
					{"type": "null"}
				]}],
			]
		}))
		.xpect_eq(json!({
			"count": 0,
			"flag": false,
			"status": "pending",
			"items": [1, null],
		}));
	}

	#[beet_core::test]
	fn handles_specials() {
		to_json(&json!({"type": "number", "value": "NaN"}))
			.xpect_eq(Value::Null);
		to_json(&json!({"type": "number", "value": "-0"})).xpect_eq(json!(0));
		to_json(&json!({"type": "undefined"})).xpect_eq(Value::Null);
		to_json(&json!({"type": "node", "sharedId": "abc"}))
			.xpect_eq(Value::Null);
	}
}
