use std::path::PathBuf;
use std::str::FromStr;

/// 1. tries to get the `SWEET_ROOT` env var.
/// 2. if wasm, returns an empty path
/// 3. Otherwise return the closest ancestor (inclusive) that contains a `Cargo.lock` file
/// 4. Otherwise returns cwd
///
/// ## Panics
/// - The current directory is not found
/// - Insufficient permissions to access the current directory
pub fn workspace_root() -> PathBuf {
	if let Ok(root_str) = std::env::var("SWEET_ROOT") {
		return PathBuf::from_str(&root_str).unwrap();
	}

	#[cfg(target_arch = "wasm32")]
	{
		return PathBuf::default();
	}
	#[cfg(not(target_arch = "wasm32"))]
	{
		use std::ffi::OsString;

		let path = std::env::current_dir().unwrap();
		let mut path_ancestors = path.as_path().ancestors();
		while let Some(p) = path_ancestors.next() {
			if std::fs::read_dir(p).unwrap().any(|p| {
				p.map(|p| p.file_name() == OsString::from("Cargo.lock"))
					.unwrap_or(false)
			}) {
				use std::path::PathBuf;
				return PathBuf::from(p);
			}
		}
		// If no workspace root is found, fall back to the current directory
		return path;
	}
}
