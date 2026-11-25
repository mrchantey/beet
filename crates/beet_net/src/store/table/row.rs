use base64::Engine;
use beet_core::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// One row of a table: a map of named fields, self-describing and never
/// positional, so a backend stores it whole and indexes it by field.
///
/// A row crosses every encoding as its json form, where a byte string is
/// `{"$bytes": "<base64>"}` exactly as in a record, so bytes survive a
/// backend that stores text and the row's [`cid`](Self::cid) is unchanged by
/// the trip.
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// let row = Row::from_serde(&value!({ "name": "cube", "scale": 0.5 })).unwrap();
/// row.field("name").unwrap().xpect_eq(Value::from("cube"));
/// row.cid().unwrap();
/// ```
#[derive(Debug, Default, Clone, PartialEq, Deref, DerefMut)]
pub struct Row(Map);

impl Row {
	/// The row of these fields.
	pub fn new(map: Map) -> Self { Self(map) }

	/// The row a value is, refused when it is not a map.
	pub fn from_value(value: Value) -> Result<Self> {
		match value {
			Value::Map(map) => Self(map).xok(),
			other => {
				bevybail!("a row is a map of fields, found {}", other.kind())
			}
		}
	}

	/// The row a serializable value is, refused when it is not a map.
	pub fn from_serde<T: Serialize>(value: &T) -> Result<Self> {
		Self::from_value(Value::from_serde(value)?)
	}

	/// The row read as a `T`.
	pub fn into_serde<T: DeserializeOwned>(self) -> Result<T> {
		Value::Map(self.0).into_serde()
	}

	/// The fields as a map.
	pub fn into_map(self) -> Map { self.0 }

	/// The row as a [`Value::Map`].
	pub fn into_value(self) -> Value { Value::Map(self.0) }

	/// The value at a dotted field path, ie `author.name`, `None` where any
	/// step is missing or not a map.
	pub fn field(&self, path: &str) -> Option<&Value> {
		let mut segments = path.split('.');
		let mut value = self.0.get(segments.next()?).ok()?;
		for segment in segments {
			value = value.as_map().ok()?.get(segment).ok()?;
		}
		Some(value)
	}

	/// The row's content id: CIDv1 over its canonical DAG-CBOR with floats
	/// permitted ([`Value::to_dag_cbor`]), so a float-free row carrying `$type`
	/// has exactly its record's cid. Refused for a non-finite float.
	pub fn cid(&self) -> Result<Cid> {
		Value::Map(self.0.clone()).dag_cbor_cid()
	}

	/// The json form as a value: bytes as `$bytes` objects, everything else
	/// as itself, which for a record is its data model form.
	pub fn into_json_form(self) -> Value {
		Self::encode_bytes(Value::Map(self.0))
	}

	/// The json form, see [`into_json_form`](Self::into_json_form).
	pub fn to_json(&self) -> serde_json::Value {
		self.clone().into_json_form().into_json()
	}

	/// A row from its json form, every `$bytes` object among its fields a
	/// byte string again.
	pub fn from_json(json: serde_json::Value) -> Result<Self> {
		Self::from_json_form(Value::from_json(json))
	}

	/// A row from its json form as a value, see [`from_json`](Self::from_json).
	pub fn from_json_form(value: Value) -> Result<Self> {
		// each field, never the row: a row of one `$bytes` field is a row
		Self::from_value(value)?
			.0
			.into_iter()
			.map(|(key, value)| (key, Self::decode_bytes(value)))
			.collect::<Map>()
			.xmap(Self)
			.xok()
	}

	/// The row its [`to_json`](Self::to_json) text holds, every float exactly
	/// as it was written ([`Value::parse_json`]), so its [`cid`](Self::cid)
	/// survives the trip.
	pub fn parse_json(text: &str) -> Result<Self> {
		Self::from_json_form(Value::parse_json(text)?)
	}

	fn encode_bytes(value: Value) -> Value {
		match value {
			Value::Bytes(bytes) => value!({
				"$bytes": (base64::engine::general_purpose::STANDARD_NO_PAD
					.encode(bytes))
			}),
			Value::List(items) => {
				Value::List(items.into_iter().map(Self::encode_bytes).collect())
			}
			Value::Map(map) => Value::Map(
				map.into_iter()
					.map(|(key, value)| (key, Self::encode_bytes(value)))
					.collect(),
			),
			other => other,
		}
	}

	fn decode_bytes(value: Value) -> Value {
		match value {
			Value::List(items) => {
				Value::List(items.into_iter().map(Self::decode_bytes).collect())
			}
			Value::Map(map) => {
				let bytes =
					match (&map).into_iter().collect::<Vec<_>>().as_slice() {
						[(key, Value::Str(base64))]
							if key.as_str() == "$bytes" =>
						{
							base64::engine::general_purpose::STANDARD_NO_PAD
								.decode(base64.trim_end_matches('='))
								.ok()
						}
						_ => None,
					};
				match bytes {
					Some(bytes) => Value::Bytes(bytes),
					None => Value::Map(
						map.into_iter()
							.map(|(key, value)| {
								(key, Self::decode_bytes(value))
							})
							.collect(),
					),
				}
			}
			other => other,
		}
	}
}

impl From<Map> for Row {
	fn from(map: Map) -> Self { Self(map) }
}

/// Serialized as its json form, so every format carries bytes the same way.
impl Serialize for Row {
	fn serialize<S: serde::Serializer>(
		&self,
		serializer: S,
	) -> Result<S::Ok, S::Error> {
		Self::encode_bytes(Value::Map(self.0.clone())).serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for Row {
	fn deserialize<D: serde::Deserializer<'de>>(
		deserializer: D,
	) -> Result<Self, D::Error> {
		Self::from_json_form(Value::deserialize(deserializer)?)
			.map_err(serde::de::Error::custom)
	}
}

/// A row with its key and cid, the one shape a [`TableProvider`] reads or
/// writes: the cid is computed once, when the row is built
/// ([`new`](Self::new)), and carried with it from then on, so moving a row
/// between backends never recomputes it.
#[derive(Debug, Clone, PartialEq, Get, Serialize, Deserialize)]
pub struct StoredRow {
	/// The row's primary key within its table.
	key: TableKey,
	/// The row's content id, [`Row::cid`] when it was built.
	cid: Cid,
	/// The fields.
	row: Row,
}

impl StoredRow {
	/// A row at `key`, its cid computed here.
	pub fn new(key: impl Into<TableKey>, row: Row) -> Result<Self> {
		let key = key.into();
		key.validate()?;
		Self {
			cid: row.cid()?,
			key,
			row,
		}
		.xok()
	}

	/// A row read back from storage, its cid as stored and never
	/// recomputed.
	pub fn from_parts(key: TableKey, cid: Cid, row: Row) -> Self {
		Self { key, cid, row }
	}

	/// The key, cid and row.
	pub fn into_parts(self) -> (TableKey, Cid, Row) {
		(self.key, self.cid, self.row)
	}

	/// The fields alone.
	pub fn into_row(self) -> Row { self.row }
}

/// A row's primary key within its table: a non-empty string, ordered by its
/// raw utf-8 bytes, which is the one order every backend scans in. A
/// [`Tid`] is the default key, written in an alphabet whose byte order is its
/// time order, so a table keyed by TID scans in creation order.
#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	Hash,
	PartialOrd,
	Ord,
	Serialize,
	Deserialize,
	Reflect,
)]
#[serde(transparent)]
pub struct TableKey(SmolStr);

impl TableKey {
	/// Refuse a key no backend can address: empty, or holding a control
	/// character.
	pub fn validate(&self) -> Result {
		if self.0.is_empty() {
			bevybail!("a table key must not be empty");
		}
		if let Some(char) = self.0.chars().find(|char| char.is_control()) {
			bevybail!(
				"table key `{}` holds the control character {char:?}",
				self.0
			);
		}
		Ok(())
	}

	/// The key as text.
	pub fn as_str(&self) -> &str { &self.0 }
}

impl core::ops::Deref for TableKey {
	type Target = str;
	fn deref(&self) -> &str { &self.0 }
}

impl core::fmt::Display for TableKey {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		self.0.fmt(f)
	}
}

impl From<&str> for TableKey {
	fn from(key: &str) -> Self { Self(key.into()) }
}
impl From<String> for TableKey {
	fn from(key: String) -> Self { Self(key.into()) }
}
impl From<SmolStr> for TableKey {
	fn from(key: SmolStr) -> Self { Self(key) }
}
impl From<&TableKey> for TableKey {
	fn from(key: &TableKey) -> Self { key.clone() }
}
/// The 13 character written form, whose byte order is its time order.
impl From<Tid> for TableKey {
	fn from(tid: Tid) -> Self { Self(tid.to_string().into()) }
}
impl From<Rkey> for TableKey {
	fn from(rkey: Rkey) -> Self { Self(rkey.into()) }
}
impl From<&Rkey> for TableKey {
	fn from(rkey: &Rkey) -> Self { Self(rkey.as_str().into()) }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Bytes survive the json form, and the cid is the same before and
	/// after.
	#[beet_core::test]
	fn json_form_round_trips_bytes() {
		let row = Row::from_value(value!({
			"b": (Value::Bytes(vec![0, 159, 255])),
			"x": 0.5,
			"nested": [{ "c": (Value::Bytes(b"hi".to_vec())) }]
		}))
		.unwrap();
		let back = Row::from_json(row.to_json()).unwrap();
		back.clone().xpect_eq(row.clone());
		back.cid().unwrap().xpect_eq(row.cid().unwrap());
	}

	/// A float-free row with `$type` hashes to its record's cid.
	#[beet_core::test]
	fn record_row_has_the_record_cid() {
		let value = value!({ "$type": "com.example.note", "text": "hi" });
		Row::from_value(value.clone())
			.unwrap()
			.cid()
			.unwrap()
			.xpect_eq(AtprotoValue::from_wire(value).unwrap().cid());
	}

	/// A row is itself after its json text, whatever its fields: every kind of
	/// value, floats of every magnitude, and maps that read as bytes.
	#[beet_core::test]
	fn json_text_keeps_the_cid() {
		// xorshift, a fixed seed
		let mut bits = 0x9e37_79b9_7f4a_7c15_u64;
		let floats = (0..2_000)
			.map(|_| {
				bits ^= bits << 13;
				bits ^= bits >> 7;
				bits ^= bits << 17;
				f64::from_bits(bits)
			})
			.filter(|float| float.is_finite())
			.map(Value::Float);
		for value in [
			Value::Null,
			Value::Bool(false),
			Value::Int(i64::MIN),
			Value::Int(3),
			Value::Uint(3),
			Value::Uint(u64::MAX),
			Value::Float(0.0),
			Value::Float(-0.0),
			Value::Float(1.0),
			Value::Float(1e16),
			Value::Float(f64::MAX),
			Value::Float(5e-324),
			Value::Bytes(Vec::new()),
			Value::Bytes(vec![0, 159, 255]),
			Value::str(""),
			Value::str("\u{0}\n\"\\/é日😀\u{2028}"),
			value!([]),
			value!({}),
			value!([null, [1, { "a": (Value::Bytes(vec![1])) }]]),
			value!({ "$bytes": "aGk" }),
			value!({ "$bytes": "aGk=" }),
			value!({ "$bytes": "not base64!" }),
			value!({ "$bytes": 3 }),
			value!({ "$bytes": "aGk", "more": 1 }),
		]
		.into_iter()
		.chain(floats)
		{
			for field in ["field", "$bytes"] {
				let row = Row::new(Map::new([(field, value.clone())]));
				// a value no row holds is refused before it is written
				let Ok(cid) = row.cid() else { continue };
				let text = serde_json::to_string_pretty(&row).unwrap();
				Row::parse_json(&text).unwrap().cid().unwrap().xpect_eq(cid);
			}
		}
	}

	/// A Tid's key orders by time, as raw bytes.
	#[beet_core::test]
	fn tid_keys_order_by_time() {
		let early = TableKey::from(Tid::from_parts(1_000, 0));
		let late = TableKey::from(Tid::from_parts(2_000_000_000, 0));
		(early < late).xpect_true();
		(early.as_bytes() < late.as_bytes()).xpect_true();
	}

	#[beet_core::test]
	fn refuses_bad_keys() {
		StoredRow::new("", Row::default()).unwrap_err();
		StoredRow::new("a\nb", Row::default()).unwrap_err();
		Row::from_value(Value::Int(3)).unwrap_err();
	}
}
