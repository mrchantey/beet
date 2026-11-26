//! Writing a [`Value`] as JSON text, without serde, the other half of
//! [`read_json`](super::read_json).
//!
//! [`Value`] is beet's own JSON data model, so its text is beet's own too: a
//! generator whose output must exist in every build (a
//! `<script type="application/ld+json">` block, a site's search index) needs
//! no `json` feature, and what is written here reads back through
//! [`Value::parse_json`] exactly, floats included.

use crate::prelude::*;
use core::fmt::Write;

impl Value {
	/// This value as compact JSON text.
	///
	/// - **Map entries are sorted by key**, so one value always writes one
	///   document, which a snapshot, a cache key and a diff all need.
	/// - **A float is the shortest text that reads back as it**, always with a
	///   point or an exponent, so it never reads back as an integer.
	/// - **A non-finite float is `null`**, the only thing JSON has for one.
	/// - **Bytes are an array of their integers**, the shape the serde bridge
	///   writes; JSON has no byte string, so they read back as a list.
	/// - **A string escapes what JSON requires and nothing else**: quotes,
	///   backslashes and control characters, the short escape where there is
	///   one.
	pub fn to_json_string(&self) -> String {
		JsonWriter::new(false).write(self)
	}

	/// This value as indented JSON text, two spaces a level and a space after
	/// each colon, otherwise as [`to_json_string`](Self::to_json_string)
	/// writes it. The layout is `serde_json`'s pretty printer's, so a file
	/// either wrote diffs clean against the other.
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// value!({ "b": [1, 2], "a": {} })
	/// 	.to_json_pretty()
	/// 	.xpect_eq("{\n  \"a\": {},\n  \"b\": [\n    1,\n    2\n  ]\n}");
	/// ```
	pub fn to_json_pretty(&self) -> String { JsonWriter::new(true).write(self) }
}

/// JSON text being written.
struct JsonWriter {
	out: String,
	/// Whether to indent, see [`Value::to_json_pretty`].
	pretty: bool,
	/// How many containers the writer is inside.
	depth: usize,
}

impl JsonWriter {
	const INDENT: &'static str = "  ";
	/// The bytes a string literal escapes, by value: a table, since a
	/// string's every byte is looked up.
	const ESCAPED: [bool; 256] = {
		let mut table = [false; 256];
		let mut byte = 0;
		while byte < 0x20 {
			table[byte] = true;
			byte += 1;
		}
		table[b'"' as usize] = true;
		table[b'\\' as usize] = true;
		table
	};

	fn new(pretty: bool) -> Self {
		Self {
			out: String::new(),
			pretty,
			depth: 0,
		}
	}

	fn write(mut self, value: &Value) -> String {
		self.value(value);
		self.out
	}

	fn value(&mut self, value: &Value) {
		// writing to a `String` cannot fail
		match value {
			Value::Null => self.out.push_str("null"),
			Value::Bool(true) => self.out.push_str("true"),
			Value::Bool(false) => self.out.push_str("false"),
			Value::Int(int) => {
				if *int < 0 {
					self.out.push('-');
				}
				self.uint(int.unsigned_abs());
			}
			Value::Uint(uint) => self.uint(*uint),
			// `Debug` is the shortest round trip, a point or exponent always
			// in it
			Value::Float(float) if float.is_finite() => {
				write!(self.out, "{float:?}").unwrap()
			}
			Value::Float(_) => self.out.push_str("null"),
			Value::Bytes(bytes) => {
				self.container(['[', ']'], bytes.iter(), |writer, byte| {
					writer.uint(*byte as u64)
				})
			}
			Value::Str(text) => self.string(text),
			Value::List(items) => {
				self.container(['[', ']'], items.iter(), Self::value)
			}
			Value::Map(map) => {
				let mut entries = map.iter().collect::<Vec<_>>();
				entries.sort_unstable_by_key(|(key, _)| key.as_str());
				self.container(
					['{', '}'],
					entries.into_iter(),
					|writer, (key, value)| {
						writer.string(key);
						writer.out.push(':');
						if writer.pretty {
							writer.out.push(' ');
						}
						writer.value(value);
					},
				);
			}
		}
	}

	/// Write `items` between `brackets`, each with `item`, comma separated
	/// and, when pretty, each on its own indented line. An empty container
	/// is its brackets alone.
	fn container<T>(
		&mut self,
		[open, close]: [char; 2],
		items: impl ExactSizeIterator<Item = T>,
		mut item: impl FnMut(&mut Self, T),
	) {
		self.out.push(open);
		if items.len() == 0 {
			self.out.push(close);
			return;
		}
		self.depth += 1;
		for (index, value) in items.enumerate() {
			if index > 0 {
				self.out.push(',');
			}
			self.newline();
			item(self, value);
		}
		self.depth -= 1;
		self.newline();
		self.out.push(close);
	}

	/// A line break and the indent of this depth, when pretty.
	fn newline(&mut self) {
		if self.pretty {
			self.out.push('\n');
			for _ in 0..self.depth {
				self.out.push_str(Self::INDENT);
			}
		}
	}

	/// `uint` in decimal, without the formatting machinery.
	fn uint(&mut self, mut uint: u64) {
		let mut digits = [0u8; 20];
		let mut at = digits.len();
		loop {
			at -= 1;
			digits[at] = b'0' + (uint % 10) as u8;
			uint /= 10;
			if uint == 0 {
				break;
			}
		}
		// ascii digits, always utf8
		self.out
			.push_str(core::str::from_utf8(&digits[at..]).unwrap_or_default());
	}

	/// `text` as a string literal, quotes included.
	fn string(&mut self, text: &str) {
		self.out.reserve(text.len() + 2);
		self.out.push('"');
		let bytes = text.as_bytes();
		let mut start = 0;
		// every byte escaped is ascii, so each slice is whole chars
		while let Some(offset) = bytes[start..]
			.iter()
			.position(|byte| Self::ESCAPED[*byte as usize])
		{
			let index = start + offset;
			self.out.push_str(&text[start..index]);
			match bytes[index] {
				b'"' => self.out.push_str("\\\""),
				b'\\' => self.out.push_str("\\\\"),
				b'\n' => self.out.push_str("\\n"),
				b'\r' => self.out.push_str("\\r"),
				b'\t' => self.out.push_str("\\t"),
				0x08 => self.out.push_str("\\b"),
				0x0c => self.out.push_str("\\f"),
				byte => write!(self.out, "\\u{byte:04x}").unwrap(),
			}
			start = index + 1;
		}
		self.out.push_str(&text[start..]);
		self.out.push('"');
	}
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
		Value::Float(1e16).to_json_string().xpect_eq("1e16");
		Value::Float(-0.0).to_json_string().xpect_eq("-0.0");
		Value::Float(f64::INFINITY)
			.to_json_string()
			.xpect_eq("null");
		Value::str("a\"b\\c\nd\u{8}\u{1}é")
			.to_json_string()
			.xpect_eq(r#""a\"b\\c\nd\b\u0001é""#);
		Value::Bytes(vec![0, 255])
			.to_json_string()
			.xpect_eq("[0,255]");
	}

	#[crate::test]
	fn writes_documents() {
		let value = Value::new_list([
			value!({ "url": "/a", "count": 2, "tags": ["x"], "none": {} }),
			Value::List(Vec::new()),
		]);
		value
			.to_json_string()
			.xpect_eq(r#"[{"count":2,"none":{},"tags":["x"],"url":"/a"},[]]"#);
		value.to_json_pretty().xpect_snapshot();
	}
}
