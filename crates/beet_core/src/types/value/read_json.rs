//! Reading JSON text as a [`Value`] exactly, the other half of
//! [`write_json`](super::write_json).
//!
//! The grammar is RFC 8259's, which fits on a page; the two hard parts of JSON
//! are converting floats to and from text, and both are `core`'s, correct for
//! every float. `serde_json` reads a float with a faster, inexact algorithm
//! unless built with its `float_roundtrip` feature, so a hashed document, where
//! one last digit is another hash, is read here.

use crate::prelude::*;

impl Value {
	/// The value `text` holds as JSON, every float exactly as written:
	/// whatever [`to_json_string`](Self::to_json_string) wrote reads back
	/// equal, bar bytes, which JSON has no form for.
	///
	/// A number with neither a point nor an exponent is an integer, unsigned
	/// when it is not negative, and a float past the range of either.
	///
	/// # Errors
	/// Errors with a [`JsonError`] on anything that is not one JSON document,
	/// naming the line and column, and on an object naming a key twice, which RFC 8259 leaves to
	/// each reader and I-JSON (RFC 7493) refuses: two readers taking different
	/// values from one document is how a signed or hashed one is forged.
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// Value::parse_json(r#"{"scale": 0.1}"#)
	/// 	.unwrap()
	/// 	.xpect_eq(value!({ "scale": 0.1 }));
	/// ```
	pub fn parse_json(text: &str) -> Result<Value> {
		let mut reader = JsonReader { text, at: 0 };
		let value = reader.value(0)?;
		reader.skip_space();
		match reader.peek() {
			None => value.xok(),
			Some(_) => Err(reader.error("expected the end of the document")),
		}
	}
}

/// Where and why JSON text failed to read, as JSON ([`Value::parse_json`]) or
/// as a type (`json_ext::from_str`, its message naming the path): a
/// [`SourceError`] needs the byte offset, a person the line and column.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid json at line {line} column {column}: {message}")]
pub struct JsonError {
	/// The byte offset into the text.
	pub offset: usize,
	/// The line, counting from 1.
	pub line: usize,
	/// The column in bytes, counting from 1.
	pub column: usize,
	/// What was wrong there.
	pub message: String,
}

impl JsonError {
	/// The error `message` at `offset` into `text`.
	fn new(text: &str, offset: usize, message: impl Into<String>) -> Self {
		let offset = offset.min(text.len());
		let before = &text.as_bytes()[..offset];
		Self {
			offset,
			line: before.iter().filter(|byte| **byte == b'\n').count() + 1,
			column: before
				.iter()
				.rev()
				.take_while(|byte| **byte != b'\n')
				.count() + 1,
			message: message.into(),
		}
	}

	/// The error `message` at the value `path` leads to in `text`, a JSON
	/// document already read, `None` where the path leads nowhere. Walked only
	/// once a type has refused the value, so a read that succeeds keeps no
	/// places.
	#[cfg(feature = "serde")]
	pub(crate) fn at_path(
		text: &str,
		path: &[FieldSegment],
		message: impl Into<String>,
	) -> Option<Self> {
		JsonReader { text, at: 0 }
			.locate(path)
			.ok()
			.map(|offset| Self::new(text, offset, message))
	}
}

/// A position in a JSON text.
struct JsonReader<'a> {
	text: &'a str,
	at: usize,
}

impl JsonReader<'_> {
	/// How many arrays and objects may nest, so hostile text cannot exhaust
	/// the stack: `serde_json`'s own limit.
	const MAX_DEPTH: usize = 127;

	fn peek(&self) -> Option<u8> { self.text.as_bytes().get(self.at).copied() }

	fn next(&mut self) -> Option<u8> {
		let byte = self.peek()?;
		self.at += 1;
		Some(byte)
	}

	fn skip_space(&mut self) {
		while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
			self.at += 1;
		}
	}

	/// The error `message` at this position.
	fn error(&self, message: &str) -> BevyError {
		JsonError::new(self.text, self.at, message).into()
	}

	/// The offset of the value `path` leads to from here, stepping over every
	/// value off it.
	#[cfg(feature = "serde")]
	fn locate(&mut self, path: &[FieldSegment]) -> Result<usize> {
		self.skip_space();
		let Some((segment, rest)) = path.split_first() else {
			return self.at.xok();
		};
		match (self.peek(), segment) {
			(Some(b'{'), FieldSegment::ObjectKey(key)) => {
				self.at += 1;
				loop {
					self.skip_space();
					let found = self.string()? == *key;
					self.skip_space();
					self.expect(":")?;
					if found {
						return self.locate(rest);
					}
					self.value(0)?;
					self.skip_space();
					// past the last entry, the key is not here
					self.expect(",")?;
				}
			}
			(Some(b'['), FieldSegment::ArrayIndex(index)) => {
				self.at += 1;
				for _ in 0..*index {
					self.value(0)?;
					self.skip_space();
					self.expect(",")?;
				}
				self.locate(rest)
			}
			_ => Err(self.error("no value at this path")),
		}
	}

	/// Step past `literal`, else an error naming it.
	fn expect(&mut self, literal: &str) -> Result {
		match self.text.as_bytes()[self.at..].starts_with(literal.as_bytes()) {
			true => {
				self.at += literal.len();
				Ok(())
			}
			false => Err(self.error(&format!("expected `{literal}`"))),
		}
	}

	/// The value here, inside `depth` arrays and objects.
	fn value(&mut self, depth: usize) -> Result<Value> {
		self.skip_space();
		if matches!(self.peek(), Some(b'[' | b'{')) && depth >= Self::MAX_DEPTH
		{
			return Err(self.error("nested too deeply"));
		}
		match self.peek() {
			Some(b'n') => self.expect("null").map(|_| Value::Null),
			Some(b't') => self.expect("true").map(|_| Value::Bool(true)),
			Some(b'f') => self.expect("false").map(|_| Value::Bool(false)),
			Some(b'"') => self.string().map(Value::Str),
			Some(b'[') => {
				let mut items = Vec::new();
				self.items(b']', |reader| {
					items.push(reader.value(depth + 1)?);
					Ok(())
				})?;
				Value::List(items).xok()
			}
			Some(b'{') => {
				let mut map = Map::default();
				self.items(b'}', |reader| {
					reader.skip_space();
					let start = reader.at;
					let key = reader.string()?;
					reader.skip_space();
					reader.expect(":")?;
					let value = reader.value(depth + 1)?;
					if map.insert(key.clone(), value).is_some() {
						reader.at = start;
						return Err(reader.error(&format!(
							"the key `{key}` is named twice"
						)));
					}
					Ok(())
				})?;
				Value::Map(map).xok()
			}
			Some(b'-' | b'0'..=b'9') => self.number(),
			_ => Err(self.error("expected a value")),
		}
	}

	/// Step past an opening bracket and its comma separated items, each read
	/// by `item`, to past `close`.
	fn items(
		&mut self,
		close: u8,
		mut item: impl FnMut(&mut Self) -> Result,
	) -> Result {
		self.at += 1;
		self.skip_space();
		if self.peek() == Some(close) {
			self.at += 1;
			return Ok(());
		}
		loop {
			item(self)?;
			self.skip_space();
			match self.peek() {
				Some(b',') => self.at += 1,
				Some(byte) if byte == close => {
					self.at += 1;
					return Ok(());
				}
				_ => {
					return Err(self.error(&format!(
						"expected `,` or `{}`",
						close as char
					)));
				}
			}
		}
	}

	/// Step past any digits, answering how many there were.
	fn digits(&mut self) -> usize {
		let start = self.at;
		while matches!(self.peek(), Some(b'0'..=b'9')) {
			self.at += 1;
		}
		self.at - start
	}

	fn number(&mut self) -> Result<Value> {
		let start = self.at;
		let negative = self.peek() == Some(b'-');
		if negative {
			self.at += 1;
		}
		// the integer accumulates as it is read, `None` once past a `u64`
		let digits_start = self.at;
		let mut magnitude = Some(0u64);
		while let Some(byte @ b'0'..=b'9') = self.peek() {
			magnitude = magnitude
				.and_then(|magnitude| magnitude.checked_mul(10))
				.and_then(|magnitude| {
					magnitude.checked_add((byte - b'0') as u64)
				});
			self.at += 1;
		}
		let count = self.at - digits_start;
		if count == 0
			|| (count > 1 && self.text.as_bytes()[digits_start] == b'0')
		{
			return Err(self.error("invalid number"));
		}
		let mut integer = true;
		if self.peek() == Some(b'.') {
			self.at += 1;
			integer = false;
			if self.digits() == 0 {
				return Err(self.error("expected a digit after the point"));
			}
		}
		if matches!(self.peek(), Some(b'e' | b'E')) {
			self.at += 1;
			integer = false;
			if matches!(self.peek(), Some(b'+' | b'-')) {
				self.at += 1;
			}
			if self.digits() == 0 {
				return Err(self.error("expected an exponent"));
			}
		}
		match (integer, negative, magnitude) {
			(true, false, Some(magnitude)) => {
				return Value::Uint(magnitude).xok();
			}
			// `-0` has no integer form: it is the float, as serde reads it
			(true, true, Some(magnitude)) if magnitude > 0 => {
				if let Some(int) = 0i64.checked_sub_unsigned(magnitude) {
					return Value::Int(int).xok();
				}
			}
			_ => {}
		}
		match self.text[start..self.at].parse::<f64>() {
			Ok(float) if float.is_finite() => Value::Float(float).xok(),
			_ => Err(self.error("number out of range")),
		}
	}

	fn string(&mut self) -> Result<SmolStr> {
		self.expect("\"")?;
		let first = self.at;
		let mut out = String::new();
		loop {
			let start = self.at;
			// every byte stopped at is ascii, so each slice is whole chars
			self.at += self.text.as_bytes()[start..]
				.iter()
				.position(|byte| matches!(byte, b'"' | b'\\' | 0x00..=0x1f))
				.unwrap_or(self.text.len() - start);
			match self.next() {
				// no escape, so the text is the string
				Some(b'"') if start == first => {
					return SmolStr::from(&self.text[start..self.at - 1]).xok();
				}
				Some(b'"') => {
					out.push_str(&self.text[start..self.at - 1]);
					return SmolStr::from(out).xok();
				}
				Some(b'\\') => {
					out.push_str(&self.text[start..self.at - 1]);
					out.push(self.escape()?);
				}
				Some(_) => {
					return Err(self.error("unescaped control character"));
				}
				None => return Err(self.error("unterminated string")),
			}
		}
	}

	/// The character an escape names, its backslash already read.
	fn escape(&mut self) -> Result<char> {
		match self.next() {
			Some(b'"') => '"',
			Some(b'\\') => '\\',
			Some(b'/') => '/',
			Some(b'b') => '\u{8}',
			Some(b'f') => '\u{c}',
			Some(b'n') => '\n',
			Some(b'r') => '\r',
			Some(b't') => '\t',
			Some(b'u') => {
				let high = self.hex4()?;
				let code = match high {
					// the first of a surrogate pair
					0xd800..=0xdbff => {
						self.expect("\\u")?;
						let low = self.hex4()?;
						match low {
							0xdc00..=0xdfff => {
								0x10000
									+ ((high - 0xd800) << 10) + (low - 0xdc00)
							}
							_ => return Err(self.error("lone surrogate")),
						}
					}
					code => code,
				};
				return char::from_u32(code)
					.ok_or_else(|| self.error("lone surrogate"));
			}
			_ => return Err(self.error("invalid escape")),
		}
		.xok()
	}

	fn hex4(&mut self) -> Result<u32> {
		let mut code = 0;
		for _ in 0..4 {
			let digit = self
				.next()
				.and_then(|byte| (byte as char).to_digit(16))
				.ok_or_else(|| self.error("expected four hex digits"))?;
			code = code * 16 + digit;
		}
		code.xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// Every kind but bytes reads back from its own text as it was written.
	#[crate::test]
	fn round_trips() {
		for value in [
			Value::Null,
			Value::Bool(true),
			Value::Int(i64::MIN),
			Value::Uint(u64::MAX),
			Value::Float(-0.0),
			Value::Float(1e16),
			Value::Float(1e300),
			Value::Float(5e-324),
			Value::Float(f64::MAX),
			Value::str("a\"b\\c\n\t\u{1}/é日😀"),
			value!({ "": [], "nested": { "list": [1, (-2i64), 1.5, null] } }),
		] {
			Value::parse_json(&value.to_json_string())
				.unwrap()
				.xpect_eq(value);
		}
	}

	/// Floats of every magnitude read back bit for bit, which is why this
	/// reader exists.
	#[crate::test]
	fn floats_are_exact() {
		// xorshift, a fixed seed
		let mut bits = 0x9e37_79b9_7f4a_7c15_u64;
		for _ in 0..20_000 {
			bits ^= bits << 13;
			bits ^= bits >> 7;
			bits ^= bits << 17;
			let float = f64::from_bits(bits);
			if float.is_finite() {
				let value = Value::Float(float);
				Value::parse_json(&value.to_json_string())
					.unwrap()
					.xpect_eq(value);
			}
		}
	}

	#[crate::test]
	fn refuses_what_is_not_json() {
		for text in [
			"",
			"{",
			"[1,]",
			"{\"a\":1,}",
			"{a:1}",
			"01",
			"1.",
			".5",
			"1e",
			"-",
			"+1",
			"nul",
			"\"a",
			"\"\n\"",
			"\"\\x\"",
			"\"\\ud800\"",
			"1 2",
			"NaN",
			"1e999",
		] {
			Value::parse_json(text).unwrap_err();
		}
		let deep = "[".repeat(10_000);
		Value::parse_json(&deep)
			.unwrap_err()
			.to_string()
			.xpect_contains("nested too deeply");
		Value::parse_json(r#"{"a": 1, "b": {"a": 1, "a": 2}}"#)
			.unwrap_err()
			.to_string()
			.xpect_contains("column 24: the key `a` is named twice");
		Value::parse_json("{\n  \"a\": x}")
			.unwrap_err()
			.to_string()
			.xpect_contains("line 2 column 8");
	}
}
