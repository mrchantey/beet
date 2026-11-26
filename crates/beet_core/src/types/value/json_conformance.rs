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
	serde_json::from_str::<Value>(text).ok()
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
/// with `ryu` (`1e-5` is its `0.00001`). Serde writes a [`Map`] sorted, its
/// generic form, so the bytes compare against beet writing the value serde
/// sees.
#[crate::test]
fn writes_what_serde_writes() {
	let mut fuzz = JsonFuzz::new();
	for _ in 0..20_000 {
		// a key named twice is refused, and so is not written
		let Ok(value) = Value::parse_json(&fuzz.document(0)) else {
			continue;
		};
		for (ours, theirs, sorted) in [
			(
				value.to_json_string(),
				serde_json::to_string(&value).unwrap(),
				json_ext::to_string(&value).unwrap(),
			),
			(
				value.to_json_pretty(),
				serde_json::to_string_pretty(&value).unwrap(),
				json_ext::to_string_pretty(&value).unwrap(),
			),
		] {
			serde_reads(&ours).unwrap().xpect_eq(value.clone());
			Value::parse_json(&theirs).unwrap().xpect_eq(value.clone());
			if !holds_float(&value) {
				sorted.xpect_eq(theirs);
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

/// Every serde shape a derive writes reads and writes as `serde_json` reads
/// and writes it, through [`json_ext`]: the value `serde_json` writes, and
/// the text `serde_json` writes read back as the value.
#[crate::test]
fn serde_shapes_cross_alike() {
	use serde::Deserialize;
	use serde::Serialize;
	use serde::de::DeserializeOwned;

	fn cross<T: core::fmt::Debug + PartialEq + Serialize + DeserializeOwned>(
		value: T,
	) {
		let ours = json_ext::to_string(&value).unwrap();
		let theirs = serde_json::to_string(&value).unwrap();
		serde_reads(&ours).xpect_eq(serde_reads(&theirs));
		for text in [theirs, ours] {
			let read = json_ext::from_str::<T>(&text).unwrap();
			if read != value {
				panic!("{text}\n  read:    {read:?}\n  written: {value:?}");
			}
		}
	}

	#[derive(Debug, PartialEq, Serialize, Deserialize)]
	struct Unit;
	#[derive(Debug, PartialEq, Serialize, Deserialize)]
	struct Newtype(u32);
	#[derive(Debug, PartialEq, Serialize, Deserialize)]
	struct Tuple(i8, String);
	#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
	#[serde(rename_all = "camelCase")]
	struct Fields {
		first_name: String,
		#[serde(skip_serializing_if = "Option::is_none")]
		middle: Option<String>,
		#[serde(default)]
		tags: Vec<SmolStr>,
		scale: f32,
		ratio: f64,
		letter: char,
		bytes: Vec<u8>,
		by_id: BTreeMap<u32, bool>,
		by_flag: BTreeMap<bool, i64>,
		nested: Option<Box<Fields>>,
		raw: Value,
	}
	#[derive(Debug, PartialEq, Serialize, Deserialize)]
	enum External {
		Unit,
		Newtype(u64),
		Tuple(u8, u8),
		Struct { at: i64 },
	}
	#[derive(Debug, PartialEq, Serialize, Deserialize)]
	#[serde(tag = "type")]
	enum Internal {
		Unit,
		Struct { at: i64, label: String },
		Newtype(Newtype2),
	}
	#[derive(Debug, PartialEq, Serialize, Deserialize)]
	struct Newtype2 {
		inner: f64,
	}
	#[derive(Debug, PartialEq, Serialize, Deserialize)]
	#[serde(tag = "t", content = "c")]
	enum Adjacent {
		Unit,
		Newtype(i32),
		Struct { at: u8 },
	}
	#[derive(Debug, PartialEq, Serialize, Deserialize)]
	#[serde(untagged)]
	enum Untagged {
		Uint(u64),
		Int(i64),
		Float(f64),
		Text(String),
		List(Vec<Untagged>),
		Map { key: Box<Untagged> },
	}
	#[derive(Debug, PartialEq, Serialize, Deserialize)]
	struct Flattened {
		id: u32,
		#[serde(flatten)]
		rest: BTreeMap<String, Value>,
	}
	#[derive(Debug, PartialEq, Serialize, Deserialize)]
	struct FlattenedTag {
		id: u32,
		#[serde(flatten)]
		tagged: Internal,
	}

	cross(Unit);
	cross(Newtype(7));
	cross(Tuple(-3, "a\"b".into()));
	cross((1u8, -1i16, 0.5f64, "x".to_string(), None::<u8>, Some(2u8)));
	let fields = Fields {
		first_name: "é\n".into(),
		middle: None,
		tags: vec!["a".into()],
		scale: 1.3,
		ratio: 0.1,
		letter: '😀',
		bytes: vec![0, 255],
		by_id: [(1, true), (20, false)].into(),
		by_flag: [(true, -1)].into(),
		nested: None,
		raw: value!({ "list": [1, -1, 1.5, null, "s"] }),
	};
	cross(Fields {
		middle: Some("m".into()),
		nested: Some(Box::new(fields.clone())),
		..fields
	});
	for value in [
		External::Unit,
		External::Newtype(u64::MAX),
		External::Tuple(1, 2),
		External::Struct { at: i64::MIN },
	] {
		cross(value);
	}
	for value in [
		Internal::Unit,
		Internal::Struct {
			at: -1,
			label: "l".into(),
		},
		Internal::Newtype(Newtype2 { inner: 2.5 }),
	] {
		cross(value);
	}
	for value in [Adjacent::Unit, Adjacent::Newtype(-9), Adjacent::Struct {
		at: 3,
	}] {
		cross(value);
	}
	cross(Untagged::List(vec![
		Untagged::Uint(1),
		Untagged::Int(-1),
		Untagged::Float(1.5),
		Untagged::Text("t".into()),
		Untagged::Map {
			key: Box::new(Untagged::Uint(2)),
		},
	]));
	cross(Flattened {
		id: 1,
		rest: [("extra".to_string(), value!([1, "two"]))].into(),
	});
	cross(FlattenedTag {
		id: 1,
		tagged: Internal::Struct {
			at: 4,
			label: "x".into(),
		},
	});
}
