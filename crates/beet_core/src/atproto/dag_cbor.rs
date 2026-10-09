//! An [`AtprotoValue`] in canonical DAG-CBOR, the encoding a repo stores a
//! record as and computes its cid over.
//!
//! The json form and DAG-CBOR differ in exactly two places: a link is
//! `{"$link": "<cid>"}` in json and CBOR tag 42 in DAG-CBOR, and bytes are
//! `{"$bytes": "<base64>"}` in json and a CBOR byte string in DAG-CBOR. So the
//! encoding crosses through [`Ipld`], the data model's own tree, converting
//! both, and `serde_ipld_dagcbor` writes it canonically (map keys sorted length
//! first, no indefinite lengths). [`MediaType::DagCbor`] is the one way in and
//! out, and its output is exactly the block a PDS stores, so a record written
//! to a bucket and one written to a real PDS answer the same [`cid`].
//!
//! ```
//! # use beet_core::prelude::*;
//! let record = AtprotoValue::try_from(
//! 	value!({ "$type": "app.bsky.feed.post", "text": "hi" }),
//! )
//! .unwrap();
//! let block = MediaType::DagCbor.serialize(&record).unwrap();
//! record.cid().xpect_eq(Cid::new(Cid::DAG_CBOR, &block));
//! MediaType::DagCbor
//! 	.deserialize::<AtprotoValue>(&block)
//! 	.unwrap()
//! 	.xpect_eq(record);
//! ```
//!
//! [`cid`]: AtprotoValue::cid
use crate::prelude::*;
use base64::Engine;
use ipld_core::ipld::Ipld;

impl AtprotoValue {
	/// The cid of this value as a record: CIDv1 over its canonical DAG-CBOR,
	/// exactly as a PDS computes it.
	pub fn cid(&self) -> Cid { Cid::new(Cid::DAG_CBOR, &self.to_dag_cbor()) }

	/// This value in canonical DAG-CBOR, what [`MediaType::DagCbor`] writes.
	/// Infallible, since construction refused anything the data model cannot
	/// encode.
	pub(crate) fn to_dag_cbor(&self) -> Vec<u8> {
		serde_ipld_dagcbor::to_vec(&Self::to_ipld(self)).expect(
			"an `Ipld` encodes into a `Vec`, short of allocation failing",
		)
	}

	/// A value read from DAG-CBOR, refused when it is not in the data model,
	/// ie it holds a float.
	pub(crate) fn from_dag_cbor(bytes: &[u8]) -> Result<Self> {
		serde_ipld_dagcbor::from_slice::<Ipld>(bytes)
			.map_err(|err| bevyhow!("Failed to deserialize DAG-CBOR: {err}"))?
			.xmap(Self::from_ipld)?
			.xmap(Self::from_wire)
	}

	/// The data model's view of a json form value: `$link` and `$bytes`
	/// objects become the link and the bytes they stand for. Total over what
	/// an [`AtprotoValue`] can hold, so each branch it rules out says why.
	fn to_ipld(value: &Value) -> Ipld {
		match value {
			Value::Null => Ipld::Null,
			Value::Bool(bool) => Ipld::Bool(*bool),
			Value::Int(int) => Ipld::Integer(*int as i128),
			Value::Uint(uint) => Ipld::Integer(*uint as i128),
			Value::Float(_) | Value::Bytes(_) => unreachable!(
				"an `AtprotoValue` encodes every float and byte string as an object"
			),
			Value::Str(string) => Ipld::String(string.to_string()),
			Value::List(items) => {
				Ipld::List(items.iter().map(Self::to_ipld).collect())
			}
			Value::Map(map) => {
				match map.into_iter().collect::<Vec<_>>().as_slice() {
					[(key, Value::Str(cid))] if key.as_str() == "$link" => {
						Ipld::Link(
							Cid::parse(cid)
								.and_then(|cid| cid.to_binary())
								.ok()
								.and_then(|binary| {
									ipld_core::cid::Cid::try_from(
										binary.as_slice(),
									)
									.ok()
								})
								.expect(
									"an `AtprotoValue` holds only `$link`s that parse",
								),
						)
					}
					[(key, Value::Str(base64))] if key.as_str() == "$bytes" => {
						Ipld::Bytes(
							base64::engine::general_purpose::STANDARD_NO_PAD
								.decode(base64.trim_end_matches('='))
								.ok()
								.expect(
									"an `AtprotoValue` holds only `$bytes` that decode",
								),
						)
					}
					entries => Ipld::Map(
						entries
							.iter()
							.map(|(key, value)| {
								(key.to_string(), Self::to_ipld(value))
							})
							.collect(),
					),
				}
			}
		}
	}

	/// The json form of a decoded block: a link becomes `{"$link": "<cid>"}`
	/// and bytes `{"$bytes": "<base64>"}`, integers read as json reads them
	/// (unsigned when not negative). A float passes through for
	/// [`from_wire`](Self::from_wire) to refuse by name.
	fn from_ipld(ipld: Ipld) -> Result<Value> {
		match ipld {
			Ipld::Null => Value::Null,
			Ipld::Bool(bool) => Value::Bool(bool),
			Ipld::Integer(int) => {
				if let Ok(uint) = u64::try_from(int) {
					Value::Uint(uint)
				} else if let Ok(int) = i64::try_from(int) {
					Value::Int(int)
				} else {
					bevybail!(
						"`{int}` is past the atproto data model's signed 64 bit integers"
					)
				}
			}
			Ipld::Float(float) => Value::Float(float),
			Ipld::String(string) => Value::Str(string.into()),
			Ipld::Bytes(bytes) => value!({
				"$bytes": (base64::engine::general_purpose::STANDARD_NO_PAD
					.encode(bytes))
			}),
			Ipld::List(items) => Value::List(
				items
					.into_iter()
					.map(Self::from_ipld)
					.collect::<Result<_>>()?,
			),
			Ipld::Map(entries) => Value::Map(
				entries
					.into_iter()
					.map(|(key, value)| {
						Self::from_ipld(value).map(|value| (key.into(), value))
					})
					.collect::<Result<_>>()?,
			),
			Ipld::Link(cid) => value!({
				"$link": (Cid::from_binary(&cid.to_bytes()).as_str())
			}),
		}
		.xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// `record` parsed from the json a PDS answered.
	#[cfg(feature = "json")]
	fn record(json: &str) -> AtprotoValue {
		AtprotoValue::from_json(serde_json::from_str(json).unwrap()).unwrap()
	}

	/// beet.org's profile, read from its PDS with the cid the PDS computed. It
	/// carries a blob, so the `$link` must encode as a tag 42 link for the
	/// cid to match.
	#[cfg(feature = "json")]
	#[crate::test]
	fn matches_a_pds_record_with_a_link() {
		record(
			r#"{"$type":"app.bsky.actor.profile","avatar":{"ref":{"$link":"bafkreievhbcfvoqgwlttfzvismqchhb3hphb6dss7lsbtqvv42x7cipzxe"},"size":10012,"$type":"blob","mimeType":"image/jpeg"},"createdAt":"2026-09-12T00:20:37.584Z","displayName":""}"#,
		)
		.cid()
		.as_str()
		.xpect_eq("bafyreiatdl7asot7lskiljqp2ulow4tm6tlmtxou6ej6yrpbq3fw7htmau");
	}

	/// A reply, nested strong refs and multi-byte text, read from a real PDS.
	#[cfg(feature = "json")]
	#[crate::test]
	fn matches_a_pds_post() {
		record(
			r#"{"$type":"app.bsky.feed.post","createdAt":"2026-09-23T15:22:16.682Z","langs":["en"],"reply":{"parent":{"cid":"bafyreihzukpzbnzrlnlrg4bweuercgc3r5tz6f37e6qr3n34rneslivaza","uri":"at://did:plc:y2aci3l7tvrs3vuoz6tou2eb/app.bsky.feed.post/3mw5t5lac4k2y"},"root":{"cid":"bafyreihzukpzbnzrlnlrg4bweuercgc3r5tz6f37e6qr3n34rneslivaza","uri":"at://did:plc:y2aci3l7tvrs3vuoz6tou2eb/app.bsky.feed.post/3mw5t5lac4k2y"}},"text":"my plane left for thailand just as you posted, im getting married here in a month\n🤘🇹🇭🤘\n    😁"}"#,
		)
		.cid()
		.as_str()
		.xpect_eq("bafyreifvk5qurbc64bidnt2abjbfs3tbux5ol2qc674ga5fs3agkd474am");
	}

	/// Bytes in their json form encode as bytes, padded or not, and a beet
	/// byte string reaches the same encoding.
	#[crate::test]
	fn encodes_bytes() {
		let encode = |value: Value| {
			AtprotoValue::from_wire(value).unwrap().to_dag_cbor()
		};
		let padded = encode(value!({ "b": { "$bytes": "aGk=" } }));
		encode(value!({ "b": { "$bytes": "aGk" } })).xpect_eq(padded.clone());
		AtprotoValue::try_from(value!({ "b": (Value::Bytes(b"hi".to_vec())) }))
			.unwrap()
			.to_dag_cbor()
			.xpect_eq(padded);
	}

	/// A float hashes as the `org.beet.core#float` object it crossed as.
	#[crate::test]
	fn hashes_a_float() {
		AtprotoValue::try_from(value!({ "x": 0.5 }))
			.unwrap()
			.cid()
			.xpect_eq(
				AtprotoValue::from_wire(value!({
					"x": { "$type": "org.beet.core#float", "value": "0.5" }
				}))
				.unwrap()
				.cid(),
			);
	}

	/// A float, a byte string and a link each survive the block, read back
	/// both as the sealed value and as the beet value it decodes to.
	#[crate::test]
	fn round_trips_the_data_model() {
		let link = Cid::raw(b"blob");
		for value in [
			value!({ "x": 0.5 }),
			value!({ "b": (Value::Bytes(vec![0, 159, 255])) }),
			value!({ "l": { "$link": (link.as_str()) } }),
			value!({
				"n": (Value::Int(-3)),
				"u": (Value::Uint(7)),
				"list": [true, null, "text"]
			}),
		] {
			let sealed = AtprotoValue::try_from(value.clone()).unwrap();
			let block = MediaType::DagCbor.serialize(&value).unwrap();
			block.clone().xpect_eq(sealed.to_dag_cbor());
			MediaType::DagCbor
				.deserialize::<AtprotoValue>(&block)
				.unwrap()
				.xpect_eq(sealed);
			MediaType::DagCbor
				.deserialize::<Value>(&block)
				.unwrap()
				.xpect_eq(value);
		}
	}

	/// A link is CBOR tag 42, not a map, which is what a PDS hashes.
	#[crate::test]
	fn writes_a_link_as_tag_42() {
		let link = Cid::raw(b"blob");
		let block = MediaType::DagCbor
			.serialize(&value!({ "$link": (link.as_str()) }))
			.unwrap();
		// tag 42 is `0xd8 0x2a`, then a byte string of a zero byte and the cid
		block[..2].to_vec().xpect_eq(vec![0xd8, 0x2a]);
	}

	/// A block holding a float is not in the data model.
	#[crate::test]
	fn refuses_a_float_block() {
		MediaType::DagCbor
			.deserialize::<AtprotoValue>(&[0xfb, 0x3f, 0xe0, 0, 0, 0, 0, 0, 0])
			.unwrap_err()
			.to_string()
			.xpect_contains("no floats");
	}
}
