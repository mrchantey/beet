//! Reading JSON text as a [`Value`] exactly, the other half of
//! [`write_json`](super::write_json).
//!
//! `serde_json` rounds a float it reads unless built with its
//! `float_roundtrip` feature, so a document whose content is hashed, where one
//! last digit is another hash, is read here instead.

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
	/// Errors on anything that is not one JSON document, naming the line and
	/// column.
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

/// A position in a JSON text.
struct JsonReader<'a> {
	text: &'a str,
	at: usize,
}

impl JsonReader<'_> {
	/// How deep a document may nest, so hostile text cannot exhaust the
	/// stack: `serde_json`'s own limit.
	const MAX_DEPTH: usize = 128;

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
		let before =
			&self.text.as_bytes()[..self.at.min(self.text.as_bytes().len())];
		let line = before.iter().filter(|byte| **byte == b'\n').count() + 1;
		let column = before
			.iter()
			.rev()
			.take_while(|byte| **byte != b'\n')
			.count() + 1;
		bevyhow!("invalid json at line {line} column {column}: {message}")
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

	fn value(&mut self, depth: usize) -> Result<Value> {
		if depth > Self::MAX_DEPTH {
			return Err(self.error("nested too deeply"));
		}
		self.skip_space();
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
					let key = reader.string()?;
					reader.skip_space();
					reader.expect(":")?;
					map.insert(key, reader.value(depth + 1)?);
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

	/// Step past any digits, answering whether there were some.
	fn digits(&mut self) -> bool {
		let start = self.at;
		while matches!(self.peek(), Some(b'0'..=b'9')) {
			self.at += 1;
		}
		self.at > start
	}

	fn number(&mut self) -> Result<Value> {
		let start = self.at;
		if self.peek() == Some(b'-') {
			self.at += 1;
		}
		let leading_zero = self.peek() == Some(b'0');
		let digits_start = self.at;
		if !self.digits() || (leading_zero && self.at - digits_start > 1) {
			return Err(self.error("invalid number"));
		}
		let mut integer = true;
		if self.peek() == Some(b'.') {
			self.at += 1;
			integer = false;
			if !self.digits() {
				return Err(self.error("expected a digit after the point"));
			}
		}
		if matches!(self.peek(), Some(b'e' | b'E')) {
			self.at += 1;
			integer = false;
			if matches!(self.peek(), Some(b'+' | b'-')) {
				self.at += 1;
			}
			if !self.digits() {
				return Err(self.error("expected an exponent"));
			}
		}
		let text = &self.text[start..self.at];
		// `-0` has no integer form: it is the float, as serde reads it
		if integer && text != "-0" {
			if let Ok(uint) = text.parse::<u64>() {
				return Value::Uint(uint).xok();
			}
			if let Ok(int) = text.parse::<i64>() {
				return Value::Int(int).xok();
			}
		}
		match text.parse::<f64>() {
			Ok(float) if float.is_finite() => Value::Float(float).xok(),
			_ => Err(self.error("number out of range")),
		}
	}

	fn string(&mut self) -> Result<SmolStr> {
		self.expect("\"")?;
		let mut out = String::new();
		loop {
			let start = self.at;
			// every byte stopped at is ascii, so each slice is whole chars
			while !matches!(
				self.peek(),
				None | Some(b'"' | b'\\' | 0x00..=0x1f)
			) {
				self.at += 1;
			}
			out.push_str(&self.text[start..self.at]);
			match self.next() {
				Some(b'"') => return SmolStr::from(out).xok(),
				Some(b'\\') => out.push(self.escape()?),
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

	/// Hand written text reads as serde reads it: whitespace, escapes and
	/// each number form.
	#[cfg(feature = "json")]
	#[crate::test]
	fn agrees_with_serde() {
		let text = r#" {
			"escapes": "\" \\ \/ \b\f\n\r\t é 😀",
			"numbers": [0, -0, -1, 1.50, 1e3, 1E-2, 18446744073709551616],
			"empty": [{}, [], ""]
		} "#;
		Value::parse_json(text).unwrap().xpect_eq(
			serde_json::from_str::<serde_json::Value>(text)
				.unwrap()
				.xmap(Value::from_json),
		);
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
		Value::parse_json("{\n  \"a\": x}")
			.unwrap_err()
			.to_string()
			.xpect_contains("line 2 column 8");
	}
}
