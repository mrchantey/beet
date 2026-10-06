use crate::prelude::*;

/// The protocol's blob reference as it appears inside a record, written
/// `$type: "blob"`, the one `$type` the PDS itself reads.
///
/// A bare [`Cid`] in a record is NOT a reference: the PDS retains a blob only
/// while a current record holds one of these, so the reference is the
/// retention.
///
/// ```json
/// {"$type":"blob","ref":{"$link":"bafkrei.."},"mimeType":"image/png","size":1234}
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
	feature = "serde",
	serde(try_from = "BlobRefWire", into = "BlobRefWire")
)]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct BlobRef {
	/// The blob's content id, under the raw codec.
	pub cid: Cid,
	/// The media type it was uploaded as.
	pub mime_type: MediaType,
	/// Its length in bytes.
	pub size: u64,
}

impl BlobRef {
	/// The `$type` a blob reference is written with.
	pub const TYPE: &'static str = "blob";

	/// The reference `uploadBlob` would answer for `bytes`, computed locally,
	/// so a file compares against a record's blob without an upload.
	pub fn of(bytes: &[u8], mime_type: MediaType) -> Self {
		Self {
			cid: Cid::raw(bytes),
			mime_type,
			size: bytes.len() as u64,
		}
	}
}

/// The wire form: the cid nested as a `$link`, the media type as its string.
#[cfg(feature = "serde")]
#[derive(Serialize, Deserialize)]
struct BlobRefWire {
	#[serde(rename = "$type")]
	r#type: SmolStr,
	r#ref: Link,
	#[serde(rename = "mimeType")]
	mime_type: SmolStr,
	size: u64,
}

/// A DAG-CBOR link in its json form.
#[cfg(feature = "serde")]
#[derive(Serialize, Deserialize)]
struct Link {
	#[serde(rename = "$link")]
	link: Cid,
}

#[cfg(feature = "serde")]
impl TryFrom<BlobRefWire> for BlobRef {
	type Error = BevyError;
	fn try_from(wire: BlobRefWire) -> Result<Self> {
		if wire.r#type != Self::TYPE {
			bevybail!(
				"expected a `$type: \"blob\"` reference, found `{}`",
				wire.r#type
			);
		}
		Self {
			cid: wire.r#ref.link,
			mime_type: MediaType::from_content_type(&wire.mime_type),
			size: wire.size,
		}
		.xok()
	}
}

#[cfg(feature = "serde")]
impl From<BlobRef> for BlobRefWire {
	fn from(blob: BlobRef) -> Self {
		Self {
			r#type: SmolStr::new_static(BlobRef::TYPE),
			r#ref: Link { link: blob.cid },
			mime_type: SmolStr::new(blob.mime_type.as_str()),
			size: blob.size,
		}
	}
}

#[cfg(all(test, feature = "json"))]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn round_trips_the_wire_form() {
		let json = r#"{"$type":"blob","ref":{"$link":"bafkreievhbcfvoqgwlttfzvismqchhb3hphb6dss7lsbtqvv42x7cipzxe"},"mimeType":"image/jpeg","size":10012}"#;
		let blob = serde_json::from_str::<BlobRef>(json).unwrap();
		blob.mime_type.xpect_eq(MediaType::Jpeg);
		blob.size.xpect_eq(10012);
		serde_json::to_string(&blob).unwrap().xpect_eq(json);
	}

	#[crate::test]
	fn computes_a_local_reference() {
		BlobRef::of(b"hello world", MediaType::Text)
			.cid
			.as_str()
			.xpect_eq(
				"bafkreifzjut3te2nhyekklss27nh3k72ysco7y32koao5eei66wof36n5e",
			);
	}
}
