//! Flattens BiDi remote values into plain JSON.
//!
//! `script.evaluate` responses encode results as tagged remote values, eg
//! `{"type": "object", "value": [["count", {"type": "number", "value": 0}]]}`.
//! [`to_json`] collapses that encoding so consumers compare against ordinary
//! [`Value`] literals.

use beet_core::prelude::*;

/// Convert a BiDi remote value into plain JSON.
///
/// Non-serializable values (nodes, functions, promises, values beyond the
/// requested serialization depth) become `null`; the JS number specials
/// (`NaN`, `Infinity`, `-Infinity`) also become `null` since JSON cannot
/// carry them, and `-0` becomes `0`.
pub(crate) fn to_json(remote: &Value) -> Value {
	let ty = remote
		.get("type")
		.and_then(|ty| ty.as_str().ok())
		.unwrap_or_default();
	let value = remote.get("value");
	match (ty, value) {
		("string" | "date", Some(value)) => value.clone(),
		("boolean", Some(value)) => value.clone(),
		("number", Some(value)) => match value.as_str() {
			Ok("-0") => Value::Uint(0),
			// NaN / Infinity / -Infinity have no JSON representation
			Ok(_) => Value::Null,
			Err(_) => value.clone(),
		},
		("bigint", Some(value)) => value
			.as_str()
			.ok()
			.and_then(|digits| digits.parse::<i64>().ok())
			.map(Value::from)
			.unwrap_or_else(|| value.clone()),
		("array" | "set" | "nodelist" | "htmlcollection", Some(value)) => value
			.as_list()
			.map(|items| Value::List(items.iter().map(to_json).collect()))
			.unwrap_or(Value::Null),
		("object" | "map", Some(value)) => value
			.as_list()
			.map(|entries| {
				Value::Map(
					entries
						.iter()
						.filter_map(|entry| entry.as_list().ok())
						.filter_map(|pair| match pair.as_slice() {
							[key, val] => Some((entry_key(key), to_json(val))),
							_ => None,
						})
						.collect::<Map>(),
				)
			})
			.unwrap_or(Value::Null),
		("regexp", Some(value)) => {
			let pattern = value["pattern"].as_str().unwrap_or_default();
			let flags = value["flags"].as_str().unwrap_or_default();
			Value::str(format!("/{pattern}/{flags}"))
		}
		// undefined, null, node, window, function, promise, depth-exceeded
		_ => Value::Null,
	}
}

/// An object entry key is either a plain string or itself a remote value.
fn entry_key(key: &Value) -> SmolStr {
	match key {
		Value::Str(key) => key.clone(),
		remote => match to_json(remote) {
			Value::Str(key) => key,
			other => other.to_json_string().into(),
		},
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	fn flattens_nested_remote_values() {
		to_json(&value!({
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
		.xpect_eq(value!({
			"count": 0,
			"flag": false,
			"status": "pending",
			"items": [1, null],
		}));
	}

	#[beet_core::test]
	fn handles_specials() {
		to_json(&value!({"type": "number", "value": "NaN"}))
			.xpect_eq(Value::Null);
		to_json(&value!({"type": "number", "value": "-0"})).xpect_eq(value!(0));
		to_json(&value!({"type": "undefined"})).xpect_eq(Value::Null);
		to_json(&value!({"type": "node", "sharedId": "abc"}))
			.xpect_eq(Value::Null);
	}
}
