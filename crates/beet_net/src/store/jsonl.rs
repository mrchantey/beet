//! The JSONL codec: canonical json rows, one per line, compressed by gzip or
//! zstd, the codec named by the object's extension.
use crate::exports::bytes::Bytes;
use beet_core::prelude::*;
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::io::Read;
use std::io::Write;

/// The compression under a JSONL object, one per file extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JsonlCodec {
	/// `.jsonl.gz`: pure rust, every target, the analytics default.
	Gzip,
	/// `.jsonl.zst` at level 19, for objects written once and read many times.
	/// Native only, the crate binds the C library.
	#[cfg(feature = "zstd")]
	Zstd,
}

impl JsonlCodec {
	/// Every extension a JSONL object may carry, whether or not its codec is
	/// compiled in, for the error an unknown suffix reports.
	const EXTENSIONS: &'static [&'static str] = &["jsonl.gz", "jsonl.zst"];

	/// The extension an object under this codec carries, without the leading
	/// dot: `jsonl.gz` or `jsonl.zst`.
	pub fn extension(&self) -> &'static str {
		match self {
			Self::Gzip => "jsonl.gz",
			#[cfg(feature = "zstd")]
			Self::Zstd => "jsonl.zst",
		}
	}

	/// Picks the codec from a path's suffix, erroring on an unknown one or on
	/// a codec this build lacks.
	pub fn from_path(path: &RelPath) -> Result<Self> {
		let path = path.as_str();
		if path.ends_with(".jsonl.gz") {
			return Self::Gzip.xok();
		}
		if path.ends_with(".jsonl.zst") {
			#[cfg(feature = "zstd")]
			return Self::Zstd.xok();
			#[cfg(not(feature = "zstd"))]
			bevybail!(
				"`{path}` is a zstd JSONL object, which requires the `zstd` feature"
			);
		}
		bevybail!(
			"`{path}` is not a JSONL object, expected one of: {}",
			Self::EXTENSIONS.join(", ")
		)
	}

	/// Compresses the encoded lines.
	fn compress(&self, lines: &[u8]) -> Result<Vec<u8>> {
		match self {
			Self::Gzip => {
				let mut encoder =
					GzEncoder::new(Vec::new(), Compression::default());
				encoder.write_all(lines)?;
				encoder.finish()?.xok()
			}
			#[cfg(feature = "zstd")]
			Self::Zstd => {
				let mut encoder = zstd::stream::write::Encoder::new(
					Vec::new(),
					Self::ZSTD_LEVEL,
				)?;
				encoder.write_all(lines)?;
				encoder.finish()?.xok()
			}
		}
	}

	/// Decompresses an object to its text.
	fn decompress(&self, bytes: &[u8]) -> Result<String> {
		let mut text = String::new();
		match self {
			Self::Gzip => GzDecoder::new(bytes).read_to_string(&mut text)?,
			#[cfg(feature = "zstd")]
			Self::Zstd => zstd::stream::read::Decoder::new(bytes)?
				.read_to_string(&mut text)?,
		};
		text.xok()
	}

	/// The slowest, smallest setting: a segment is written once and read many
	/// times, and on real JSONL zstd 19 beats gzip 9 by about 15 percent.
	#[cfg(feature = "zstd")]
	const ZSTD_LEVEL: i32 = 19;
}

/// Newline-delimited canonical json under a [`JsonlCodec`].
///
/// Every row is serialised through `serde_json::Value`, whose objects sort
/// their keys, so the same information always encodes to the same line
/// whatever the field order of the type or the insertion order of a map.
pub struct Jsonl;

impl Jsonl {
	/// Encodes one canonical line per row, in the order given.
	pub fn encode<'a, T: 'a + Serialize>(
		codec: JsonlCodec,
		rows: impl IntoIterator<Item = &'a T>,
	) -> Result<Bytes> {
		rows.into_iter()
			.map(Self::line)
			.collect::<Result<Vec<_>>>()?
			.xmap(|lines| Self::encode_lines(codec, &lines))
	}

	/// Encodes lines already in their canonical form, ie from
	/// [`line`](Self::line), one per row in the order given.
	pub fn encode_lines(
		codec: JsonlCodec,
		lines: impl IntoIterator<Item = impl AsRef<str>>,
	) -> Result<Bytes> {
		let mut bytes = Vec::new();
		for line in lines {
			bytes.extend(line.as_ref().as_bytes());
			bytes.push(b'\n');
		}
		Bytes::from(codec.compress(&bytes)?).xok()
	}

	/// The canonical json line for one row, without its newline.
	pub fn line<T: Serialize>(row: &T) -> Result<String> {
		serde_json::to_string(&serde_json::to_value(row)?)?.xok()
	}

	/// Decodes every line as `T`; a stored object's codec is
	/// [`JsonlCodec::from_path`].
	pub fn decode<T: DeserializeOwned>(
		codec: JsonlCodec,
		bytes: &[u8],
	) -> Result<Vec<T>> {
		Self::decode_lines(codec, bytes)?
			.iter()
			.map(|line| serde_json::from_str(line).map_err(Into::into))
			.collect()
	}

	/// Decodes to the raw lines, blank lines dropped.
	pub fn decode_lines(
		codec: JsonlCodec,
		bytes: &[u8],
	) -> Result<Vec<String>> {
		codec
			.decompress(bytes)?
			.lines()
			.filter(|line| !line.trim().is_empty())
			.map(String::from)
			.collect::<Vec<_>>()
			.xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use serde::Deserialize;
	use serde::Serialize;

	/// Derive order is not alphabetical, so a sorted line proves the canonical
	/// path rather than the derive.
	#[derive(Debug, PartialEq, Serialize, Deserialize)]
	struct Row {
		zed: u32,
		alpha: String,
		mid: Option<bool>,
	}

	fn rows() -> Vec<Row> {
		vec![
			Row {
				zed: 1,
				alpha: "a".into(),
				mid: Some(true),
			},
			Row {
				zed: 2,
				alpha: "b".into(),
				mid: None,
			},
		]
	}

	fn round_trip(codec: JsonlCodec) {
		let bytes = Jsonl::encode(codec, &rows()).unwrap();
		// the extension round-trips too
		JsonlCodec::from_path(&RelPath::new(format!(
			"rows.{}",
			codec.extension()
		)))
		.unwrap()
		.xpect_eq(codec);
		Jsonl::decode::<Row>(codec, &bytes)
			.unwrap()
			.xpect_eq(rows());
		Jsonl::decode_lines(codec, &bytes).unwrap().xpect_eq(vec![
			r#"{"alpha":"a","mid":true,"zed":1}"#.to_string(),
			r#"{"alpha":"b","mid":null,"zed":2}"#.to_string(),
		]);
		// the same rows are the same bytes, whichever entry point
		Jsonl::encode(codec, &rows())
			.unwrap()
			.xpect_eq(bytes.clone());
		Jsonl::encode_lines(codec, [
			r#"{"alpha":"a","mid":true,"zed":1}"#,
			r#"{"alpha":"b","mid":null,"zed":2}"#,
		])
		.unwrap()
		.xpect_eq(bytes);
	}

	#[beet_core::test]
	fn round_trips_sorted_keys_gzip() {
		round_trip(JsonlCodec::Gzip);
		Jsonl::encode(JsonlCodec::Gzip, &rows()).unwrap()[..2]
			.to_vec()
			.xpect_eq(vec![0x1f, 0x8b]);
	}

	#[cfg(feature = "zstd")]
	#[beet_core::test]
	fn round_trips_sorted_keys_zstd() {
		round_trip(JsonlCodec::Zstd);
		Jsonl::encode(JsonlCodec::Zstd, &rows()).unwrap()[..4]
			.to_vec()
			.xpect_eq(vec![0x28, 0xb5, 0x2f, 0xfd]);
	}

	#[beet_core::test]
	fn picks_the_codec_from_the_suffix() {
		JsonlCodec::from_path(&RelPath::new("a/b/2026.jsonl.gz"))
			.unwrap()
			.xpect_eq(JsonlCodec::Gzip);
		#[cfg(feature = "zstd")]
		JsonlCodec::from_path(&RelPath::new("a/b/2026.jsonl.zst"))
			.unwrap()
			.xpect_eq(JsonlCodec::Zstd);
		JsonlCodec::from_path(&RelPath::new("a/b/2026.jsonl"))
			.unwrap_err()
			.to_string()
			.xpect_contains("jsonl.gz, jsonl.zst");
		JsonlCodec::from_path(&RelPath::new("a/b/2026.json"))
			.unwrap_err()
			.to_string()
			.xpect_contains("not a JSONL object");
	}
}
