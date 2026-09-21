//! Copy declared paths between two workspace directories.

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// `<DirCopy src="assets" dest="site/assets" paths="wasm/beet-full.wasm"/>` —
/// the single declaration of what one tree BORROWS from another.
///
/// Two directories can share files without either owning the other: the site's
/// assets are its own, but a few workspace-built artifacts (a wasm binary, a
/// geoip database) are produced in the workspace tree and served from the site.
/// This names exactly those. Under a [`RepoStage`] it borrows into the staging
/// dir the deploy publishes, so a rebuilt artifact cannot go stale on the far
/// side and nothing is written into the checkout; elsewhere (the local
/// `assets` verb) `dest` is workspace-relative.
///
/// Mirror semantics per path: a destination file or directory is replaced, so a
/// stale binary is overwritten rather than merged around.
#[action]
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn DirCopy(
	/// The workspace-relative source directory.
	#[field]
	src: WsPath,
	/// The destination directory: inside the staging dir under a
	/// [`RepoStage`] ancestor, else workspace-relative.
	#[field]
	dest: WsPath,
	/// Comma-separated paths to copy, each relative to both ends and naming
	/// either a file or a directory.
	#[field]
	paths: SmolStr,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let src_dir = src.into_abs();
	// under a stage the destination is the staging dir's, so the borrow lands
	// where the mirror reads rather than in the checkout
	let dest_dir = cx
		.caller
		.with_state::<(AncestorQuery<&RepoStage>, StackQuery), _>(
			move |entity, (stages, stacks)| -> Result<AbsPath> {
				match stages.get(entity) {
					Ok(_) => RepoStage::dir(&stacks, entity)?
						.join(dest.to_string())
						.xok(),
					Err(_) => dest.into_abs().xok(),
				}
			},
		)
		.await??;
	let copied = DirCopy::copy_paths(&src_dir, &dest_dir, &paths)?;
	info!("copied {copied} borrowed path(s) {src_dir} -> {dest_dir}");
	Pass(cx.input).xok()
}

impl DirCopy {
	/// Mirror every `paths` entry from `src_dir` into `dest_dir` (a file or a
	/// directory, replaced wholesale), answering how many; a declared path
	/// that is absent is an error, since the destination would silently keep
	/// a stale copy.
	pub fn copy_paths(
		src_dir: &AbsPath,
		dest_dir: &AbsPath,
		paths: &str,
	) -> Result<usize> {
		let mut copied = 0;
		for path in Self::iter_paths(paths) {
			let from = src_dir.join(path);
			let to = dest_dir.join(path);
			if !fs_ext::exists(&from)? {
				bevybail!(
					"nothing to copy at {}: the path is declared but absent, so the destination would silently keep a stale copy",
					from
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
			debug!("copied {} -> {}", from, to);
			copied += 1;
		}
		Ok(copied)
	}

	/// A copy of every `paths` entry from `src` to `dest`.
	pub fn new(
		src: impl Into<WsPath>,
		dest: impl Into<WsPath>,
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
