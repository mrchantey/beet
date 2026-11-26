//! Beet's JSON reader and writer against `serde_json`, built for these tests
//! with `float_roundtrip`, its exact mode: random documents, the same
//! documents corrupted, and the edges of the grammar each read alike, and
//! every value writes text `serde_json` reads back as it.
use crate::prelude::*;

/// Random JSON text, from a fixed seed.
struct JsonFuzz(u64);

impl JsonFuzz {
	fn new() -> Self { Self(0x9e37_79b9_7f4a_7c15) }

	/// xorshift
	fn next(&mut self) -> u64 {
		self.0 ^= self.0 << 13;
		self.0 ^= self.0 >> 7;
		self.0 ^= self.0 << 17;
		self.0
	}

	fn below(&mut self, len: usize) -> usize {
		(self.next() % len as u64) as usize
	}

	fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
		items[self.below(items.len())]
	}

	fn space(&mut self) -> &'static str {
		self.pick(&["", "", "", " ", "\n\t", "\r\n  "])
	}

	fn digits(&mut self, max: usize) -> String {
		(0..1 + self.below(max))
			.map(|_| char::from(b'0' + self.below(10) as u8))
			.collect()
	}

	fn document(&mut self, depth: usize) -> String {
		let kinds = if depth > 4 { 3 } else { 5 };
		match self.below(kinds) {
			0 => self.pick(&["null", "true", "false"]).to_string(),
			1 => self.number(),
			2 => self.string(),
			3 => {
				let items = (0..self.below(5))
					.map(|_| {
						format!(
							"{}{}{}",
							self.space(),
							self.document(depth + 1),
							self.space()
						)
					})
					.collect::<Vec<_>>();
				format!("[{}]", items.join(","))
			}
			_ => {
				let entries = (0..self.below(5))
					.map(|_| {
						format!(
							"{}{}{}:{}{}",
							self.space(),
							self.string(),
							self.space(),
							self.space(),
							self.document(depth + 1)
						)
					})
					.collect::<Vec<_>>();
				format!("{{{}}}", entries.join(","))
			}
		}
	}

	/// A number in any form the grammar has, or a float's own text.
	fn number(&mut self) -> String {
		match self.below(3) {
			0 => {
				let float = f64::from_bits(self.next());
				let float = if float.is_finite() { float } else { 0.5 };
				match self.below(3) {
					0 => format!("{float:?}"),
					1 => format!("{float:e}"),
					_ => format!("{float}"),
				}
			}
			1 => format!("{}", self.next() as i64),
			_ => {
				let sign = self.pick(&["", "-"]);
				let int = match self.below(3) {
					0 => "0".to_string(),
					_ => format!("{}{}", 1 + self.below(9), self.digits(25)),
				};
				let fraction = match self.below(2) {
					0 => String::new(),
					_ => format!(".{}", self.digits(25)),
				};
				let exponent = match self.below(2) {
					0 => String::new(),
					_ => format!(
						"{}{}{}",
						self.pick(&["e", "E"]),
						self.pick(&["", "+", "-"]),
						self.digits(4)
					),
				};
				format!("{sign}{int}{fraction}{exponent}")
			}
		}
	}

	fn string(&mut self) -> String {
		let text = (0..self.below(8))
			.map(|_| {
				match self.below(5) {
					0 => self
						.pick(&[
							"\\\"", "\\\\", "\\/", "\\b", "\\f", "\\n", "\\r",
							"\\t",
						])
						.to_string(),
					1 => format!("\\u{:04x}", self.below(0xd800)),
					// a surrogate pair
					2 => format!(
						"\\u{:04X}\\u{:04x}",
						0xd800 + self.below(0x400),
						0xdc00 + self.below(0x400)
					),
					3 => self
						.pick(&["é", "日", "😀", "\u{2028}", "\u{7f}"])
						.into(),
					_ => self.pick(&["a", "Z", " ", "0", "$", "'"]).into(),
				}
			})
			.collect::<String>();
		format!("\"{text}\"")
	}

	/// `text` with one character inserted, removed or replaced by one that
	/// means something in JSON.
	fn corrupt(&mut self, text: &str) -> String {
		let mut chars = text.chars().collect::<Vec<_>>();
		let at = self.below(chars.len() + 1);
		let alphabet = "{}[]:,\"\\0123456789.eE+-tnfu \u{0}\u{a0}";
		let char = alphabet
			.chars()
			.nth(self.below(alphabet.chars().count()))
			.unwrap();
		match (self.below(3), at < chars.len()) {
			(0, true) => {
				chars.remove(at);
			}
			(1, true) => chars[at] = char,
			_ => chars.insert(at, char),
		}
		chars.into_iter().collect()
	}
}

/// What serde reads `text` as, exactly.
fn serde_reads(text: &str) -> Option<Value> {
	serde_json::from_str::<serde_json::Value>(text)
		.ok()
		.map(Value::from_json)
}

/// Assert both readers take `text` alike: the same value, or both refusing
/// it. A key named twice is the one known difference, which `serde_json`
/// answers with the last value and beet refuses.
fn agree(text: &str) {
	match (Value::parse_json(text), serde_reads(text)) {
		(Ok(ours), Some(theirs)) if ours == theirs => {}
		(Err(_), None) => {}
		(Err(err), Some(_)) if err.to_string().contains("named twice") => {}
		(ours, theirs) => {
			panic!("{text:?}\n  beet:  {ours:?}\n  serde: {theirs:?}")
		}
	}
}

#[crate::test]
fn reads_random_documents_alike() {
	let mut fuzz = JsonFuzz::new();
	for _ in 0..20_000 {
		let text = fuzz.document(0);
		// valid but for a key named twice or a number past a float's range
		if let Err(err) = Value::parse_json(&text) {
			let err = err.to_string();
			(err.contains("named twice") || err.contains("out of range"))
				.xpect_true();
		}
		agree(&text);
	}
}

#[crate::test]
fn refuses_corrupt_documents_alike() {
	let mut fuzz = JsonFuzz::new();
	for _ in 0..20_000 {
		let mut text = fuzz.document(0);
		for _ in 0..1 + fuzz.below(3) {
			text = fuzz.corrupt(&text);
		}
		agree(&text);
	}
}

/// The edges of the grammar, after the JSONTestSuite's cases.
#[crate::test]
fn reads_the_edges_alike() {
	for text in [
		// numbers
		"0",
		"-0",
		"-0.0",
		"0e1",
		"0E+1",
		"1E22",
		"1e-400",
		"-1e-400",
		"123e65",
		"1e308",
		"1.7976931348623157e308",
		"1.7976931348623159e308",
		"2.2250738585072011e-308",
		"4.9406564584124654e-324",
		"9007199254740993",
		"18446744073709551615",
		"18446744073709551616",
		"-9223372036854775808",
		"-9223372036854775809",
		"123123e-100000",
		"-123123e100000",
		"1e99999999999999999999",
		"0.1000000000000000055511151231257827021181583404541015625",
		"01",
		"-01",
		"2.",
		".2",
		"1.e1",
		"0x1",
		"+1",
		"1e+",
		"- 1",
		"--1",
		"Infinity",
		"-Infinity",
		"NaN",
		"1_000",
		"1 000",
		// strings
		r#""""#,
		r#""\u0000""#,
		r#""𝄞""#,
		r#""\uDD1E""#,
		r#""\uD834""#,
		r#""\uD834x""#,
		r#""\uD834A""#,
		r#""é""#,
		r#""\U0041""#,
		r#""\a""#,
		r#""\'""#,
		"\"\u{7f}\"",
		"\"\t\"",
		"\"\u{1f}\"",
		"'a'",
		r#""a"#,
		// structure
		"[]",
		"{}",
		"[[[]]]",
		"[1,]",
		"[,1]",
		"[1,,2]",
		"{\"a\":1,}",
		"{,}",
		"{\"a\"}",
		"{\"a\":}",
		"{1:1}",
		"{a:1}",
		"[1]]",
		"[1",
		"{\"a\":1}}",
		"[] []",
		"",
		" ",
		"\u{feff}[]",
		"[\u{a0}]",
		"[\u{c}]",
		" [ 1 , 2 ] ",
		"nul",
		"truee",
		"[true false]",
		"{\"a\":1,\"a\":2}",
	] {
		agree(text);
	}
	// the nesting limit, one either side of it
	for depth in [126, 127, 128, 129] {
		agree(&format!("{}{}", "[".repeat(depth), "]".repeat(depth)));
	}
}

/// What beet writes, serde reads back as the value written, and lays out
/// byte for byte as serde does but for a float's text, which serde writes
/// with `ryu` (`1e-5` is its `0.00001`).
#[crate::test]
fn writes_what_serde_writes() {
	let mut fuzz = JsonFuzz::new();
	for _ in 0..20_000 {
		// a key named twice is refused, and so is not written
		let Ok(value) = Value::parse_json(&fuzz.document(0)) else {
			continue;
		};
		for (ours, theirs) in [
			(
				value.to_json_string(),
				serde_json::to_string(&value).unwrap(),
			),
			(
				value.to_json_pretty(),
				serde_json::to_string_pretty(&value).unwrap(),
			),
		] {
			serde_reads(&ours).unwrap().xpect_eq(value.clone());
			Value::parse_json(&theirs).unwrap().xpect_eq(value.clone());
			if !holds_float(&value) {
				ours.xpect_eq(theirs);
			}
		}
	}
}

fn holds_float(value: &Value) -> bool {
	match value {
		Value::Float(_) => true,
		Value::List(items) => items.iter().any(holds_float),
		Value::Map(map) => map.values().any(holds_float),
		_ => false,
	}
}
