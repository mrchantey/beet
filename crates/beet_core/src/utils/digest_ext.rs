//! Digests as lowercase hex, over bytes or a file, generic over the
//! [`Digest`] the caller picks (`sha2::Sha256`, `md5::Md5`): one spelling
//! of the hex loop and one chunked file read, rather than each site's own.
use crate::prelude::*;
pub use digest::Digest;

/// The digest of `bytes` as lowercase hex.
pub fn hex<D: Digest>(bytes: &[u8]) -> String {
	to_hex(D::digest(bytes).as_slice())
}

/// The digest of the file at `path` as lowercase hex, read in chunks so a
/// multi-gigabyte file is never held whole.
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
pub fn hex_file<D: Digest>(
	path: impl AsRef<std::path::Path>,
) -> FsResult<String> {
	use std::io::Read;
	let path = path.as_ref();
	let mut file =
		std::fs::File::open(path).map_err(|err| FsError::io(path, err))?;
	let mut hasher = D::new();
	let mut buffer = vec![0u8; 1 << 20];
	loop {
		let read = file
			.read(&mut buffer)
			.map_err(|err| FsError::io(path, err))?;
		if read == 0 {
			break;
		}
		hasher.update(&buffer[..read]);
	}
	to_hex(hasher.finalize().as_slice()).xok()
}

/// Bytes as lowercase hex.
pub fn to_hex(bytes: &[u8]) -> String {
	bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(all(test, feature = "fs", not(target_arch = "wasm32")))]
mod test {
	use crate::prelude::*;

	/// The hex of bytes and of a file holding the same bytes agree, through
	/// md5 as any store's stat spells it.
	#[crate::test]
	fn file_and_bytes_agree() {
		let dir = TempDir::new_ws().unwrap();
		let path = dir.join("bytes.bin");
		let bytes = (0..3_000_000u32).map(|n| n as u8).collect::<Vec<_>>();
		fs_ext::write(&path, &bytes).unwrap();
		digest_ext::hex_file::<md5::Md5>(&path)
			.unwrap()
			.xpect_eq(digest_ext::hex::<md5::Md5>(&bytes));
		digest_ext::hex::<md5::Md5>(b"")
			.xpect_eq("d41d8cd98f00b204e9800998ecf8427e");
		digest_ext::to_hex(&[0, 15, 255]).xpect_eq("000fff");
	}
}
