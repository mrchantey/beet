use crate::prelude::*;
use beet_core::prelude::*;
use core::ops::Bound;
use serde::Deserialize;
use serde::Serialize;

/// A secondary index a row type declares ([`TableStoreRow::indexes`]): a
/// name, and the ordered fields it sorts rows by, each a dotted path into the
/// row. Rows tied on every field order by key.
///
/// An indexed field holds a scalar: null (or missing), a bool, a number or a
/// string. A list, map or byte string there is refused on write, since no
/// backend orders them alike.
///
/// ```
/// # use beet_net::prelude::*;
/// let by_author = TableIndex::new("by_author", ["author", "created"]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TableIndex {
	/// The index's name within its table, `[a-z][a-z0-9_]*`.
	pub name: SmolStr,
	/// The fields it orders by, outermost first.
	pub fields: Vec<SmolStr>,
}

impl TableIndex {
	/// The index `name` over `fields`.
	pub fn new(
		name: impl Into<SmolStr>,
		fields: impl IntoIterator<Item = impl Into<SmolStr>>,
	) -> Self {
		Self {
			name: name.into(),
			fields: fields.into_iter().map(Into::into).collect(),
		}
	}

	/// The values of this index's fields in `row`, a missing field null.
	pub fn values(&self, row: &Row) -> Result<Vec<Value>> {
		self.fields
			.iter()
			.map(|field| {
				let value = row.field(field).cloned().unwrap_or(Value::Null);
				IndexKey::check_scalar(&value).map_err(|err| {
					bevyhow!("index `{}` field `{field}`: {err}", self.name)
				})?;
				value.xok()
			})
			.collect()
	}

	/// The entry `row` at `key` sorts by in this index: its field values then
	/// its key.
	pub fn entry(&self, key: &TableKey, row: &Row) -> Result<IndexKey> {
		let mut values = self.values(row)?;
		values.push(Value::Str(key.as_str().into()));
		IndexKey::encode(&values)
	}
}

/// The bounds, direction and page of a [`TableProvider::scan_index`]. Each
/// bound is a tuple of leading field values, so a prefix of the fields is a
/// range: `IndexScan::default().eq([author])` reads every row by one author,
/// in its next field's order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexScan {
	/// The lowest tuple, whichever the direction.
	pub start: Bound<Vec<Value>>,
	/// The highest tuple, whichever the direction.
	pub end: Bound<Vec<Value>>,
	/// The most rows a page answers.
	pub limit: Option<usize>,
	/// Highest entry first.
	pub reverse: bool,
	/// Where the previous page stopped.
	pub cursor: Option<ScanCursor>,
}

impl Default for IndexScan {
	fn default() -> Self {
		Self {
			start: Bound::Unbounded,
			end: Bound::Unbounded,
			limit: None,
			reverse: false,
			cursor: None,
		}
	}
}

impl IndexScan {
	/// Entries whose leading fields equal `values`.
	pub fn eq(
		self,
		values: impl IntoIterator<Item = impl Into<Value>>,
	) -> Self {
		let values = values.into_iter().map(Into::into).collect::<Vec<_>>();
		self.from(values.clone()).through(values)
	}
	/// Entries from `values`, inclusive.
	pub fn from(
		mut self,
		values: impl IntoIterator<Item = impl Into<Value>>,
	) -> Self {
		self.start =
			Bound::Included(values.into_iter().map(Into::into).collect());
		self
	}
	/// Entries after every one led by `values`.
	pub fn after(
		mut self,
		values: impl IntoIterator<Item = impl Into<Value>>,
	) -> Self {
		self.start =
			Bound::Excluded(values.into_iter().map(Into::into).collect());
		self
	}
	/// Entries up to and including every one led by `values`.
	pub fn through(
		mut self,
		values: impl IntoIterator<Item = impl Into<Value>>,
	) -> Self {
		self.end =
			Bound::Included(values.into_iter().map(Into::into).collect());
		self
	}
	/// Entries before `values`.
	pub fn before(
		mut self,
		values: impl IntoIterator<Item = impl Into<Value>>,
	) -> Self {
		self.end =
			Bound::Excluded(values.into_iter().map(Into::into).collect());
		self
	}
	/// At most `limit` rows a page.
	pub fn limit(mut self, limit: usize) -> Self {
		self.limit = Some(limit);
		self
	}
	/// Highest entry first.
	pub fn reverse(mut self) -> Self {
		self.reverse = true;
		self
	}
	/// Continue after a previous page.
	pub fn with_cursor(mut self, cursor: Option<ScanCursor>) -> Self {
		self.cursor = cursor;
		self
	}

	/// The bounds as encoded entries, start inclusive and end exclusive,
	/// narrowed past the cursor: an entry `e` is in the scan exactly when
	/// `start <= e < end`.
	pub fn entry_range(&self) -> Result<(Option<IndexKey>, Option<IndexKey>)> {
		let mut start = match &self.start {
			Bound::Included(values) => Some(IndexKey::encode(values)?),
			Bound::Excluded(values) => {
				Some(IndexKey::encode(values)?.successor_of_prefix())
			}
			Bound::Unbounded => None,
		};
		let mut end = match &self.end {
			Bound::Included(values) => {
				Some(IndexKey::encode(values)?.successor_of_prefix())
			}
			Bound::Excluded(values) => Some(IndexKey::encode(values)?),
			Bound::Unbounded => None,
		};
		if let Some(cursor) = &self.cursor {
			let last = IndexKey::from_cursor(cursor)?;
			match self.reverse {
				false => start = Some(last.successor()),
				true => end = Some(last),
			}
		}
		(start, end).xok()
	}

	/// Whether the encoded `entry` lies within [`entry_range`](Self::entry_range).
	pub fn contains(
		range: &(Option<IndexKey>, Option<IndexKey>),
		entry: &IndexKey,
	) -> bool {
		range.0.as_ref().is_none_or(|start| entry >= start)
			&& range.1.as_ref().is_none_or(|end| entry < end)
	}
}

/// An index entry in one order-preserving tuple encoding, so an index orders
/// identically on every backend and a prefix of fields is a byte range.
///
/// The precedent is FoundationDB's tuple layer, its order chosen to match
/// SQLite's ordering of `json_extract` values, which is how the SQLite
/// backend's expression index agrees with it: each element is a type tag then
/// its body, null before every number before every string.
/// - null: `0x01`.
/// - a number, bools as `0` and `1`: `0x02`, then the value as an `f64`
///   order-flipped big-endian (sign bit set for a positive, every bit
///   inverted for a negative, `-0` as `0`), then the integer's exact residual
///   past that `f64` as a sign-flipped big-endian `i64`, so integers past
///   2^53 and floats interleave in numeric order. Floats order by IEEE total
///   order; a row never holds a non-finite one, which its cid refuses.
/// - a string: `0x03`, its utf-8 bytes with each `0x00` written `0x00 0xff`,
///   then `0x00`, so strings order by raw bytes and no element is a prefix of
///   another.
///
/// No tag is `0xff`, so appending `0xff` to an encoded prefix sorts after
/// every entry it leads.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deref)]
pub struct IndexKey(Vec<u8>);

impl IndexKey {
	const NULL: u8 = 0x01;
	const NUMBER: u8 = 0x02;
	const STRING: u8 = 0x03;
	/// The alphabet the text form writes five bits a character in, ascending
	/// byte order being ascending value: a TID's.
	const ALPHABET: &'static [u8; 32] = b"234567abcdefghijklmnopqrstuvwxyz";

	/// Refuse a value no index orders: a list, map or byte string.
	pub fn check_scalar(value: &Value) -> Result {
		match value {
			Value::Null
			| Value::Bool(_)
			| Value::Int(_)
			| Value::Uint(_)
			| Value::Float(_)
			| Value::Str(_) => Ok(()),
			other => bevybail!(
				"an indexed field holds a scalar, found {}",
				other.kind()
			),
		}
	}

	/// The entry for `values`, in order.
	pub fn encode(values: &[Value]) -> Result<Self> {
		let mut bytes = Vec::new();
		for value in values {
			Self::check_scalar(value)?;
			match value {
				Value::Null => bytes.push(Self::NULL),
				Value::Bool(bool) => {
					Self::push_number(&mut bytes, *bool as i128 as f64, 0)
				}
				Value::Int(int) => Self::push_int(&mut bytes, *int as i128),
				Value::Uint(uint) => Self::push_int(&mut bytes, *uint as i128),
				Value::Float(float) => {
					if !float.is_finite() {
						bevybail!(
							"an indexed float must be finite, found {float}"
						);
					}
					Self::push_number(&mut bytes, *float, 0)
				}
				Value::Str(string) => {
					bytes.push(Self::STRING);
					for byte in string.as_bytes() {
						bytes.push(*byte);
						if *byte == 0 {
							bytes.push(0xff);
						}
					}
					bytes.push(0);
				}
				_ => unreachable!("checked scalar"),
			}
		}
		Self(bytes).xok()
	}

	/// The values an entry encodes, numbers as SQLite reads them back: an
	/// integral value an integer, anything else a float.
	pub fn decode(&self) -> Result<Vec<Value>> {
		let mut values = Vec::new();
		let mut bytes = self.0.as_slice();
		while let Some((tag, rest)) = bytes.split_first() {
			bytes = rest;
			match *tag {
				Self::NULL => values.push(Value::Null),
				Self::NUMBER => {
					if bytes.len() < 16 {
						bevybail!("index entry ends inside a number");
					}
					let float = u64::from_be_bytes(bytes[..8].try_into()?);
					let float = match float >> 63 {
						1 => f64::from_bits(float ^ (1 << 63)),
						_ => f64::from_bits(!float),
					};
					let residual =
						(u64::from_be_bytes(bytes[8..16].try_into()?)
							^ (1 << 63)) as i64;
					bytes = &bytes[16..];
					values.push(
						match float.fract() == 0.0 && float.abs() < 9.3e18 {
							true => Value::Int(
								(float as i128 + residual as i128) as i64,
							),
							false => Value::Float(float),
						},
					);
				}
				Self::STRING => {
					let mut text = Vec::new();
					loop {
						match bytes {
							[0, 0xff, rest @ ..] => {
								text.push(0);
								bytes = rest;
							}
							[0, rest @ ..] => {
								bytes = rest;
								break;
							}
							[byte, rest @ ..] => {
								text.push(*byte);
								bytes = rest;
							}
							[] => bevybail!("index entry ends inside a string"),
						}
					}
					values.push(Value::Str(String::from_utf8(text)?.into()));
				}
				other => bevybail!("unknown index entry tag {other:#04x}"),
			}
		}
		values.xok()
	}

	/// The raw bytes.
	pub fn as_bytes(&self) -> &[u8] { &self.0 }

	/// The least entry greater than this one.
	pub fn successor(&self) -> Self {
		let mut bytes = self.0.clone();
		bytes.push(0);
		Self(bytes)
	}

	/// The least entry greater than every entry this one leads.
	pub fn successor_of_prefix(&self) -> Self {
		let mut bytes = self.0.clone();
		bytes.push(0xff);
		Self(bytes)
	}

	/// The text form, five bits a character in an alphabet whose byte order
	/// is its value order, so text compares as the bytes do: what a backend
	/// keying by string (a Dynamo sort key) stores.
	pub fn to_sortable_text(&self) -> String {
		let mut out = String::with_capacity(self.0.len() * 8 / 5 + 1);
		let mut buffer = 0u32;
		let mut bits = 0;
		for byte in &self.0 {
			buffer = (buffer << 8) | *byte as u32;
			bits += 8;
			while bits >= 5 {
				bits -= 5;
				out.push(
					Self::ALPHABET[((buffer >> bits) & 31) as usize] as char,
				);
			}
		}
		if bits > 0 {
			out.push(
				Self::ALPHABET[((buffer << (5 - bits)) & 31) as usize] as char,
			);
		}
		out
	}

	/// An entry from its [`to_sortable_text`](Self::to_sortable_text) form.
	pub fn from_sortable_text(text: &str) -> Result<Self> {
		let mut bytes = Vec::with_capacity(text.len() * 5 / 8);
		let mut buffer = 0u32;
		let mut bits = 0;
		for char in text.bytes() {
			let Some(digit) =
				Self::ALPHABET.iter().position(|item| *item == char)
			else {
				bevybail!("`{text}` is not a sortable index entry");
			};
			buffer = (buffer << 5) | digit as u32;
			bits += 5;
			if bits >= 8 {
				bits -= 8;
				bytes.push((buffer >> bits) as u8);
			}
		}
		Self(bytes).xok()
	}

	/// The cursor a page ending at this entry answers.
	pub fn to_cursor(&self) -> ScanCursor {
		ScanCursor::new(self.to_sortable_text())
	}

	/// The entry a cursor stopped at.
	pub fn from_cursor(cursor: &ScanCursor) -> Result<Self> {
		Self::from_sortable_text(cursor.as_str())
	}

	fn push_int(bytes: &mut Vec<u8>, int: i128) {
		let float = int as f64;
		Self::push_number(bytes, float, (int - float as i128) as i64);
	}

	fn push_number(bytes: &mut Vec<u8>, float: f64, residual: i64) {
		bytes.push(Self::NUMBER);
		// `-0` orders as `0`, as SQLite compares them
		let float = if float == 0.0 { 0.0 } else { float };
		let bits = float.to_bits();
		let ordered = match bits >> 63 {
			0 => bits | (1 << 63),
			_ => !bits,
		};
		bytes.extend_from_slice(&ordered.to_be_bytes());
		bytes.extend_from_slice(&((residual as u64) ^ (1 << 63)).to_be_bytes());
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn encoding_orders_values() {
		let encoded = table_test::ordered_values()
			.iter()
			.map(|value| IndexKey::encode(&[value.clone()]).unwrap())
			.collect::<Vec<_>>();
		for pair in encoded.windows(2) {
			(pair[0] < pair[1]).xpect_true();
		}
		// the text form orders identically
		for pair in encoded.windows(2) {
			(pair[0].to_sortable_text() < pair[1].to_sortable_text())
				.xpect_true();
		}
	}

	#[beet_core::test]
	fn round_trips() {
		for value in table_test::ordered_values() {
			let key =
				IndexKey::encode(&[value.clone(), Value::Str("k".into())])
					.unwrap();
			IndexKey::from_sortable_text(&key.to_sortable_text())
				.unwrap()
				.xpect_eq(key.clone());
			let decoded = key.decode().unwrap();
			IndexKey::encode(&decoded).unwrap().xpect_eq(key);
		}
		// bools are the numbers SQLite reads them as
		IndexKey::encode(&[Value::Bool(true)])
			.unwrap()
			.xpect_eq(IndexKey::encode(&[Value::Int(1)]).unwrap());
		IndexKey::encode(&[Value::Float(-0.0)])
			.unwrap()
			.xpect_eq(IndexKey::encode(&[Value::Int(0)]).unwrap());
	}

	/// A prefix's successor sorts after every entry it leads, and before the
	/// next prefix.
	#[beet_core::test]
	fn prefix_is_a_range() {
		let prefix = IndexKey::encode(&[Value::Str("a".into())]).unwrap();
		let led =
			IndexKey::encode(&[Value::Str("a".into()), Value::Int(i64::MAX)])
				.unwrap();
		let next = IndexKey::encode(&[Value::Str("a\0".into())]).unwrap();
		(prefix < led).xpect_true();
		(led < prefix.successor_of_prefix()).xpect_true();
		(prefix.successor_of_prefix() < next).xpect_true();
	}

	#[beet_core::test]
	fn refuses_non_scalars() {
		IndexKey::encode(&[value!([1])]).unwrap_err();
		IndexKey::encode(&[value!({ "a": 1 })]).unwrap_err();
		IndexKey::encode(&[Value::Bytes(vec![1])]).unwrap_err();
		IndexKey::encode(&[Value::Float(f64::NAN)]).unwrap_err();
	}
}
