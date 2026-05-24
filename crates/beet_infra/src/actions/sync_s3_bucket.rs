use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Sync local assets to the nearest ancestor [`S3FsStore`].
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
	S3Sync::push(local_dir, &s3_uri).send().await?;
	trace!(
		"synced assets to {s3_uri} (region: {})",
		s3_fs_store.s3_store().region()
	);
	trace!("SyncS3BucketAction: complete");
	Pass(cx.input).xok()
}
