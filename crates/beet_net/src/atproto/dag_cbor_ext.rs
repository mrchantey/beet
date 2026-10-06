//! A record in canonical DAG-CBOR, the encoding its cid is computed over.
//!
//! A record travels as json, the protocol's own json form: a link is
//! `{"$link": "<cid>"}` and bytes are `{"$bytes": "<base64>"}`. Its cid is the
//! sha2-256 of its DAG-CBOR, where a link is CBOR tag 42 and map keys sort
//! length first. Computed here exactly as a PDS computes it, so a record
//! written to an [`EmulatorPds`](crate::prelude::EmulatorPds) and one written
//! to a real PDS answer the same cid.
//!
//! ```
//! # use beet_core::prelude::*;
//! # use beet_net::prelude::*;
//! let record = value!({ "$type": "app.bsky.feed.post", "text": "hi" });
//! dag_cbor_ext::record_cid(&record)
//! 	.unwrap()
//! 	.as_str()
//! 	.xpect_starts_with("bafyrei");
//! ```
use base64::Engine;
use beet_core::prelude::*;
use ipld_core::ipld::Ipld;

/// The cid of `record`, CIDv1 over its canonical DAG-CBOR.
pub fn record_cid(record: &Value) -> Result<Cid> {
	encode(record)?
		.xmap(|bytes| Cid::new(Cid::DAG_CBOR, &bytes))
		.xok()
}

/// `record` in canonical DAG-CBOR. A float is an error, since the data model
/// has none: a record carrying one is not a record any PDS accepts.
pub fn encode(record: &Value) -> Result<Vec<u8>> {
	serde_ipld_dagcbor::to_vec(&to_ipld(record)?)?.xok()
}

/// The data model's view of a json value: `$link` and `$bytes` objects become
/// the link and the bytes they stand for.
fn to_ipld(value: &Value) -> Result<Ipld> {
	match value {
		Value::Null => Ipld::Null,
		Value::Bool(bool) => Ipld::Bool(*bool),
		Value::Int(int) => Ipld::Integer(*int as i128),
		Value::Uint(uint) => Ipld::Integer(*uint as i128),
		Value::Float(float) => bevybail!(
			"the atproto data model has no floats, found {float}: write it \
			 as an integer or a string"
		),
		Value::Bytes(bytes) => Ipld::Bytes(bytes.clone()),
		Value::Str(string) => Ipld::String(string.to_string()),
		Value::List(items) => {
			Ipld::List(items.iter().map(to_ipld).collect::<Result<_>>()?)
		}
		Value::Map(map) => match map.into_iter().collect::<Vec<_>>().as_slice()
		{
			[(key, Value::Str(cid))] if key.as_str() == "$link" => {
				Ipld::Link(
					ipld_core::cid::Cid::try_from(
						Cid::parse(cid)?.to_binary()?.as_slice(),
					)
					// no_std, so the cid crate's error is not an `Error`
					.map_err(|err| bevyhow!("link `{cid}`: {err}"))?,
				)
			}
			[(key, Value::Str(base64))] if key.as_str() == "$bytes" => {
				Ipld::Bytes(
					base64::engine::general_purpose::STANDARD_NO_PAD
						.decode(base64.trim_end_matches('='))
						// without `base64/std` the error is not an `Error`
						.map_err(|err| {
							bevyhow!("`$bytes` is not base64: {err}")
						})?,
				)
			}
			entries => Ipld::Map(
				entries
					.iter()
					.map(|(key, value)| {
						(key.to_string(), to_ipld(value)).xmap(
							|(key, value)| value.map(|value| (key, value)),
						)
					})
					.collect::<Result<_>>()?,
			),
		},
	}
	.xok()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// `record` parsed from the json a PDS answered.
	fn record(json: &str) -> Value {
		Value::from_json(serde_json::from_str(json).unwrap())
	}

	/// beet.org's profile, read from its PDS with the cid the PDS computed. It
	/// carries a blob, so the `$link` must encode as a tag 42 link for the
	/// cid to match.
	#[beet_core::test]
	fn matches_a_pds_record_with_a_link() {
		dag_cbor_ext::record_cid(&record(
			r#"{"$type":"app.bsky.actor.profile","avatar":{"ref":{"$link":"bafkreievhbcfvoqgwlttfzvismqchhb3hphb6dss7lsbtqvv42x7cipzxe"},"size":10012,"$type":"blob","mimeType":"image/jpeg"},"createdAt":"2026-09-12T00:20:37.584Z","displayName":""}"#,
		))
		.unwrap()
		.as_str()
		.xpect_eq("bafyreiatdl7asot7lskiljqp2ulow4tm6tlmtxou6ej6yrpbq3fw7htmau");
	}

	/// A reply, nested strong refs and multi-byte text, read from a real PDS.
	#[beet_core::test]
	fn matches_a_pds_post() {
		dag_cbor_ext::record_cid(&record(
			r#"{"$type":"app.bsky.feed.post","createdAt":"2026-09-23T15:22:16.682Z","langs":["en"],"reply":{"parent":{"cid":"bafyreihzukpzbnzrlnlrg4bweuercgc3r5tz6f37e6qr3n34rneslivaza","uri":"at://did:plc:y2aci3l7tvrs3vuoz6tou2eb/app.bsky.feed.post/3mw5t5lac4k2y"},"root":{"cid":"bafyreihzukpzbnzrlnlrg4bweuercgc3r5tz6f37e6qr3n34rneslivaza","uri":"at://did:plc:y2aci3l7tvrs3vuoz6tou2eb/app.bsky.feed.post/3mw5t5lac4k2y"}},"text":"my plane left for thailand just as you posted, im getting married here in a month\n🤘🇹🇭🤘\n    😁"}"#,
		))
		.unwrap()
		.as_str()
		.xpect_eq("bafyreifvk5qurbc64bidnt2abjbfs3tbux5ol2qc674ga5fs3agkd474am");
	}

	/// Bytes in their json form encode as bytes, padded or not.
	#[beet_core::test]
	fn decodes_bytes() {
		let padded =
			dag_cbor_ext::encode(&value!({ "b": { "$bytes": "aGk=" } }))
				.unwrap();
		dag_cbor_ext::encode(&value!({ "b": { "$bytes": "aGk" } }))
			.unwrap()
			.xpect_eq(padded.clone());
		dag_cbor_ext::encode(&value!({ "b": (Value::Bytes(b"hi".to_vec())) }))
			.unwrap()
			.xpect_eq(padded);
	}

	#[beet_core::test]
	fn refuses_a_float() {
		dag_cbor_ext::encode(&value!({ "x": 0.5 }))
			.unwrap_err()
			.to_string()
			.xpect_contains("no floats");
	}
}
