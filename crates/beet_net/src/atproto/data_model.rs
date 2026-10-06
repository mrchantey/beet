//! A beet [`Value`] in the protocol's data model, and back.
use base64::Engine;
use beet_core::prelude::*;

/// The boundary between a beet [`Value`] and a record body as a repo stores
/// it, in the protocol's json form.
///
/// The data model is json's with two additions and one omission. Bytes are
/// `{"$bytes": "<base64>"}`, a link is `{"$link": "<cid>"}`, and there are no
/// floats at all, since a float has no canonical encoding to hash. A beet
/// component holds floats freely, so a float crosses the boundary as a
/// self-describing object:
///
/// ```json
/// {"$type": "org.beet.core#f64", "value": "0.5"}
/// ```
///
/// `value` is the shortest decimal that reads back to the same `f64`, so the
/// encoding is exact (NaN, the infinities and `-0` included), deterministic
/// (a record's cid does not move between writes) and legible to a reader
/// that knows nothing of beet, which is why it is a string rather than the
/// float's eight bytes. The `$type` makes it a value any reader can resolve
/// against beet's lexicon, the protocol's own way for a value to say what it
/// is, so it round trips through a store or a stranger's tool that keeps what
/// it does not understand.
///
/// [`encode`](Self::encode) runs on every body a repo writes and
/// [`decode`](Self::decode) on every body read back into a type, so a record
/// type holds `f32`, `f64` and `Vec<u8>` as plain fields.
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// let value = value!({ "scale": 0.5, "name": "cube" });
/// let body = DataModel::encode(value.clone());
/// dag_cbor_ext::record_cid(&body).unwrap();
/// DataModel::decode(body).unwrap().xpect_eq(value);
/// ```
pub struct DataModel;

impl DataModel {
	/// The def a float crosses the boundary as.
	pub const F64: Nsid = Nsid::new_static("org.beet.core#f64");

	/// `value` in the data model: every float as an [`F64`](Self::F64)
	/// object, every byte string as `$bytes`.
	pub fn encode(value: Value) -> Value {
		match value {
			Value::Float(float) => value!({
				"$type": (Self::F64.as_str()),
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

	/// The inverse of [`encode`](Self::encode): every [`F64`](Self::F64)
	/// object a float again, every `$bytes` a byte string. A malformed one is
	/// an error naming it rather than a value that silently stays an object.
	pub fn decode(value: Value) -> Result<Value> {
		match value {
			Value::List(items) => Value::List(
				items.into_iter().map(Self::decode).collect::<Result<_>>()?,
			),
			Value::Map(map) => {
				if let Some(float) = Self::as_f64(&map)? {
					return Value::Float(float).xok();
				}
				if let Some(bytes) = Self::as_bytes(&map)? {
					return Value::Bytes(bytes).xok();
				}
				Value::Map(
					map.into_iter()
						.map(|(key, value)| Ok((key, Self::decode(value)?)))
						.collect::<Result<_>>()?,
				)
			}
			other => other,
		}
		.xok()
	}

	/// The float an [`F64`](Self::F64) object holds, `None` for any other map.
	fn as_f64(map: &Map) -> Result<Option<f64>> {
		if map.len() != 2
			|| map.get("$type").ok().and_then(|ty| ty.as_str().ok())
				!= Some(Self::F64.as_str())
		{
			return Ok(None);
		}
		let Some(text) =
			map.get("value").ok().and_then(|text| text.as_str().ok())
		else {
			bevybail!("an `{}` holds no `value` string", Self::F64);
		};
		text.parse::<f64>().map(Some).map_err(|_| {
			bevyhow!("an `{}` holds `{text}`, not a float", Self::F64)
		})
	}

	/// The bytes a `$bytes` object holds, `None` for any other map.
	fn as_bytes(map: &Map) -> Result<Option<Vec<u8>>> {
		match map.into_iter().collect::<Vec<_>>().as_slice() {
			[(key, Value::Str(base64))] if key.as_str() == "$bytes" => {
				base64::engine::general_purpose::STANDARD_NO_PAD
					.decode(base64.trim_end_matches('='))
					// without `base64/std` the error is not an `Error`
					.map_err(|err| bevyhow!("`$bytes` is not base64: {err}"))?
					.xmap(Some)
					.xok()
			}
			_ => Ok(None),
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Every float survives exactly, the ones a decimal string most often
	/// loses included.
	#[beet_core::test]
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
			let back =
				DataModel::decode(DataModel::encode(Value::Float(float)))
					.unwrap();
			let Value::Float(back) = back else {
				panic!("{float} did not decode to a float");
			};
			back.to_bits().xpect_eq(float.to_bits());
		}
		DataModel::decode(DataModel::encode(Value::Float(f64::NAN)))
			.unwrap()
			.xmap(
				|value| matches!(value, Value::Float(float) if float.is_nan()),
			)
			.xpect_true();
	}

	/// The written form is the documented one, and hashes.
	#[beet_core::test]
	fn writes_the_documented_form() {
		let body = DataModel::encode(
			value!({ "x": 0.5, "b": (Value::Bytes(b"hi".to_vec())) }),
		);
		serde_json::to_string(&body.clone().into_json())
			.unwrap()
			.xpect_contains(
				r#""x":{"$type":"org.beet.core#f64","value":"0.5"}"#,
			)
			.xpect_contains(r#""b":{"$bytes":"aGk"}"#);
		dag_cbor_ext::record_cid(&body).unwrap();
	}

	#[beet_core::test]
	fn refuses_a_malformed_float() {
		DataModel::decode(
			value!({ "$type": "org.beet.core#f64", "value": "half" }),
		)
		.unwrap_err()
		.to_string()
		.xpect_contains("not a float");
	}
}
