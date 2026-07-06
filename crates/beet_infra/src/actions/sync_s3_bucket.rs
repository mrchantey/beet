use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Mirror the local dir to the nearest ancestor [`S3FsStore`]'s bucket.
///
/// A *mirror*, not an append: bucket objects no longer present locally are pruned
/// (`aws s3 sync --delete`), so the bucket exactly reflects the source dir. Without
/// this a renamed or removed source file lingers across deploys -- eg a home route
/// converted `index.bsx` -> `index.md` leaves the stale `index.bsx`, so the served
/// binary sees two routes for `/` and panics on boot (`Duplicate route`). Mirroring
/// stops stale files from accumulating. The synced dirs (`site/`, `assets/`) are
/// read-only at runtime, so nothing in the bucket is authored anywhere but locally.
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
	let s3_uri = s3_fs_store.s3_store().s3_uri();
	let local_dir = s3_fs_store.fs_store().effective_root();
	trace!(
		"SyncS3BucketAction: syncing {} to {}",
		local_dir.display(),
		s3_uri
	);
	// `.delete(true)`: mirror, pruning bucket objects no longer in the local dir.
	S3Sync::push(local_dir, &s3_uri).delete(true).send().await?;
	trace!(
		"synced to {s3_uri} (region: {})",
		s3_fs_store.s3_store().region()
	);
	trace!("SyncS3BucketAction: complete");
	Pass(cx.input).xok()
}
