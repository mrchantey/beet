use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;


/// Sync local assets to the nearest ancestor [`S3Bucket`].
#[action]
#[derive(Default, Component)]
pub async fn SyncS3BucketAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let s3_bucket = cx
		.caller
		.with_state::<AncestorQuery<&S3Bucket>, _>(|entity, query| {
			query.get(entity).cloned()
		})
		.await?;
	let s3_uri = s3_bucket.s3_uri();
	let local_dir = AbsPathBuf::new_workspace_rel("examples/assets")?;
	S3Sync::push(local_dir, &s3_uri).send().await?;
	info!("synced assets to {s3_uri} (region: {})", s3_bucket.region());
	Pass(cx.input).xok()
}
