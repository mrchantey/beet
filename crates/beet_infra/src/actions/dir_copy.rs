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
#[derive(Debug, Clone, Default, Get, SetWith, Component, Reflect)]
#[reflect(Component, Default)]
#[require(DirCopyAction)]
pub struct DirCopy {
	/// The workspace-relative source directory.
	src: SmolPath,
	/// The workspace-relative destination directory.
	dest: SmolPath,
	/// Comma-separated paths to copy, each relative to both ends and naming
	/// either a file or a directory.
	paths: SmolStr,
}

impl DirCopy {
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
	pub fn iter_paths(&self) -> impl Iterator<Item = &str> {
		self.paths
			.split(',')
			.map(str::trim)
			.filter(|path| !path.is_empty())
	}
}

/// Copy each declared path from `src` to `dest`, replacing what is there.
#[action]
#[derive(Default, Component)]
pub async fn DirCopyAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let copy = cx.caller.get_cloned::<DirCopy>().await?;
	let src = WsPathBuf::new(copy.src().to_string()).into_abs();
	let dest = WsPathBuf::new(copy.dest().to_string()).into_abs();
	let mut copied = 0;
	for path in copy.iter_paths() {
		let from = src.join(path);
		let to = dest.join(path);
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
	info!(
		"copied {copied} borrowed path(s) {} -> {}",
		copy.src(),
		copy.dest()
	);
	Pass(cx.input).xok()
}
