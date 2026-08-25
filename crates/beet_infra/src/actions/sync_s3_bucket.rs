use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Syncs the nearest ancestor [`S3FsStore`]'s two ends, in either direction.
/// Read by [`SyncS3BucketAction`], which does the work.
///
/// The defaults are the conservative ones: push, additive. `delete` opts into a
/// *mirror*, where objects absent from the source are pruned so the destination
/// exactly reflects it. A deploy wants that (a renamed or removed source file
/// otherwise lingers across deploys, eg a home route converted `index.bsx` ->
/// `index.md` leaves the stale `index.bsx`, so the served binary sees two routes
/// for `/` and panics on boot), a source of record does not.
#[derive(Debug, Clone, Default, Get, SetWith, Component, Reflect)]
#[reflect(Component, Default)]
#[require(SyncS3BucketAction)]
pub struct SyncS3Bucket {
	/// Which end is the source.
	direction: SyncDirection,
	/// Prune destination entries absent from the source (a mirror rather than an
	/// additive sync). Guarded on push by [`SyncS3Bucket::assert_mirrorable`].
	delete: bool,
	/// Upload the targets of symbolic links rather than skipping them, so a
	/// symlinked subdir is materialized into the bucket.
	follow_symlinks: bool,
	/// Sync without credentials, for a public-read bucket.
	no_sign_request: bool,
	/// Optional subdir of the bucket to sync against; the bucket root by default.
	#[set_with(unwrap_option)]
	bucket_dir: Option<SmolPath>,
	/// Comma-separated paths under the local dir to sync, each naming a file or
	/// a directory; empty (the default) syncs the whole dir.
	///
	/// An ALLOWLIST, not a set of exclusions: a site whose entry sits at a repo
	/// root shares that dir with everything else in the checkout (crates,
	/// content, `target/`), so it names what IS the site and nothing else can
	/// leak into the bucket by appearing beside it. The filters apply to both
	/// ends, so a mirror still prunes only within the named paths.
	paths: SmolStr,
}

impl SyncS3Bucket {
	/// The [`paths`](Self::paths) allowlist as sync filters: exclude everything,
	/// then include each declared path both as a file and as a directory prefix.
	/// Empty when nothing is declared, so the whole dir syncs.
	fn filters(&self) -> Vec<S3Filter> {
		let paths = self
			.paths
			.split(',')
			.map(str::trim)
			.filter(|path| !path.is_empty())
			.collect::<Vec<_>>();
		if paths.is_empty() {
			return Vec::new();
		}
		std::iter::once(S3Filter::Exclude("*".into()))
			.chain(paths.into_iter().flat_map(|path| {
				[
					S3Filter::Include(path.into()),
					S3Filter::Include(format!("{path}/*")),
				]
			}))
			.collect()
	}

	/// Refuse to mirror a local dir that would empty the bucket: a missing or
	/// empty source, or (when following symlinks) a symlinked child dir whose
	/// target is missing or empty — the signature of an unhydrated checkout.
	///
	/// Only the push direction destroys remote state, so only push is guarded; a
	/// pull with `delete` overwrites a local dir the caller asked for.
	fn assert_mirrorable(&self, local_dir: &AbsPathBuf) -> Result {
		if !self.delete || self.direction != SyncDirection::Push {
			return Ok(());
		}
		if fs_ext::is_dir_empty(local_dir)? {
			bevybail!(
				"refusing to mirror an empty local dir into a bucket: {}\nhydrate it first, eg `just beet-shared pull`",
				local_dir.display()
			);
		}
		if !self.follow_symlinks {
			return Ok(());
		}
		// a linked child dir is materialized into the bucket by this push, so an
		// unhydrated link is the same emptying hazard one level down.
		for child in ReadDir::all(local_dir)?
			.into_iter()
			.filter(|child| fs_ext::is_symlink(child))
		{
			if fs_ext::is_dir_empty(&child)? {
				bevybail!(
					"refusing to mirror an empty symlinked dir into a bucket: {}\nhydrate it first, eg `just beet-shared pull`",
					child.display()
				);
			}
		}
		Ok(())
	}
}

/// Sync the nearest ancestor [`S3FsStore`]'s local dir and bucket, per the
/// [`SyncS3Bucket`] settings on this entity (its defaults when absent).
#[action]
#[derive(Default, Component)]
pub async fn SyncS3BucketAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	trace!("SyncS3BucketAction: starting");
	let s3_fs_store = cx
		.caller
		.with_state::<AncestorQuery<&S3FsStore>, _>(|entity, query| {
			query.get(entity).cloned()
		})
		.await??;
	let opts = cx
		.caller
		.get_cloned::<SyncS3Bucket>()
		.await
		.unwrap_or_default();
	let s3_uri = match opts.bucket_dir() {
		Some(dir) => format!("{}/{dir}", s3_fs_store.s3_store().s3_uri()),
		None => s3_fs_store.s3_store().s3_uri(),
	};
	let local_dir = s3_fs_store.fs_store().effective_root();
	opts.assert_mirrorable(&local_dir)?;
	let sync = match opts.direction() {
		SyncDirection::Push => S3Sync::push(local_dir.clone(), &s3_uri),
		SyncDirection::Pull => S3Sync::pull(&s3_uri, local_dir.clone()),
	}
	.filters(opts.filters());
	trace!(
		"SyncS3BucketAction: syncing {} {} {s3_uri}",
		local_dir.display(),
		match opts.direction() {
			SyncDirection::Push => "->",
			SyncDirection::Pull => "<-",
		}
	);
	sync.delete(opts.delete())
		.follow_symlinks(opts.follow_symlinks())
		.no_sign_request(opts.no_sign_request())
		.send()
		.await?;
	trace!(
		"synced {s3_uri} (region: {:?})",
		s3_fs_store.s3_store().region()
	);
	trace!("SyncS3BucketAction: complete");
	Pass(cx.input).xok()
}

#[cfg(test)]
mod test {
	use super::*;

	/// A mirroring push refuses an empty source dir, the shape that would empty
	/// the bucket; an additive push (the default) makes no such demand, since it
	/// can only add.
	#[beet_core::test]
	fn mirror_guard_rejects_an_empty_dir() {
		let dir = TempDir::new().unwrap();
		let root = (*dir).clone();
		SyncS3Bucket::default().assert_mirrorable(&root).unwrap();
		let mirror = SyncS3Bucket::default().with_delete(true);
		mirror.assert_mirrorable(&root).unwrap_err();
		fs_ext::write(root.join("index.html"), "<div/>").unwrap();
		mirror.assert_mirrorable(&root).unwrap();
	}

	/// An empty `paths` syncs the whole dir; a declared one becomes an allowlist,
	/// excluding everything before including each path as both a file and a
	/// directory prefix.
	#[beet_core::test]
	fn paths_render_an_allowlist() {
		SyncS3Bucket::default().filters().xpect_eq(Vec::new());
		SyncS3Bucket::default()
			.with_paths("main.bsx, routes,")
			.filters()
			.xpect_eq(vec![
				S3Filter::Exclude("*".into()),
				S3Filter::Include("main.bsx".into()),
				S3Filter::Include("main.bsx/*".into()),
				S3Filter::Include("routes".into()),
				S3Filter::Include("routes/*".into()),
			]);
	}

	/// A symlinked child dir is materialized into the bucket by a
	/// `follow_symlinks` mirror, so an unhydrated link is the same hazard one
	/// level down — and only when links are followed.
	#[cfg(unix)]
	#[beet_core::test]
	fn mirror_guard_rejects_an_unhydrated_symlink() {
		let dir = TempDir::new().unwrap();
		let root = (*dir).clone();
		fs_ext::write(root.join("index.html"), "<div/>").unwrap();
		let target = root.join("target");
		fs_ext::create_dir_all(&target).unwrap();
		std::os::unix::fs::symlink(&target, root.join("assets")).unwrap();

		let mirror = SyncS3Bucket::default().with_delete(true);
		mirror.assert_mirrorable(&root).unwrap();
		let mirror = mirror.with_follow_symlinks(true);
		mirror.assert_mirrorable(&root).unwrap_err();
		fs_ext::write(target.join("logo.png"), "png").unwrap();
		mirror.assert_mirrorable(&root).unwrap();
	}
}
