//! Cross-platform file system utilities, including
//! wrappers around [`std::fs`], [`async_fs`] and [`js_runtime`]
//! with ergonomics better suited to the application layer:
//! - outputs the file path on fs error
//! - creates missing directories when writing files
use crate::prelude::*;
use futures_lite::Stream;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::ExitStatus;

/// The workspace relative directory for this file,
/// internally using the `file!()` macro.
/// ## Example
///
/// ```rust
/// # use beet_core::prelude::*;
/// let dir = dir!();
/// ```
#[macro_export]
macro_rules! dir {
	() => {
		std::path::Path::new(file!()).parent().unwrap()
	};
}


/// Get the current working directory
pub fn current_dir() -> FsResult<PathBuf> {
	#[cfg(target_arch = "wasm32")]
	return js_runtime::cwd().xinto::<PathBuf>().xok();
	#[cfg(not(target_arch = "wasm32"))]
	return std::env::current_dir().map_err(|e| FsError::io(".", e));
}

/// Copy a directory recursively, creating it if it doesnt exist
/// This also provides consistent behavior with the `cp` command:
/// -
pub fn copy_recursive(
	source: impl AsRef<Path>,
	destination: impl AsRef<Path>,
) -> FsResult {
	let source = source.as_ref();
	let destination = destination.as_ref();

	fs_ext::create_dir_all(&destination)?;
	for entry in ReadDir::all(source)? {
		let file_name = path_ext::file_name(&entry)?;
		if entry.is_dir() {
			fs_ext::copy_recursive(&entry, destination.join(file_name))?;
		} else {
			fs::copy(&entry, destination.join(file_name))
				.map_err(|err| FsError::io(entry, err))?;
		}
	}
	Ok(())
}


/// Checks if a path exists on the filesystem.
pub fn exists(path: impl AsRef<Path>) -> FsResult<bool> {
	let path = path.as_ref();
	#[cfg(target_arch = "wasm32")]
	return js_runtime::exists(&path.to_string_lossy()).xok();
	#[cfg(not(target_arch = "wasm32"))]
	match fs::exists(path) {
		Ok(val) => Ok(val),
		Err(err) => Err(FsError::io(path, err)),
	}
}

/// Async version of [`exists`].
pub async fn exists_async(path: impl AsRef<Path>) -> FsResult<bool> {
	#[cfg(not(all(feature = "fs", not(target_arch = "wasm32"))))]
	{
		fs_ext::exists(path)
	}
	#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
	{
		let path = path.as_ref();
		match async_fs::metadata(path).await {
			Ok(_) => Ok(true),
			Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
			Err(err) => Err(FsError::io(path, err)),
		}
	}
}


/// Creates a directory and all parent directories if they don't exist.
pub fn create_dir_all(path: impl AsRef<Path>) -> FsResult<()> {
	let path = path.as_ref();
	#[cfg(target_arch = "wasm32")]
	return js_runtime::create_dir_all(&path.to_string_lossy()).xok();
	#[cfg(not(target_arch = "wasm32"))]
	return fs::create_dir_all(path).map_err(|err| FsError::io(path, err));
}

/// Async version of [`create_dir_all`].
pub async fn create_dir_all_async(path: impl AsRef<Path>) -> FsResult<()> {
	#[cfg(not(all(feature = "fs", not(target_arch = "wasm32"))))]
	{
		fs_ext::create_dir_all(path)
	}
	#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
	{
		let path = path.as_ref();
		async_fs::create_dir_all(path)
			.await
			.map_err(|err| FsError::io(path, err))
	}
}

/// recursively remove a file or directory
pub fn remove(path: impl AsRef<Path>) -> FsResult {
	let path = path.as_ref();
	match fs::metadata(path) {
		Ok(meta) => {
			if meta.is_dir() {
				fs::remove_dir_all(path)
					.map_err(|err| FsError::io(path, err))?;
			} else {
				fs::remove_file(path).map_err(|err| FsError::io(path, err))?;
			}
			Ok(())
		}
		Err(err) => Err(FsError::io(path, err)),
	}
}

/// Async version of [`remove`].
pub async fn remove_async(path: impl AsRef<Path>) -> FsResult {
	#[cfg(not(all(feature = "fs", not(target_arch = "wasm32"))))]
	{
		fs_ext::remove(path)
	}
	#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
	{
		let path = path.as_ref();
		match async_fs::metadata(path).await {
			Ok(meta) => {
				if meta.is_dir() {
					async_fs::remove_dir_all(path)
						.await
						.map_err(|err| FsError::io(path, err))?;
				} else {
					async_fs::remove_file(path)
						.await
						.map_err(|err| FsError::io(path, err))?;
				}
				Ok(())
			}
			Err(err) => Err(FsError::io(path, err)),
		}
	}
}

/// 1. tries to get the `WORKSPACE_ROOT` env var.
/// 2. if wasm, returns an empty path
/// 3. Otherwise return the closest ancestor (inclusive) that contains a `Cargo.lock` file
/// 4. Otherwise returns cwd
///
/// ## Panics
/// - The current directory is not found
/// - Insufficient permissions to access the current directory
pub fn workspace_root() -> PathBuf { crate::prelude::workspace_root() }

/// Reads a file as bytes.
pub fn read(path: impl AsRef<Path>) -> FsResult<Vec<u8>> {
	#[cfg(target_arch = "wasm32")]
	return js_runtime::read_file(&path.as_ref().to_string_lossy())
		.ok_or_else(|| FsError::file_not_found(path.as_ref()));
	#[cfg(not(target_arch = "wasm32"))]
	return std::fs::read(&path).map_err(|e| FsError::io(path, e));
}
/// Async version of [`read`].
pub async fn read_async(path: impl AsRef<Path>) -> FsResult<Vec<u8>> {
	#[cfg(not(all(feature = "fs", not(target_arch = "wasm32"))))]
	{
		fs_ext::read(path)
	}
	#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
	{
		async_fs::read(&path)
			.await
			.map_err(|e| FsError::io(path, e))
	}
}

/// Reads a file as a UTF-8 string.
pub fn read_to_string(path: impl AsRef<Path>) -> FsResult<String> {
	fs_ext::read(path.as_ref()).and_then(|bytes| {
		String::from_utf8(bytes).map_err(|e| FsError::other(path.as_ref(), e))
	})
}
/// Async version of [`read_to_string`].
pub async fn read_to_string_async(path: impl AsRef<Path>) -> FsResult<String> {
	#[cfg(not(all(feature = "fs", not(target_arch = "wasm32"))))]
	{
		fs_ext::read_to_string(path)
	}
	#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
	{
		async_fs::read_to_string(&path)
			.await
			.map_err(|e| FsError::io(path, e))
	}
}

/// Stream a file as byte chunks.
///
/// On native with the `fs` feature, reads asynchronously in 64 KiB
/// chunks via [`async_fs::File`]. Otherwise falls back to reading the
/// entire file synchronously and yielding a single chunk.
pub fn read_stream(
	path: impl AsRef<Path>,
) -> FsResult<
	Pin<Box<dyn Stream<Item = Result<Vec<u8>, BevyError>> + Send + Sync>>,
> {
	let path = path.as_ref().to_path_buf();

	#[cfg(not(all(feature = "fs", not(target_arch = "wasm32"))))]
	{
		let bytes = fs_ext::read(&path)?;
		Ok(Box::pin(futures_lite::stream::once(Ok(bytes))))
	}
	#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
	{
		use futures_lite::io::AsyncReadExt;
		// 64 KiB — matches the Linux kernel's default pipe/socket buffer size, ensuring a full
		// buffer drain in a single syscall. Also a round multiple of the 4 KiB page size (16 pages),
		// and comfortably fits in L2 cache. A reliable general-purpose default for async I/O.
		const CHUNK_SIZE: usize = 64 * 1024;
		// Validate path exists synchronously to return an early error.
		if !std::fs::exists(&path).unwrap_or(false) {
			return Err(FsError::file_not_found(&path));
		}
		let stream =
			futures_lite::stream::unfold(None::<async_fs::File>, move |file| {
				let path = path.clone();
				async move {
					let mut file = match file {
						Some(file) => file,
						None => match async_fs::File::open(&path).await {
							Ok(file) => file,
							Err(err) => {
								return Some((
									Err(BevyError::from(FsError::io(
										&path, err,
									))),
									None,
								));
							}
						},
					};
					let mut buf = vec![0u8; CHUNK_SIZE];
					match file.read(&mut buf).await {
						Ok(0) => None,
						Ok(num) => {
							buf.truncate(num);
							Some((Ok(buf), Some(file)))
						}
						Err(err) => Some((
							Err(BevyError::from(FsError::io(&path, err))),
							None,
						)),
					}
				}
			});
		Ok(Box::pin(stream))
	}
}

/// Stream a file as UTF-8 string chunks via [`TextStream`].
///
/// Internally calls [`read_stream`] and pipes bytes through
/// [`stream_ext::bytes_to_text`] for cross-boundary UTF-8 decoding.
pub fn read_stream_string(
	path: impl AsRef<Path>,
) -> FsResult<crate::utils::TextStream> {
	let byte_stream = fs_ext::read_stream(path)?;
	crate::utils::stream_ext::bytes_to_text(byte_stream).xok()
}


/// Computes a hash of a file's contents.
pub fn hash_file(path: impl AsRef<Path>) -> FsResult<u64> {
	let bytes = fs_ext::read(path)?;
	let hash = fs_ext::hash_bytes(&bytes);
	Ok(hash)
}

/// Computes a hash of a byte slice.
pub fn hash_bytes(bytes: &[u8]) -> u64 {
	let mut hasher = std::hash::DefaultHasher::new();
	use std::hash::Hash;
	use std::hash::Hasher;
	bytes.hash(&mut hasher);
	hasher.finish()
}
/// Computes a hash of a string.
pub fn hash_string(str: &str) -> u64 {
	let bytes = str.as_bytes();
	fs_ext::hash_bytes(bytes)
}

#[extend::ext]
impl ExitStatus {
	fn xresult(&self) -> bevy::prelude::Result<()> {
		if self.success() {
			Ok(())
		} else {
			bevybail!("Process exited with non-zero status: {}", self)
		}
	}
}

/// Run a 'touch' command for the provided path
pub fn touch(path: impl AsRef<Path>) -> bevy::prelude::Result {
	std::process::Command::new("touch")
		.arg(path.as_ref())
		.status()?
		.xresult()?
		.xok()
}

/// Write a file, ensuring the path exists
pub fn write(path: impl AsRef<Path>, data: impl AsRef<[u8]>) -> FsResult {
	let path = path.as_ref();
	if let Some(parent) = path.parent() {
		fs_ext::create_dir_all(parent)?;
	}
	#[cfg(target_arch = "wasm32")]
	{
		js_runtime::write_file(&path.to_string_lossy(), data.as_ref());
		Ok(())
	}
	#[cfg(not(target_arch = "wasm32"))]
	{
		fs::write(path, data).map_err(|err| FsError::io(path, err))?;
		Ok(())
	}
}

/// Async version of write: Write a file, ensuring the path exists.
/// Falls back to `fs_ex::write` without the feature flag
pub async fn write_async(
	path: impl AsRef<Path>,
	data: impl AsRef<[u8]>,
) -> FsResult {
	#[cfg(not(all(feature = "fs", not(target_arch = "wasm32"))))]
	{
		fs_ext::write(path, data)
	}
	#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
	{
		let path = path.as_ref();
		if let Some(parent) = path.parent() {
			fs_ext::create_dir_all_async(parent).await?;
		}
		async_fs::write(path, data)
			.await
			.map_err(|err| FsError::io(path, err))?;
		Ok(())
	}
}

/// Write a file only if the data is different from the existing file.
/// If the file does not exist it will be created.
pub fn write_if_diff(
	path: impl AsRef<Path>,
	data: impl AsRef<[u8]>,
) -> FsResult {
	let path = path.as_ref();
	match fs::read(path) {
		Ok(existing_data) if existing_data == data.as_ref() => {
			return Ok(());
		}
		_ => {
			fs_ext::write(path, data)?;
		}
	}
	Ok(())
}

#[cfg(test)]
/// Returns the path to a directory containing some files for testing.
pub fn test_dir() -> PathBuf {
	fs_ext::workspace_root().join(Path::new("tests/test_dir"))
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[test]
	fn workspace_root() {
		fs_ext::workspace_root()
			.file_stem()
			.unwrap()
			.to_str()
			.unwrap()
			.xpect_eq("beet");
		fs_ext::workspace_root()
			.join("Cargo.lock")
			.xmap(fs_ext::exists)
			.unwrap()
			.xpect_true();
	}

	#[test]
	fn to_string() {
		let content =
			fs_ext::read_to_string(fs_ext::test_dir().join("mod.rs")).unwrap();
		content.contains("pub mod included_dir;").xpect_true();

		fs_ext::read_to_string(fs_ext::test_dir().join("foo.rs")).xpect_err();
	}

	#[test]
	fn to_bytes() {
		let bytes = fs_ext::read(fs_ext::test_dir().join("mod.rs")).unwrap();
		bytes.len().xpect_greater_than(10);

		fs_ext::read(fs_ext::test_dir().join("foo.rs")).xpect_err();
	}

	#[test]
	fn hash() {
		let hash1 =
			fs_ext::hash_file(fs_ext::test_dir().join("mod.rs")).unwrap();
		let hash2 =
			fs_ext::hash_file(fs_ext::test_dir().join("included_file.rs"))
				.unwrap();
		hash1.xpect_not_eq(hash2);

		let str =
			fs_ext::read_to_string(fs_ext::test_dir().join("mod.rs")).unwrap();
		let hash3 = fs_ext::hash_string(&str);
		hash3.xpect_eq(hash1);
	}

	#[crate::test]
	async fn read_stream_matches_read() {
		let path = fs_ext::test_dir().join("mod.rs");
		let expected = fs_ext::read(&path).unwrap();

		let stream = fs_ext::read_stream(&path).unwrap();
		futures_lite::pin!(stream);

		let mut collected = Vec::new();
		while let Some(chunk) = stream.next().await {
			collected.extend_from_slice(&chunk.unwrap());
		}
		collected.xpect_eq(expected);
	}

	#[crate::test]
	async fn read_stream_string_matches_read_to_string() {
		let path = fs_ext::test_dir().join("mod.rs");
		let expected = fs_ext::read_to_string(&path).unwrap();

		let mut stream = fs_ext::read_stream_string(&path).unwrap();

		let mut collected = String::new();
		while let Some(chunk) = stream.next().await {
			collected.push_str(&chunk.unwrap());
		}
		collected.xpect_eq(expected);
	}

	#[crate::test]
	async fn read_stream_missing_file() {
		fs_ext::read_stream("nonexistent_file.txt")
			.is_err()
			.xpect_true();
	}

	#[crate::test]
	async fn read_stream_string_missing_file() {
		fs_ext::read_stream_string("nonexistent_file.txt")
			.is_err()
			.xpect_true();
	}
}
