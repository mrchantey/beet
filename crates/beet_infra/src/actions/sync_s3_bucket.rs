use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;


/// Sync local assets to the nearest ancestor [`S3BucketBlock`]'s bucket.
#[action]
#[derive(Default, Component)]
pub async fn SyncS3BucketAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let (bucket_name, region) = cx
		.caller
		.with_state::<(AncestorQuery<&S3BucketBlock>, AncestorQuery<&Stack>), _>(
			|entity, (bucket_query, stack_query)| -> Result<_> {
				let bucket_block = bucket_query.get(entity)?;
				let stack = stack_query.get(entity)?;
				let name = stack
					.resource_ident(bucket_block.label().clone())
					.primary_identifier()
					.to_string();
				let region = bucket_block
					.region
					.as_ref()
					.unwrap_or(stack.aws_region())
					.clone();
				(name, region).xok()
			},
		)
		.await?;
	let local_dir = AbsPathBuf::new_workspace_rel("examples/assets")?;
	S3Sync::push(local_dir, format!("s3://{bucket_name}/"))
		.send()
		.await?;
	info!("synced assets to s3://{bucket_name}/ (region: {region})");
	Pass(cx.input).xok()
}
