//! Writing a [`Value`] as JSON text, without serde.
//!
//! [`Value`] is beet's own JSON data model, so being unable to print one
//! without `serde_json` is a gap rather than a convenience: a generator whose
//! output must exist in every build (a `<script type="application/ld+json">`
//! block, a site's search index) would otherwise put itself behind the `json`
//! feature. Reading json stays serde's job.

use crate::prelude::*;

impl Value {
	/// This value as compact JSON text.
	///
	/// Agrees with the serde round trip on everything a [`Value`] can hold:
	/// map entries are written sorted by key, matching the deterministic
	/// [`Map`] serializer, and a non-finite float is `null`, which is the only
	/// thing JSON has for one.
	pub fn to_json_string(&self) -> String {
		let mut out = String::new();
		self.write_json(&mut out);
		out
	}

	/// Append this value's JSON text to `out`.
	fn write_json(&self, out: &mut String) {
		match self {
			Self::Null => out.push_str("null"),
			Self::Bool(bool) => out.push_str(match bool {
				true => "true",
				false => "false",
			}),
			Self::Int(int) => out.push_str(&int.to_string()),
			Self::Uint(uint) => out.push_str(&uint.to_string()),
			// json has no infinity and no NaN, so neither does this
			Self::Float(float) => match float.is_finite() {
				true => out.push_str(&format_float(*float)),
				false => out.push_str("null"),
			},
			// bytes are a json array of their integers, the shape the serde
			// bridge writes
			Self::Bytes(bytes) => write_seq(
				out,
				bytes.iter().map(|byte| Value::Uint(*byte as u64)),
			),
			Self::Str(string) => out.push_str(&json_string(string)),
			Self::List(list) => write_seq(out, list.iter().cloned()),
			Self::Map(map) => {
				let mut entries: Vec<_> = map.iter().collect();
				entries.sort_by(|(left, _), (right, _)| left.cmp(right));
				out.push('{');
				for (index, (key, value)) in entries.into_iter().enumerate() {
					if index > 0 {
						out.push(',');
					}
					out.push_str(&json_string(key));
					out.push(':');
					value.write_json(out);
				}
				out.push('}');
			}
		}
	}
}

/// Append `values` as a `[..]` array.
fn write_seq(out: &mut String, values: impl IntoIterator<Item = Value>) {
	out.push('[');
	for (index, value) in values.into_iter().enumerate() {
		if index > 0 {
			out.push(',');
		}
		value.write_json(out);
	}
	out.push(']');
}

/// A float as JSON, which has no integer/float distinction but does require a
/// digit on each side of the point.
fn format_float(float: f64) -> String {
	match float == float.trunc() && float.abs() < 1e15 {
		true => format!("{float:.1}"),
		false => format!("{float}"),
	}
}

/// `text` as a JSON string literal, quotes included.
fn json_string(text: &str) -> String {
	let mut out = String::with_capacity(text.len() + 2);
	out.push('"');
	for char in text.chars() {
		match char {
			'"' => out.push_str("\\\""),
			'\\' => out.push_str("\\\\"),
			'\n' => out.push_str("\\n"),
			'\r' => out.push_str("\\r"),
			'\t' => out.push_str("\\t"),
			char if (char as u32) < 0x20 => {
				out.push_str(&format!("\\u{:04x}", char as u32))
			}
			char => out.push(char),
		}
	}
	out.push('"');
	out
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn writes_scalars() {
		Value::Null.to_json_string().xpect_eq("null");
		Value::Bool(true).to_json_string().xpect_eq("true");
		Value::Int(-7).to_json_string().xpect_eq("-7");
		Value::Uint(7).to_json_string().xpect_eq("7");
		Value::Float(1.5).to_json_string().xpect_eq("1.5");
		Value::Float(2.0).to_json_string().xpect_eq("2.0");
		Value::Float(f64::INFINITY)
			.to_json_string()
			.xpect_eq("null");
		Value::str("a\"b\\c\nd")
			.to_json_string()
			.xpect_eq(r#""a\"b\\c\nd""#);
		Value::Bytes(vec![0, 255])
			.to_json_string()
			.xpect_eq("[0,255]");
	}

	/// Map entries are sorted by key, so the same value always writes the same
	/// document, which is what a snapshot, a cache key and a diff all need.
	#[crate::test]
	fn writes_documents() {
		let mut map = Map::default();
		map.insert("url", Value::str("/a"));
		map.insert("count", Value::Uint(2));
		map.insert("tags", Value::new_list([Value::str("x")]));
		Value::new_list([Value::Map(map)])
			.to_json_string()
			.xpect_eq(r#"[{"count":2,"tags":["x"],"url":"/a"}]"#);
	}

	/// The written text parses back to the value it came from, so this writer
	/// and the serde bridge cannot drift.
	#[cfg(feature = "json")]
	#[crate::test]
	fn agrees_with_serde() {
		let value = Value::from_json(serde_json::json!({
			"nested": { "list": [1, -2, 1.5, true, null], "text": "a\"b\nc" },
			"bytes": [0, 255],
		}));
		serde_json::from_str::<serde_json::Value>(&value.to_json_string())
			.unwrap()
			.xmap(Value::from_json)
			.xpect_eq(value);
	}
}
