//! JSONL: one JSON value per line, [`MediaType::Jsonl`], and its zstd
//! compressed form, `.jsonl.zst`.
//!
//! [`MediaType::Jsonl`]'s serde writes and reads a sequence through these
//! helpers. A writer whose lines are already text, ie canonical lines it
//! diffs by before writing, frames them with [`join`] and reads them back with
//! [`split`].
//!
//! ```
//! # use beet_core::prelude::*;
//! let lines = ["{\"a\":1}", "{\"a\":2}"];
//! let packed =
//! 	jsonl_ext::compress(&jsonl_ext::join(lines), jsonl_ext::Level::Fast)
//! 		.unwrap();
//! let text = jsonl_ext::decompress(&packed).unwrap();
//! assert_eq!(jsonl_ext::split(&text).unwrap(), lines);
//! ```
use crate::prelude::*;
use std::io::Read;

/// The extension of a JSONL object, without its dot.
pub const EXTENSION: &str = "jsonl";

/// The extension of a zstd compressed JSONL object, without its dot.
pub const ZSTD_EXTENSION: &str = "jsonl.zst";

/// How hard [`compress`] works.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Level {
	/// zstd 3, the library default: the cheap pass, for an object read once.
	Fast,
	/// zstd 19, its slowest single threaded level, spread over every core
	/// for a large object: for an object kept for good.
	Best,
}

/// `lines`, each followed by a newline.
pub fn join(lines: impl IntoIterator<Item = impl AsRef<str>>) -> Vec<u8> {
	let mut bytes = Vec::new();
	for line in lines {
		bytes.extend(line.as_ref().as_bytes());
		bytes.push(b'\n');
	}
	bytes
}

/// The lines of `bytes`, blank ones dropped.
pub fn split(bytes: &[u8]) -> Result<Vec<String>> {
	core::str::from_utf8(bytes)?
		.lines()
		.filter(|line| !line.trim().is_empty())
		.map(String::from)
		.collect::<Vec<_>>()
		.xok()
}

/// `bytes` under zstd at `level`. The pure-rust encoder on wasm reaches only
/// its fastest level, so a wasm writer trades size for building anywhere.
pub fn compress(bytes: &[u8], level: Level) -> Result<Vec<u8>> {
	cfg_if! {
		if #[cfg(target_arch = "wasm32")] {
			let _ = level;
			ruzstd::encoding::compress_to_vec(
				bytes,
				ruzstd::encoding::CompressionLevel::Fastest,
			)
			.xok()
		} else {
			use std::io::Write;
			let mut encoder =
				zstd::stream::write::Encoder::new(Vec::new(), match level {
					Level::Fast => 3,
					Level::Best => 19,
				})?;
			// the slow level is minutes per gigabyte on one core; zstd splits a
			// large input into jobs across the machine, a small one stays one job
			if let (Level::Best, Ok(cores)) =
				(level, std::thread::available_parallelism())
			{
				encoder.multithread(cores.get() as u32)?;
			}
			encoder.write_all(bytes)?;
			encoder.finish()?.xok()
		}
	}
}

/// The bytes a [`compress`] packed, on either target: both write the
/// standard frame.
pub fn decompress(bytes: &[u8]) -> Result<Vec<u8>> {
	let mut out = Vec::new();
	cfg_if! {
		if #[cfg(target_arch = "wasm32")] {
			ruzstd::decoding::StreamingDecoder::new(bytes)?.read_to_end(&mut out)?;
		} else {
			zstd::stream::read::Decoder::new(bytes)?.read_to_end(&mut out)?;
		}
	}
	out.xok()
}

/// One line per element of `value`, a sequence, through the data model.
pub(crate) fn serialize<T: serde::Serialize>(value: &T) -> Result<Vec<u8>> {
	match Value::from_serde(value)? {
		Value::List(items) => join(items.iter().map(Value::to_json_string)).xok(),
		other => bevybail!(
			"JSONL writes a sequence, one value per line, not {}",
			other.kind()
		),
	}
}

/// A sequence of one value per line of `bytes`.
pub(crate) fn deserialize<T: serde::de::DeserializeOwned>(
	bytes: &[u8],
) -> Result<T> {
	split(bytes)?
		.iter()
		.map(|line| Value::parse_json(line))
		.collect::<Result<Vec<_>>>()?
		.xmap(Value::List)
		.into_serde()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use serde::Deserialize;
	use serde::Serialize;

	#[derive(Debug, PartialEq, Serialize, Deserialize)]
	struct Row {
		id: u32,
		name: String,
	}

	fn rows() -> Vec<Row> {
		vec![Row { id: 1, name: "a".into() }, Row {
			id: 2,
			name: "b".into(),
		}]
	}

	#[crate::test]
	fn media_type_round_trips_rows() {
		let bytes = MediaType::Jsonl.serialize(&rows()).unwrap();
		String::from_utf8(bytes.clone())
			.unwrap()
			.xpect_eq("{\"id\":1,\"name\":\"a\"}\n{\"id\":2,\"name\":\"b\"}\n");
		MediaType::Jsonl
			.deserialize::<Vec<Row>>(&bytes)
			.unwrap()
			.xpect_eq(rows());
		MediaType::Jsonl
			.serialize(&Row { id: 1, name: "a".into() })
			.unwrap_err()
			.to_string()
			.xpect_contains("writes a sequence");
	}

	#[crate::test]
	fn compresses_at_every_level() {
		let text = jsonl_ext::join(["{\"a\":1}", "", "{\"a\":2}"]);
		for level in [jsonl_ext::Level::Fast, jsonl_ext::Level::Best] {
			let packed = jsonl_ext::compress(&text, level).unwrap();
			// the zstd frame magic
			packed[..4].to_vec().xpect_eq(vec![0x28, 0xb5, 0x2f, 0xfd]);
			jsonl_ext::split(&jsonl_ext::decompress(&packed).unwrap())
				.unwrap()
				.xpect_eq(vec!["{\"a\":1}".to_string(), "{\"a\":2}".to_string()]);
		}
	}
}
