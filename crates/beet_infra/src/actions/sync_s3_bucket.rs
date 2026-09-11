use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Syncs the nearest ancestor [`S3FsStore`]'s two ends, in either direction.
///
/// The defaults are the conservative ones: push, additive. `delete` opts into a
/// *mirror*, where objects absent from the source are pruned so the destination
/// exactly reflects it. A deploy wants that (a renamed or removed source file
/// otherwise lingers across deploys, eg a home route converted `index.bsx` ->
/// `index.md` leaves the stale `index.bsx`, so the served binary sees two routes
/// for `/` and panics on boot), a source of record does not.
#[action]
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn SyncS3Bucket(
	/// Which end is the source.
	#[field]
	direction: SyncDirection,
	/// Prune destination entries absent from the source (a mirror rather than an
	/// additive sync). Guarded on push by [`SyncS3Bucket::assert_mirrorable`].
	#[field]
	delete: bool,
	/// Upload the targets of symbolic links rather than skipping them, so a
	/// symlinked subdir is materialized into the bucket.
	#[field]
	follow_symlinks: bool,
	/// Sync without credentials, for a public-read bucket.
	#[field]
	no_sign_request: bool,
	/// Optional subdir of the bucket to sync against; the bucket root by default.
	#[field]
	bucket_dir: Option<SmolPath>,
	/// Comma-separated paths under the local dir to sync, each naming a file or
	/// a directory; empty (the default) syncs the whole dir.
	///
	/// An ALLOWLIST, not a set of exclusions: a site whose entry sits at a repo
	/// root shares that dir with everything else in the checkout (crates,
	/// content, `target/`), so it names what IS the site and nothing else can
	/// leak into the bucket by appearing beside it. The filters apply to both
	/// ends, so a mirror still prunes only within the named paths.
	#[field]
	paths: SmolStr,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	trace!("SyncS3Bucket: starting");
	// the two ends of the sync, plus the per-deploy prefix a `<DirSync>` bucket
	// publishes under. That prefix is resolved HERE rather than at declaration,
	// because which version a sync belongs to is the verb's answer and not the
	// declaration's: see `dir_sync::deploy_subdir`.
	let (s3_fs_store, deploy_subdir) = cx
		.caller
		.with_state::<(
			AncestorQuery<&S3FsStore>,
			AncestorQuery<&DirSync>,
			StackQuery,
			Query<(&ErasedBlock, &ErasedStoreBlock)>,
		), _>(|entity, (stores, syncs, stacks, declared)| -> Result<_> {
			let store = stores.get(entity)?.clone();
			// a store spawned directly (rather than through `<DirSync>`) already
			// carries whatever root it means to publish into
			let subdir = match syncs.get(entity) {
				Ok(sync) => crate::actions::deploy_subdir(
					entity, sync, &stacks, &declared,
				)?,
				Err(_) => None,
			};
			(store, subdir).xok()
		})
		.await??;
	let s3_store = match deploy_subdir {
		Some(subdir) => s3_fs_store.s3_store().clone().with_subdir(subdir),
		None => s3_fs_store.s3_store().clone(),
	};
	// `s3_uri` already ends in a separator
	let s3_uri = match &bucket_dir {
		Some(dir) => format!("{}{dir}", s3_store.s3_uri()),
		None => s3_store.s3_uri(),
	};
	let local_dir = s3_fs_store.fs_store().effective_root();
	// only a mirroring push can destroy remote state, so only it is guarded
	if delete && direction == SyncDirection::Push {
		SyncS3Bucket::assert_mirrorable(&local_dir, follow_symlinks)?;
	}
	let sync = match direction {
		SyncDirection::Push => S3Sync::push(local_dir.clone(), &s3_uri),
		SyncDirection::Pull => S3Sync::pull(&s3_uri, local_dir.clone()),
	}
	.filters(SyncS3Bucket::filters(&paths));
	trace!(
		"SyncS3Bucket: syncing {} {} {s3_uri}",
		local_dir.display(),
		match direction {
			SyncDirection::Push => "->",
			SyncDirection::Pull => "<-",
		}
	);
	sync.delete(delete)
		.follow_symlinks(follow_symlinks)
		.no_sign_request(no_sign_request)
		.send()
		.await?;
	trace!("synced {s3_uri} (region: {:?})", s3_store.region());
	trace!("SyncS3Bucket: complete");
	Pass(cx.input).xok()
}

impl SyncS3Bucket {
	/// A `paths` allowlist as sync filters: exclude everything, then include each
	/// declared path both as a file and as a directory prefix. Empty when nothing
	/// is declared, so the whole dir syncs.
	pub fn filters(paths: &str) -> Vec<S3Filter> {
		let paths = paths
			.split(',')
			.map(str::trim)
			.filter(|path| !path.is_empty())
			.collect::<Vec<_>>();
		if paths.is_empty() {
			return Vec::new();
		}
		core::iter::once(S3Filter::Exclude("*".into()))
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
	/// Only the push direction destroys remote state, so only push calls this; a
	/// pull with `delete` overwrites a local dir the caller asked for.
	pub fn assert_mirrorable(
		local_dir: &AbsPathBuf,
		follow_symlinks: bool,
	) -> Result {
		if fs_ext::is_dir_empty(local_dir)? {
			bevybail!(
				"refusing to mirror an empty local dir into a bucket: {}\nhydrate it first, eg `just beet-shared pull`",
				local_dir.display()
			);
		}
		if !follow_symlinks {
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

#[cfg(test)]
mod test {
	use super::*;

	/// A mirroring push refuses an empty source dir, the shape that would empty
	/// the bucket.
	#[beet_core::test]
	fn mirror_guard_rejects_an_empty_dir() {
		let dir = TempDir::new().unwrap();
		let root = (*dir).clone();
		SyncS3Bucket::assert_mirrorable(&root, false).unwrap_err();
		fs_ext::write(root.join("index.html"), "<div/>").unwrap();
		SyncS3Bucket::assert_mirrorable(&root, false).unwrap();
	}

	/// An empty `paths` syncs the whole dir; a declared one becomes an allowlist,
	/// excluding everything before including each path as both a file and a
	/// directory prefix.
	#[beet_core::test]
	fn paths_render_an_allowlist() {
		SyncS3Bucket::filters("").xpect_eq(Vec::new());
		SyncS3Bucket::filters("main.bsx, routes,").xpect_eq(vec![
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

		SyncS3Bucket::assert_mirrorable(&root, false).unwrap();
		SyncS3Bucket::assert_mirrorable(&root, true).unwrap_err();
		fs_ext::write(target.join("logo.png"), "png").unwrap();
		SyncS3Bucket::assert_mirrorable(&root, true).unwrap();
	}
}
