//! Local filesystem request client with security controls.
//!
//! [`FileClient`] reads files from the local filesystem and returns
//! them as [`Response`] with the content type inferred via
//! [`MediaType::from_path`]. Access is gated by configurable security
//! policies that prevent directory traversal and dot-file access by
//! default.

use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;
use path_clean::PathClean;
use std::path::Path;
use std::path::PathBuf;

/// A client that resolves requests to local filesystem paths.
///
/// Security defaults are conservative — external paths and dot-files
/// are disallowed unless explicitly opted in.
///
/// ```no_run
/// # use beet_net::prelude::*;
/// # use beet_core::prelude::*;
/// # async fn run() -> Result<Response> {
/// let client = FileClient::default();
/// let response = client.send("index.html").await?;
/// # Ok(response)
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct FileClient {
	/// Allow access to paths outside of the cwd.
	pub external_file_paths: bool,
	/// Allow access to files and directories beginning with `.`,
	/// ie `.env`.
	pub dot_files: bool,
	/// Whether to stream file contents instead of reading all at once.
	pub streaming: bool,
}

impl Default for FileClient {
	fn default() -> Self {
		Self {
			external_file_paths: false,
			dot_files: false,
			streaming: false,
		}
	}
}

impl FileClient {
	/// Create a new [`FileClient`] with default security settings.
	pub fn new() -> Self { Self::default() }

	/// Allow access to paths outside of the cwd.
	pub fn with_external_file_paths(mut self, allowed: bool) -> Self {
		self.external_file_paths = allowed;
		self
	}

	/// Allow access to dot-files (files or directories starting with `.`).
	pub fn with_dot_files(mut self, allowed: bool) -> Self {
		self.dot_files = allowed;
		self
	}

	/// Enable streaming mode for file reads.
	pub fn with_streaming(mut self, streaming: bool) -> Self {
		self.streaming = streaming;
		self
	}

	/// Send a file request, reading from the local filesystem.
	///
	/// The `path` is resolved relative to the current working directory
	/// unless [`external_file_paths`](FileClient::external_file_paths)
	/// is enabled.
	pub async fn send(&self, path: impl AsRef<str>) -> Result<Response> {
		let raw = path.as_ref();
		let file_path = self.resolve_path(raw)?;
		self.validate_path(&file_path)?;

		let media_type = MediaType::from_path(&file_path);

		let bytes = fs_ext::read_async(&file_path)
			.await
			.map_err(|err| bevyhow!("Failed to read file {raw}: {err}"))?;

		if self.streaming {
			let chunk = Bytes::from(bytes);
			let stream = futures::stream::once(async move { Ok(chunk) });
			let body = Body::stream(stream);
			Response::ok_body(body, media_type).xok()
		} else {
			Response::ok_body(bytes, media_type).xok()
		}
	}

	/// Resolve a raw path string into a cleaned [`PathBuf`].
	///
	/// - Strips a leading `file://` scheme if present.
	/// - Cleans the path to resolve `..` and `.` components.
	/// - If the path is relative, joins it to the cwd.
	fn resolve_path(&self, raw: &str) -> Result<PathBuf> {
		let stripped = raw.strip_prefix("file://").unwrap_or(raw);

		let path = PathBuf::from(stripped).clean();

		if path.is_absolute() {
			path.xok()
		} else {
			let cwd = fs_ext::current_dir()
				.map_err(|err| bevyhow!("Failed to get cwd: {err}"))?;
			cwd.join(&path).clean().xok()
		}
	}

	/// Validate the resolved path against the security policy.
	fn validate_path(&self, path: &Path) -> Result {
		// Dot-file check: reject any component starting with `.`
		if !self.dot_files {
			for component in path.components() {
				if let std::path::Component::Normal(segment) = component {
					if segment.to_str().is_some_and(|seg| seg.starts_with('.'))
					{
						bevybail!(
							"Access to dot-files is not allowed: {}",
							path.display()
						);
					}
				}
			}
		}

		// External path check: must be under cwd
		if !self.external_file_paths {
			let cwd = fs_ext::current_dir()
				.map_err(|err| bevyhow!("Failed to get cwd: {err}"))?;
			if !path.starts_with(&cwd) {
				bevybail!(
					"Access to paths outside cwd is not allowed: {}",
					path.display()
				);
			}
		}

		Ok(())
	}
}


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn resolve_strips_file_scheme() {
		let client = FileClient::new().with_external_file_paths(true);
		let path = client.resolve_path("file:///home/user/doc.txt").unwrap();
		path.xpect_eq(PathBuf::from("/home/user/doc.txt"));
	}

	#[test]
	fn resolve_cleans_traversal() {
		let client = FileClient::new().with_external_file_paths(true);
		let path = client.resolve_path("/home/user/../user/doc.txt").unwrap();
		path.xpect_eq(PathBuf::from("/home/user/doc.txt"));
	}

	#[test]
	fn validate_rejects_dot_files() {
		let client = FileClient::new().with_external_file_paths(true);
		client
			.validate_path(Path::new("/home/user/.env"))
			.xpect_err();
		client
			.validate_path(Path::new("/home/.config/foo"))
			.xpect_err();
	}

	#[test]
	fn validate_allows_dot_files_when_enabled() {
		let client = FileClient::new()
			.with_external_file_paths(true)
			.with_dot_files(true);
		client.validate_path(Path::new("/home/user/.env")).unwrap();
	}

	#[test]
	fn validate_rejects_external_paths() {
		let client = FileClient::new();
		// An absolute path outside the cwd should fail
		client.validate_path(Path::new("/etc/passwd")).xpect_err();
	}

	#[test]
	fn validate_allows_external_when_enabled() {
		let client = FileClient::new()
			.with_external_file_paths(true)
			.with_dot_files(true);
		client.validate_path(Path::new("/etc/hosts")).unwrap();
	}

	#[beet_core::test]
	async fn send_reads_file() {
		let cwd = fs_ext::current_dir().unwrap();
		let cargo_toml = cwd.join("Cargo.toml");
		// Only run if we're in a directory with a Cargo.toml
		if !cargo_toml.exists() {
			return;
		}
		let client = FileClient::new();
		let response = client.send("Cargo.toml").await.unwrap();
		response.status().xpect_eq(StatusCode::OK);
	}

	#[beet_core::test]
	async fn send_streaming_reads_file() {
		let cwd = fs_ext::current_dir().unwrap();
		let cargo_toml = cwd.join("Cargo.toml");
		if !cargo_toml.exists() {
			return;
		}
		let client = FileClient::new().with_streaming(true);
		let response = client.send("Cargo.toml").await.unwrap();
		response.status().xpect_eq(StatusCode::OK);
	}

	#[beet_core::test]
	async fn send_missing_file_errors() {
		let client = FileClient::new();
		client
			.send("definitely_nonexistent_file_12345.txt")
			.await
			.xpect_err();
	}

	#[beet_core::test]
	async fn send_dot_file_rejected() {
		let client = FileClient::new();
		client.send(".env").await.xpect_err();
	}

	#[test]
	fn media_type_inferred() {
		let client = FileClient::new().with_external_file_paths(true);
		let path = client.resolve_path("style.css").unwrap();
		MediaType::from_path(&path).xpect_eq(MediaType::Css);
	}
}
