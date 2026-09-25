//! The JSONL codec: canonical json rows, one per line, compressed under the
//! codec the object's extension names. Every object beet writes is zstd; gzip
//! is read and written only with the `gzip` feature, for objects another tool
//! produced; nothing beet writes uses it.
use crate::exports::bytes::Bytes;
use beet_core::prelude::*;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::io::Read;
#[cfg(any(feature = "gzip", not(target_arch = "wasm32")))]
use std::io::Write;

/// The compression under a JSONL object, one per file extension.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JsonlCodec {
	/// `.jsonl.zst`, the codec every object is written under: the C library
	/// natively and the pure-rust `ruzstd` on wasm, both the standard frame, so
	/// an object written on either target reads on both.
	#[default]
	Zstd,
	/// `.jsonl.gz`, for objects another tool produced.
	#[cfg(feature = "gzip")]
	Gzip,
}

/// How hard a [`JsonlCodec`] works on an object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JsonlLevel {
	/// The cheap pass, for an object read once: a segment written every minute.
	Fast,
	/// The slowest, smallest setting, for an object kept for good: a daily
	/// archive. On real JSONL zstd 19 beats gzip 9 by about 20 percent.
	Best,
}

impl JsonlCodec {
	/// Every extension a JSONL object may carry, the written one first, whether
	/// or not its codec is compiled in.
	pub const EXTENSIONS: &'static [&'static str] = &["jsonl.zst", "jsonl.gz"];

	/// The extension an object under this codec carries, without the leading
	/// dot: `jsonl.zst` or `jsonl.gz`.
	pub fn extension(&self) -> &'static str {
		match self {
			Self::Zstd => "jsonl.zst",
			#[cfg(feature = "gzip")]
			Self::Gzip => "jsonl.gz",
		}
	}

	/// Splits a path into its stem and the JSONL extension it carries, `None`
	/// for a path that is not a JSONL object. Any of [`Self::EXTENSIONS`]
	/// matches, compiled in or not, so a listing never silently drops an object
	/// this build cannot read: [`Self::from_path`] reports that instead.
	pub fn split_path(path: &str) -> Option<(&str, &'static str)> {
		Self::EXTENSIONS.iter().find_map(|ext| {
			path.strip_suffix(ext)?
				.strip_suffix('.')
				.map(|stem| (stem, *ext))
		})
	}

	/// Picks the codec from a path's suffix, erroring on an unknown one or on
	/// a codec this build lacks.
	pub fn from_path(path: &RelPath) -> Result<Self> {
		let Some((_, ext)) = Self::split_path(path.as_str()) else {
			bevybail!(
				"`{path}` is not a JSONL object, expected one of: {}",
				Self::EXTENSIONS.join(", ")
			);
		};
		match ext {
			"jsonl.zst" => Self::Zstd.xok(),
			#[cfg(feature = "gzip")]
			"jsonl.gz" => Self::Gzip.xok(),
			_ => bevybail!(
				"`{path}` is a gzip JSONL object, which needs the `gzip` feature"
			),
		}
	}

	/// Compresses the encoded lines.
	fn compress(&self, level: JsonlLevel, lines: &[u8]) -> Result<Vec<u8>> {
		match self {
			Self::Zstd => level.zstd(lines),
			#[cfg(feature = "gzip")]
			Self::Gzip => {
				let mut encoder =
					flate2::write::GzEncoder::new(Vec::new(), level.gzip());
				encoder.write_all(lines)?;
				encoder.finish()?.xok()
			}
		}
	}

	/// Decompresses an object to its text.
	fn decompress(&self, bytes: &[u8]) -> Result<String> {
		let mut text = String::new();
		match self {
			#[cfg(not(target_arch = "wasm32"))]
			Self::Zstd => zstd::stream::read::Decoder::new(bytes)?
				.read_to_string(&mut text)?,
			#[cfg(target_arch = "wasm32")]
			Self::Zstd => ruzstd::decoding::StreamingDecoder::new(bytes)?
				.read_to_string(&mut text)?,
			#[cfg(feature = "gzip")]
			Self::Gzip => {
				flate2::read::GzDecoder::new(bytes).read_to_string(&mut text)?
			}
		};
		text.xok()
	}
}

impl JsonlLevel {
	/// zstd 3, the library default, and 19, its slowest single-threaded level.
	#[cfg(not(target_arch = "wasm32"))]
	fn zstd(&self, lines: &[u8]) -> Result<Vec<u8>> {
		let level = match self {
			Self::Fast => 3,
			Self::Best => 19,
		};
		let mut encoder = zstd::stream::write::Encoder::new(Vec::new(), level)?;
		// the slow level is minutes per gigabyte on one core; zstd splits a
		// large input into jobs across the machine, a small one stays one job
		if let (Self::Best, Ok(cores)) =
			(self, std::thread::available_parallelism())
		{
			encoder.multithread(cores.get() as u32)?;
		}
		encoder.write_all(lines)?;
		encoder.finish()?.xok()
	}

	/// The pure-rust encoder implements one level, roughly zstd 1, so the
	/// level is moot: a wasm writer trades size for building anywhere.
	#[cfg(target_arch = "wasm32")]
	fn zstd(&self, lines: &[u8]) -> Result<Vec<u8>> {
		ruzstd::encoding::compress_to_vec(
			lines,
			ruzstd::encoding::CompressionLevel::Fastest,
		)
		.xok()
	}

	/// gzip 6, the library default, and 9.
	#[cfg(feature = "gzip")]
	fn gzip(&self) -> flate2::Compression {
		match self {
			Self::Fast => flate2::Compression::default(),
			Self::Best => flate2::Compression::best(),
		}
	}
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
		level: JsonlLevel,
		rows: impl IntoIterator<Item = &'a T>,
	) -> Result<Bytes> {
		rows.into_iter()
			.map(Self::line)
			.collect::<Result<Vec<_>>>()?
			.xmap(|lines| Self::encode_lines(codec, level, &lines))
	}

	/// Encodes lines already in their canonical form, ie from
	/// [`line`](Self::line), one per row in the order given: the entry point
	/// for a writer that diffs by line before it writes.
	pub fn encode_lines(
		codec: JsonlCodec,
		level: JsonlLevel,
		lines: impl IntoIterator<Item = impl AsRef<str>>,
	) -> Result<Bytes> {
		let mut bytes = Vec::new();
		for line in lines {
			bytes.extend(line.as_ref().as_bytes());
			bytes.push(b'\n');
		}
		Bytes::from(codec.compress(level, &bytes)?).xok()
	}

	/// The canonical json line for one row, without its newline.
	pub fn line<T: Serialize>(row: &T) -> Result<String> {
		serde_json::to_string(&serde_json::to_value(row)?)?.xok()
	}

	/// Decodes every line as `T`.
	pub fn decode<T: DeserializeOwned>(
		codec: JsonlCodec,
		bytes: &[u8],
	) -> Result<Vec<T>> {
		Self::decode_lines(codec, bytes)?
			.iter()
			.map(|line| serde_json::from_str(line).map_err(Into::into))
			.collect()
	}

	/// Decodes a stored object under the codec its path names.
	pub fn decode_path<T: DeserializeOwned>(
		path: &RelPath,
		bytes: &[u8],
	) -> Result<Vec<T>> {
		Self::decode(JsonlCodec::from_path(path)?, bytes)
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
	use crate::exports::bytes::Bytes;
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

	/// Round-trips every level and returns the `Best` bytes for a magic check.
	fn round_trip(codec: JsonlCodec) -> Bytes {
		let path = RelPath::new(format!("rows.{}", codec.extension()));
		// the extension round-trips too
		JsonlCodec::from_path(&path).unwrap().xpect_eq(codec);
		for level in [JsonlLevel::Fast, JsonlLevel::Best] {
			let bytes = Jsonl::encode(codec, level, &rows()).unwrap();
			Jsonl::decode_path::<Row>(&path, &bytes)
				.unwrap()
				.xpect_eq(rows());
			Jsonl::decode_lines(codec, &bytes).unwrap().xpect_eq(vec![
				r#"{"alpha":"a","mid":true,"zed":1}"#.to_string(),
				r#"{"alpha":"b","mid":null,"zed":2}"#.to_string(),
			]);
			// the same rows are the same bytes, whichever entry point
			Jsonl::encode(codec, level, &rows())
				.unwrap()
				.xpect_eq(bytes.clone());
			Jsonl::encode_lines(codec, level, [
				r#"{"alpha":"a","mid":true,"zed":1}"#,
				r#"{"alpha":"b","mid":null,"zed":2}"#,
			])
			.unwrap()
			.xpect_eq(bytes);
		}
		Jsonl::encode(codec, JsonlLevel::Best, &rows()).unwrap()
	}

	#[beet_core::test]
	fn round_trips_sorted_keys_zstd() {
		round_trip(JsonlCodec::Zstd)[..4]
			.to_vec()
			.xpect_eq(vec![0x28, 0xb5, 0x2f, 0xfd]);
	}

	#[cfg(feature = "gzip")]
	#[beet_core::test]
	fn round_trips_sorted_keys_gzip() {
		round_trip(JsonlCodec::Gzip)[..2]
			.to_vec()
			.xpect_eq(vec![0x1f, 0x8b]);
	}

	#[beet_core::test]
	fn picks_the_codec_from_the_suffix() {
		JsonlCodec::from_path(&RelPath::new("a/b/2026.jsonl.zst"))
			.unwrap()
			.xpect_eq(JsonlCodec::Zstd);
		JsonlCodec::split_path("a/b/2026.jsonl.gz")
			.xpect_eq(Some(("a/b/2026", "jsonl.gz")));
		#[cfg(feature = "gzip")]
		JsonlCodec::from_path(&RelPath::new("a/b/2026.jsonl.gz"))
			.unwrap()
			.xpect_eq(JsonlCodec::Gzip);
		#[cfg(not(feature = "gzip"))]
		JsonlCodec::from_path(&RelPath::new("a/b/2026.jsonl.gz"))
			.unwrap_err()
			.to_string()
			.xpect_contains("needs the `gzip` feature");
		JsonlCodec::split_path("a/b/2026.jsonl").xpect_eq(None);
		JsonlCodec::from_path(&RelPath::new("a/b/2026.jsonl"))
			.unwrap_err()
			.to_string()
			.xpect_contains("jsonl.zst, jsonl.gz");
		JsonlCodec::from_path(&RelPath::new("a/b/2026.json"))
			.unwrap_err()
			.to_string()
			.xpect_contains("not a JSONL object");
	}
}
