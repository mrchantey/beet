use crate::prelude::*;
use std::path::PathBuf;

/// 1. tries to get the `WORKSPACE_ROOT` env var.
/// 2. if wasm, returns an empty path
/// 3. Otherwise return the closest ancestor (inclusive) that contains a `Cargo.lock` file
/// 4. Otherwise returns cwd
///
/// ## Panics
/// - The current directory is not found
/// - Insufficient permissions to access the current directory
/// - In wasm and js_runtime::WORKSPACE_ROOT returns None
pub fn workspace_root() -> PathBuf {
	if let Ok(root_str) = env_ext::var("WORKSPACE_ROOT") {
		return root_str.into();
	}
	#[cfg(target_arch = "wasm32")]
	{
		panic!("no WORKSPACE_ROOT env in js runtime");
	}
	#[cfg(not(target_arch = "wasm32"))]
	{
		use std::ffi::OsString;
		let path = fs_ext::current_dir().unwrap();
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
