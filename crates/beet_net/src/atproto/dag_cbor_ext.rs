//! A record in canonical DAG-CBOR, the encoding its cid is computed over.
//!
//! A record travels as an [`AtprotoValue`], the protocol's json form, which
//! holds no float by construction: a link is `{"$link": "<cid>"}` and bytes
//! are `{"$bytes": "<base64>"}`. Its cid is the sha2-256 of its DAG-CBOR, where
//! a link is CBOR tag 42 and map keys sort length first. Computed here exactly
//! as a PDS computes it, so a record written to an
//! [`EmulatorPds`](crate::prelude::EmulatorPds) and one written to a real PDS
//! answer the same cid.
//!
//! ```
//! # use beet_core::prelude::*;
//! # use beet_net::prelude::*;
//! let record =
//! 	AtprotoValue::try_from(value!({ "$type": "app.bsky.feed.post", "text": "hi" }))
//! 		.unwrap();
//! dag_cbor_ext::record_cid(&record)
//! 	.as_str()
//! 	.xpect_starts_with("bafyrei");
//! ```
use base64::Engine;
use beet_core::prelude::*;
use ipld_core::ipld::Ipld;

/// The cid of `record`, CIDv1 over its canonical DAG-CBOR.
pub fn record_cid(record: &AtprotoValue) -> Cid {
	Cid::new(Cid::DAG_CBOR, &encode(record))
}

/// `record` in canonical DAG-CBOR. Infallible, since an [`AtprotoValue`] was
/// refused at construction for anything the data model cannot encode.
pub fn encode(record: &AtprotoValue) -> Vec<u8> {
	serde_ipld_dagcbor::to_vec(&to_ipld(record))
		.expect("an `Ipld` encodes into a `Vec`, short of allocation failing")
}

/// The data model's view of a json value: `$link` and `$bytes` objects become
/// the link and the bytes they stand for. Total over what an
/// [`AtprotoValue`] can hold, so each branch it rules out says why.
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
		Value::List(items) => Ipld::List(items.iter().map(to_ipld).collect()),
		Value::Map(map) => match map.into_iter().collect::<Vec<_>>().as_slice()
		{
			[(key, Value::Str(cid))] if key.as_str() == "$link" => Ipld::Link(
				Cid::parse(cid)
					.and_then(|cid| cid.to_binary())
					.ok()
					.and_then(|binary| {
						ipld_core::cid::Cid::try_from(binary.as_slice()).ok()
					})
					.expect("an `AtprotoValue` holds only `$link`s that parse"),
			),
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
					.map(|(key, value)| (key.to_string(), to_ipld(value)))
					.collect(),
			),
		},
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// `record` parsed from the json a PDS answered.
	fn record(json: &str) -> AtprotoValue {
		AtprotoValue::from_json(serde_json::from_str(json).unwrap()).unwrap()
	}

	/// beet.org's profile, read from its PDS with the cid the PDS computed. It
	/// carries a blob, so the `$link` must encode as a tag 42 link for the
	/// cid to match.
	#[beet_core::test]
	fn matches_a_pds_record_with_a_link() {
		dag_cbor_ext::record_cid(&record(
			r#"{"$type":"app.bsky.actor.profile","avatar":{"ref":{"$link":"bafkreievhbcfvoqgwlttfzvismqchhb3hphb6dss7lsbtqvv42x7cipzxe"},"size":10012,"$type":"blob","mimeType":"image/jpeg"},"createdAt":"2026-09-12T00:20:37.584Z","displayName":""}"#,
		))
		.as_str()
		.xpect_eq("bafyreiatdl7asot7lskiljqp2ulow4tm6tlmtxou6ej6yrpbq3fw7htmau");
	}

	/// A reply, nested strong refs and multi-byte text, read from a real PDS.
	#[beet_core::test]
	fn matches_a_pds_post() {
		dag_cbor_ext::record_cid(&record(
			r#"{"$type":"app.bsky.feed.post","createdAt":"2026-09-23T15:22:16.682Z","langs":["en"],"reply":{"parent":{"cid":"bafyreihzukpzbnzrlnlrg4bweuercgc3r5tz6f37e6qr3n34rneslivaza","uri":"at://did:plc:y2aci3l7tvrs3vuoz6tou2eb/app.bsky.feed.post/3mw5t5lac4k2y"},"root":{"cid":"bafyreihzukpzbnzrlnlrg4bweuercgc3r5tz6f37e6qr3n34rneslivaza","uri":"at://did:plc:y2aci3l7tvrs3vuoz6tou2eb/app.bsky.feed.post/3mw5t5lac4k2y"}},"text":"my plane left for thailand just as you posted, im getting married here in a month\n🤘🇹🇭🤘\n    😁"}"#,
		))
		.as_str()
		.xpect_eq("bafyreifvk5qurbc64bidnt2abjbfs3tbux5ol2qc674ga5fs3agkd474am");
	}

	/// Bytes in their json form encode as bytes, padded or not, and a beet
	/// byte string reaches the same encoding.
	#[beet_core::test]
	fn encodes_bytes() {
		let encode = |value: Value| {
			dag_cbor_ext::encode(&AtprotoValue::from_wire(value).unwrap())
		};
		let padded = encode(value!({ "b": { "$bytes": "aGk=" } }));
		encode(value!({ "b": { "$bytes": "aGk" } })).xpect_eq(padded.clone());
		dag_cbor_ext::encode(
			&AtprotoValue::try_from(value!({
				"b": (Value::Bytes(b"hi".to_vec()))
			}))
			.unwrap(),
		)
		.xpect_eq(padded);
	}

	/// A float hashes as the `org.beet.core#float` object it crossed as.
	#[beet_core::test]
	fn hashes_a_float() {
		dag_cbor_ext::record_cid(
			&AtprotoValue::try_from(value!({ "x": 0.5 })).unwrap(),
		)
		.xpect_eq(dag_cbor_ext::record_cid(&record(
			r#"{"x":{"$type":"org.beet.core#float","value":"0.5"}}"#,
		)));
	}
}
