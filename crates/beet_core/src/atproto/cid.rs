use super::string_primitive;
use crate::prelude::*;
use sha2::Digest;
use sha2::Sha256;

/// A content id, the protocol's own: CIDv1 over a sha2-256 multihash, printed
/// base32 lower behind the `b` multibase prefix, ie `bafkrei..` for raw bytes
/// and `bafyrei..` for a record.
///
/// Computed rather than asked for, so a local file compares against a record's
/// blob without an upload, and a record's cid is known before it is written.
/// Hand written on `sha2`: a CIDv1 is a version varint, a codec varint, then a
/// multihash of `0x12 0x20` and thirty-two bytes.
///
/// ```
/// # use beet_core::prelude::*;
/// let cid = Cid::raw(b"hello");
/// cid.as_str().xpect_starts_with("bafkrei");
/// Cid::parse(cid.as_str()).unwrap().xpect_eq(cid.clone());
/// cid.codec().xpect_eq(Cid::RAW);
/// ```
#[derive(
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "SmolStr", into = "SmolStr"))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct Cid(SmolStr);

string_primitive!(Cid);

impl Cid {
	/// The multicodec of raw bytes, the codec of a blob.
	pub const RAW: u64 = 0x55;
	/// The multicodec of DAG-CBOR, the codec of a record.
	pub const DAG_CBOR: u64 = 0x71;
	/// The multihash code of sha2-256.
	const SHA2_256: u8 = 0x12;
	/// The sha2-256 digest length.
	const DIGEST_LEN: u8 = 32;
	/// The RFC 4648 base32 alphabet, lowercase, which the `b` multibase names.
	const BASE32: &'static [u8; 32] = b"abcdefghijklmnopqrstuvwxyz234567";

	/// The cid of `bytes` as a blob, what `uploadBlob` answers for them.
	pub fn raw(bytes: &[u8]) -> Self { Self::new(Self::RAW, bytes) }

	/// The cid of `bytes` already encoded under `codec`, ie [`Self::DAG_CBOR`]
	/// for a record's canonical encoding.
	pub fn new(codec: u64, bytes: &[u8]) -> Self {
		let mut binary = Vec::with_capacity(36);
		Self::write_varint(1, &mut binary);
		Self::write_varint(codec, &mut binary);
		binary.push(Self::SHA2_256);
		binary.push(Self::DIGEST_LEN);
		binary.extend_from_slice(Sha256::digest(bytes).as_slice());
		Self::from_binary(&binary)
	}

	/// Parse the printed form, refusing anything but a CIDv1 over sha2-256
	/// under the raw or DAG-CBOR codec, the only cids the protocol blesses.
	pub fn parse(text: &str) -> Result<Self> {
		Self::decode(text)?;
		Self(SmolStr::new(text)).xok()
	}

	/// The multicodec this cid was computed under, [`Self::RAW`] or
	/// [`Self::DAG_CBOR`].
	pub fn codec(&self) -> u64 {
		// parsed on construction, so the binary form is always well formed
		self.to_binary()
			.ok()
			.and_then(|binary| Self::read_varint(&binary[1..]))
			.map(|(codec, _)| codec)
			.unwrap_or_default()
	}

	/// The binary form, `version, codec, multihash`, which DAG-CBOR embeds
	/// behind a zero byte as a tag 42 link.
	pub fn to_binary(&self) -> Result<Vec<u8>> {
		Self::base32_decode(self.multibase_body()?)
	}

	/// The cid of a binary form, the inverse of [`Self::to_binary`].
	pub fn from_binary(binary: &[u8]) -> Self {
		Self(format!("b{}", Self::base32_encode(binary)).into())
	}

	/// The base32 text behind the `b` prefix.
	fn multibase_body(&self) -> Result<&str> {
		self.0.strip_prefix('b').ok_or_else(|| {
			bevyhow!(
				"cid `{}` is not base32: expected the `b` multibase prefix",
				self.0
			)
		})
	}

	/// Validate the printed form, answering its binary form.
	fn decode(text: &str) -> Result<Vec<u8>> {
		let Some(body) = text.strip_prefix('b') else {
			bevybail!(
				"cid `{text}` is not base32: expected the `b` multibase prefix"
			);
		};
		let binary = Self::base32_decode(body)
			.map_err(|err| bevyhow!("cid `{text}`: {err}"))?;
		let Some((version, rest)) = Self::read_varint(&binary) else {
			bevybail!("cid `{text}` is truncated");
		};
		let Some((codec, rest)) = Self::read_varint(&binary[rest..])
			.map(|(codec, len)| (codec, rest + len))
		else {
			bevybail!("cid `{text}` is truncated");
		};
		if version != 1 {
			bevybail!("cid `{text}` is version {version}, expected 1");
		}
		if codec != Self::RAW && codec != Self::DAG_CBOR {
			bevybail!(
				"cid `{text}` has codec {codec:#x}, expected raw (0x55) or dag-cbor (0x71)"
			);
		}
		match &binary[rest..] {
			[Self::SHA2_256, Self::DIGEST_LEN, digest @ ..]
				if digest.len() == Self::DIGEST_LEN as usize =>
			{
				binary.xok()
			}
			_ => bevybail!("cid `{text}` is not a sha2-256 multihash"),
		}
	}

	/// An unsigned LEB128 varint, the multiformats integer encoding.
	fn write_varint(mut value: u64, out: &mut Vec<u8>) {
		loop {
			let byte = (value & 0x7f) as u8;
			value >>= 7;
			if value == 0 {
				out.push(byte);
				return;
			}
			out.push(byte | 0x80);
		}
	}

	/// The varint at the start of `bytes` and how many bytes it took.
	fn read_varint(bytes: &[u8]) -> Option<(u64, usize)> {
		let mut value = 0u64;
		for (i, byte) in bytes.iter().enumerate().take(9) {
			value |= ((byte & 0x7f) as u64) << (7 * i);
			if byte & 0x80 == 0 {
				return Some((value, i + 1));
			}
		}
		None
	}

	fn base32_encode(bytes: &[u8]) -> String {
		let mut out = String::with_capacity(bytes.len() * 8 / 5 + 1);
		let (mut buffer, mut bits) = (0u32, 0u32);
		for byte in bytes {
			buffer = (buffer << 8) | *byte as u32;
			bits += 8;
			while bits >= 5 {
				bits -= 5;
				out.push(
					Self::BASE32[((buffer >> bits) & 31) as usize] as char,
				);
			}
		}
		if bits > 0 {
			out.push(
				Self::BASE32[((buffer << (5 - bits)) & 31) as usize] as char,
			);
		}
		out
	}

	fn base32_decode(text: &str) -> Result<Vec<u8>> {
		let mut out = Vec::with_capacity(text.len() * 5 / 8);
		let (mut buffer, mut bits) = (0u32, 0u32);
		for char in text.bytes() {
			let Some(value) =
				Self::BASE32.iter().position(|item| *item == char)
			else {
				bevybail!(
					"`{}` is not a lowercase base32 character",
					char as char
				);
			};
			buffer = (buffer << 5) | value as u32;
			bits += 5;
			if bits >= 8 {
				bits -= 8;
				out.push((buffer >> bits) as u8);
			}
		}
		out.xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// The known answers, computed independently of this implementation.
	#[crate::test]
	fn computes_known_cids() {
		Cid::raw(b"").as_str().xpect_eq(
			"bafkreihdwdcefgh4dqkjv67uzcmw7ojee6xedzdetojuzjevtenxquvyku",
		);
		Cid::raw(b"hello world").as_str().xpect_eq(
			"bafkreifzjut3te2nhyekklss27nh3k72ysco7y32koao5eei66wof36n5e",
		);
	}

	#[crate::test]
	fn round_trips_the_binary_form() {
		let cid = Cid::new(Cid::DAG_CBOR, b"{}");
		cid.as_str().xpect_starts_with("bafyrei");
		cid.codec().xpect_eq(Cid::DAG_CBOR);
		Cid::from_binary(&cid.to_binary().unwrap()).xpect_eq(cid);
	}

	#[crate::test]
	fn parses_a_pds_cid() {
		let text =
			"bafyreiatdl7asot7lskiljqp2ulow4tm6tlmtxou6ej6yrpbq3fw7htmau";
		Cid::parse(text).unwrap().codec().xpect_eq(Cid::DAG_CBOR);
	}

	#[crate::test]
	fn rejects_malformed() {
		Cid::parse("").unwrap_err();
		// base58, a CIDv0
		Cid::parse("QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG")
			.unwrap_err()
			.to_string()
			.xpect_contains("multibase");
		// truncated digest
		Cid::parse("bafkreihdwdcefgh4dqkjv67uzcmw7oje")
			.unwrap_err()
			.to_string()
			.xpect_contains("sha2-256");
		Cid::parse("bafkrei!").unwrap_err();
	}
}
