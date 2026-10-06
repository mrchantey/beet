use crate::prelude::*;
use base64::Engine;

/// A [`Value`] in the protocol's data model: json's shape with two additions
/// and one omission, sealed so it can only hold what a repo accepts.
///
/// Bytes are `{"$bytes": "<base64>"}`, a link is `{"$link": "<cid>"}`, and
/// there are no floats at all, since a float has no canonical encoding to
/// hash. A beet component holds floats freely, so a float crosses into the
/// data model as a self-describing object:
///
/// ```json
/// {"$type": "org.beet.core#float", "value": "0.5"}
/// ```
///
/// `value` is the shortest decimal that reads back to the same `f64`, so the
/// encoding is exact (NaN, the infinities and `-0` included), deterministic (a
/// record's cid does not move between writes) and legible to a reader that
/// knows nothing of beet, which is why it is a string rather than the float's
/// eight bytes. The `$type` makes it a value any reader resolves against
/// beet's lexicon, the protocol's own way for a value to say what it is, so it
/// survives a store or a stranger's tool that keeps what it does not know.
///
/// Two ways in, one way out:
/// - [`From<Value>`]: a beet value encoded, every float an
///   [`FLOAT`](Self::FLOAT) object and every byte string `$bytes`;
/// - [`from_wire`](Self::from_wire): a value read off the network, already in
///   the data model, refused when it is not;
/// - [`decode`](Self::decode) (and [`From<AtprotoValue>`] for [`Value`]): back
///   to a beet value, infallible because construction refused anything it
///   could not decode.
///
/// [`Deref`] reads the data model form itself, what a repo stores and a cid
/// hashes; there is no mutable access, which is what keeps it valid.
///
/// ```
/// # use beet_core::prelude::*;
/// let value = value!({ "scale": 0.5, "name": "cube" });
/// let record = AtprotoValue::from(value.clone());
/// record
/// 	.as_map()
/// 	.unwrap()
/// 	.get("scale")
/// 	.unwrap()
/// 	.as_map()
/// 	.unwrap()
/// 	.contains("$type")
/// 	.xpect_true();
/// record.decode().xpect_eq(value);
/// ```
#[derive(Debug, Default, Clone, PartialEq, Deref)]
pub struct AtprotoValue(Value);

impl AtprotoValue {
	/// The def a float crosses into the data model as.
	pub const FLOAT: Nsid = Nsid::new_static("org.beet.core#float");

	/// A value read off the network, already in the data model. A float or a
	/// raw byte string anywhere is refused, since no wire form carries either,
	/// as is a [`FLOAT`](Self::FLOAT) object that does not hold a float.
	pub fn from_wire(value: Value) -> Result<Self> {
		Self::validate(&value)?;
		Self(value).xok()
	}

	/// A serializable type in the data model.
	#[cfg(feature = "serde")]
	pub fn from_serde<T: Serialize>(value: &T) -> Result<Self> {
		Value::from_serde(value)?.xmap(Self::from).xok()
	}

	/// The data model form read as a `T`, its floats and bytes decoded first.
	#[cfg(feature = "serde")]
	pub fn into_serde<T: DeserializeOwned>(self) -> Result<T> {
		self.decode().into_serde()
	}

	/// A value read off the network as json.
	#[cfg(feature = "json")]
	pub fn from_json(json: serde_json::Value) -> Result<Self> {
		Self::from_wire(Value::from_json(json))
	}

	/// The data model form as json, exactly what a repo stores.
	#[cfg(feature = "json")]
	pub fn to_json(&self) -> serde_json::Value { self.0.clone().into_json() }

	/// Back to a beet value: every [`FLOAT`](Self::FLOAT) object a float and
	/// every `$bytes` a byte string again.
	pub fn decode(self) -> Value { Self::decode_value(self.0) }

	/// This value as the body of a record in `collection`: an object, its
	/// `$type` written as the collection when absent. A body that is not an
	/// object, or names another collection, is refused.
	pub fn into_record(self, collection: &Nsid) -> Result<Self> {
		let Value::Map(mut map) = self.0 else {
			bevybail!(
				"a `{collection}` record must be an object, found {}",
				self.0.kind()
			);
		};
		match map.get("$type").ok() {
			None => {
				map.insert("$type", collection.as_str());
			}
			Some(Value::Str(r#type))
				if r#type.as_str() == collection.as_str() => {}
			Some(other) => bevybail!(
				"a record written to `{collection}` declares `$type: {other}`"
			),
		}
		Self(Value::Map(map)).xok()
	}

	fn encode(value: Value) -> Value {
		match value {
			Value::Float(float) => value!({
				"$type": (Self::FLOAT.as_str()),
				"value": (float.to_string())
			}),
			Value::Bytes(bytes) => value!({
				"$bytes": (base64::engine::general_purpose::STANDARD_NO_PAD
					.encode(bytes))
			}),
			Value::List(items) => {
				Value::List(items.into_iter().map(Self::encode).collect())
			}
			Value::Map(map) => Value::Map(
				map.into_iter()
					.map(|(key, value)| (key, Self::encode(value)))
					.collect(),
			),
			other => other,
		}
	}

	fn decode_value(value: Value) -> Value {
		match value {
			Value::List(items) => {
				Value::List(items.into_iter().map(Self::decode_value).collect())
			}
			Value::Map(map) => {
				if let Some(float) = Self::as_float(&map) {
					return Value::Float(float);
				}
				if let Some(bytes) = Self::as_bytes(&map) {
					return Value::Bytes(bytes);
				}
				Value::Map(
					map.into_iter()
						.map(|(key, value)| (key, Self::decode_value(value)))
						.collect(),
				)
			}
			other => other,
		}
	}

	/// Refuse anything [`decode`](Self::decode) could not read back, naming
	/// what was found.
	fn validate(value: &Value) -> Result {
		match value {
			Value::Float(float) => bevybail!(
				"`{float}` is not in the atproto data model, which has no \
				 floats: a float is written as an `{}` object",
				Self::FLOAT
			),
			Value::Bytes(_) => bevybail!(
				"raw bytes are not in the atproto data model: bytes are \
				 written `{{\"$bytes\": \"<base64>\"}}`"
			),
			Value::List(items) => {
				for item in items {
					Self::validate(item)?;
				}
			}
			Value::Map(map) => {
				let is_float =
					map.get("$type").ok().and_then(|ty| ty.as_str().ok())
						== Some(Self::FLOAT.as_str());
				if is_float && Self::as_float(map).is_none() {
					bevybail!(
						"an `{}` must hold one `value` string reading as a float, \
						 found {map}",
						Self::FLOAT
					);
				}
				let is_bytes = map.contains("$bytes") && map.len() == 1;
				if is_bytes && Self::as_bytes(map).is_none() {
					bevybail!("`$bytes` must be base64, found {map}");
				}
				for (_, child) in map {
					Self::validate(child)?;
				}
			}
			_ => {}
		}
		Ok(())
	}

	/// The float a [`FLOAT`](Self::FLOAT) object holds, `None` for any other
	/// map.
	fn as_float(map: &Map) -> Option<f64> {
		(map.len() == 2
			&& map.get("$type").ok().and_then(|ty| ty.as_str().ok())
				== Some(Self::FLOAT.as_str()))
		.then(|| map.get("value").ok()?.as_str().ok()?.parse::<f64>().ok())
		.flatten()
	}

	/// The bytes a `$bytes` object holds, `None` for any other map.
	fn as_bytes(map: &Map) -> Option<Vec<u8>> {
		match map.into_iter().collect::<Vec<_>>().as_slice() {
			[(key, Value::Str(base64))] if key.as_str() == "$bytes" => {
				base64::engine::general_purpose::STANDARD_NO_PAD
					.decode(base64.trim_end_matches('='))
					.ok()
			}
			_ => None,
		}
	}
}

impl From<Value> for AtprotoValue {
	fn from(value: Value) -> Self { Self(Self::encode(value)) }
}

impl From<AtprotoValue> for Value {
	fn from(value: AtprotoValue) -> Self { value.decode() }
}

/// Serialized as the data model form itself.
#[cfg(feature = "serde")]
impl Serialize for AtprotoValue {
	fn serialize<S: serde::Serializer>(
		&self,
		serializer: S,
	) -> Result<S::Ok, S::Error> {
		self.0.serialize(serializer)
	}
}

/// Deserialized as a value off the wire, through
/// [`from_wire`](AtprotoValue::from_wire).
#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for AtprotoValue {
	fn deserialize<D: serde::Deserializer<'de>>(
		deserializer: D,
	) -> Result<Self, D::Error> {
		Self::from_wire(Value::deserialize(deserializer)?)
			.map_err(serde::de::Error::custom)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// Every float survives exactly, the ones a decimal string most often
	/// loses included.
	#[crate::test]
	fn floats_round_trip_exactly() {
		for float in [
			0.1,
			-0.0,
			1e-300,
			f64::MAX,
			f64::MIN_POSITIVE,
			f64::INFINITY,
			f64::NEG_INFINITY,
		] {
			let Value::Float(back) =
				AtprotoValue::from(Value::Float(float)).decode()
			else {
				panic!("{float} did not decode to a float");
			};
			back.to_bits().xpect_eq(float.to_bits());
		}
		AtprotoValue::from(Value::Float(f64::NAN))
			.decode()
			.xmap(
				|value| matches!(value, Value::Float(float) if float.is_nan()),
			)
			.xpect_true();
	}

	/// The written form is the documented one, and reads back off the wire.
	#[cfg(feature = "json")]
	#[crate::test]
	fn writes_the_documented_form() {
		let value = value!({ "x": 0.5, "b": (Value::Bytes(b"hi".to_vec())) });
		let record = AtprotoValue::from(value.clone());
		let json = record.to_json();
		serde_json::to_string(&json)
			.unwrap()
			.xpect_contains(
				r#""x":{"$type":"org.beet.core#float","value":"0.5"}"#,
			)
			.xpect_contains(r#""b":{"$bytes":"aGk"}"#);
		AtprotoValue::from_json(json)
			.unwrap()
			.decode()
			.xpect_eq(value);
	}

	/// What no repo accepts is refused off the wire, naming it.
	#[crate::test]
	fn refuses_what_the_wire_cannot_carry() {
		AtprotoValue::from_wire(value!({ "x": 0.5 }))
			.unwrap_err()
			.to_string()
			.xpect_contains("no floats");
		AtprotoValue::from_wire(value!({
			"x": { "$type": "org.beet.core#float", "value": "half" }
		}))
		.unwrap_err()
		.to_string()
		.xpect_contains("reading as a float");
		AtprotoValue::from_wire(value!({ "b": { "$bytes": "!!" } }))
			.unwrap_err()
			.to_string()
			.xpect_contains("base64");
	}

	#[crate::test]
	fn types_a_record_body() {
		let collection = Nsid::new_static("com.example.note");
		AtprotoValue::from(value!({ "text": "hi" }))
			.into_record(&collection)
			.unwrap()
			.as_map()
			.unwrap()
			.get("$type")
			.unwrap()
			.clone()
			.xpect_eq(Value::from("com.example.note"));
		AtprotoValue::from(value!({ "$type": "com.example.other" }))
			.into_record(&collection)
			.unwrap_err()
			.to_string()
			.xpect_contains("declares `$type");
	}
}
