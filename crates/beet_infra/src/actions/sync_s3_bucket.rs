use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

#[derive(Default, Clone, Component)]
#[require(SyncS3BucketAction)]
pub struct SyncS3Bucket {
	path: WsPathBuf,
}

impl SyncS3Bucket {
	pub fn new(path: impl Into<WsPathBuf>) -> Self {
		Self { path: path.into() }
	}
}
/// Sync local assets to the nearest ancestor [`S3Bucket`].
#[action]
#[derive(Default, Component)]
async fn SyncS3BucketAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let s3_bucket = cx
		.caller
		.with_state::<AncestorQuery<&S3Bucket>, _>(|entity, query| {
			query.get(entity).cloned()
		})
		.await?;
	let s3_uri = s3_bucket.s3_uri();
	let local_dir = cx
		.caller
		.get::<SyncS3Bucket, _>(|sync| sync.path.into_abs())
		.await?;
	S3Sync::push(local_dir, &s3_uri).send().await?;
	info!("synced assets to {s3_uri} (region: {})", s3_bucket.region());
	Pass(cx.input).xok()
}
