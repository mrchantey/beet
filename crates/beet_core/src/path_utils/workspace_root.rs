use crate::prelude::*;
use std::path::PathBuf;

/// 1. tries to get the `WORKSPACE_ROOT` env var.
/// 2. if wasm, returns an empty path (the store root is the ambient origin — a bucket
///    root on a Cloudflare Worker, the served page origin in a browser — so paths
///    resolve relative to an empty root; a js runtime with a real root sets `WORKSPACE_ROOT`).
/// 3. Otherwise return the closest ancestor (inclusive) that contains a `Cargo.lock` file
/// 4. Otherwise returns cwd
///
/// ## Panics
/// - The current directory is not found
/// - Insufficient permissions to access the current directory
pub fn workspace_root() -> PathBuf {
	if let Ok(root_str) = env_ext::var("WORKSPACE_ROOT") {
		return root_str.into();
	}
	cfg_if! {
		if #[cfg(target_arch = "wasm32")] {
			// no filesystem/workspace in a js runtime (browser, Cloudflare Worker): the
			// store root is the ambient origin, so paths resolve relative to an empty root.
			// A js runtime with a real root sets WORKSPACE_ROOT above; panicking here made
			// serving any site from a Worker impossible.
			return PathBuf::new();
		} else {
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
}
