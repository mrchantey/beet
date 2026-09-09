//! Copy declared paths between two workspace directories.

use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// `<DirCopy src="assets" dest="site/assets" paths="wasm/beet-full.wasm"/>` —
/// the single declaration of what one tree BORROWS from another.
///
/// Two directories can share files without either owning the other: the site's
/// assets are its own, but a few workspace-built artifacts (a wasm binary, a
/// geoip database) are produced in the workspace tree and served from the site.
/// This names exactly those, and runs ahead of every publish so a rebuilt
/// artifact cannot go stale on the far side.
///
/// Mirror semantics per path: a destination file or directory is replaced, so a
/// stale binary is overwritten rather than merged around.
#[action]
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn DirCopy(
	/// The workspace-relative source directory.
	#[field]
	src: SmolPath,
	/// The workspace-relative destination directory.
	#[field]
	dest: SmolPath,
	/// Comma-separated paths to copy, each relative to both ends and naming
	/// either a file or a directory.
	#[field]
	paths: SmolStr,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let src_dir = WsPathBuf::new(src.to_string()).into_abs();
	let dest_dir = WsPathBuf::new(dest.to_string()).into_abs();
	let mut copied = 0;
	for path in DirCopy::iter_paths(&paths) {
		let from = src_dir.join(path);
		let to = dest_dir.join(path);
		if !fs_ext::exists(&from)? {
			bevybail!(
				"nothing to copy at {}: the borrowed path is declared but absent, so the destination would silently keep a stale copy",
				from.display()
			);
		}
		if let Some(parent) = to.parent() {
			fs_ext::create_dir_all(parent)?;
		}
		// mirror: whatever is there is replaced wholesale.
		fs_ext::remove(&to).ok();
		match fs_ext::is_dir(&from) {
			true => fs_ext::copy_recursive(&from, &to)?,
			false => {
				fs_ext::copy(&from, &to)?;
			}
		}
		debug!("copied {} -> {}", from.display(), to.display());
		copied += 1;
	}
	info!("copied {copied} borrowed path(s) {src} -> {dest}");
	Pass(cx.input).xok()
}

impl DirCopy {
	/// A copy of every `paths` entry from `src` to `dest`.
	pub fn new(
		src: impl Into<SmolPath>,
		dest: impl Into<SmolPath>,
		paths: impl Into<SmolStr>,
	) -> Self {
		Self {
			src: src.into(),
			dest: dest.into(),
			paths: paths.into(),
		}
	}

	/// The declared paths, skipping empty segments.
	pub fn iter_paths(paths: &str) -> impl Iterator<Item = &str> {
		paths
			.split(',')
			.map(str::trim)
			.filter(|path| !path.is_empty())
	}
}
