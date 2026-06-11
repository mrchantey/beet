use beet::prelude::*;

/// Request params for the [`SyncS3`] command, surfaced in `--help`.
#[derive(Reflect, Default)]
#[reflect(Default)]
struct SyncS3Params {
	/// Source path, a local directory or an `s3://` URI.
	#[reflect(@RequiredField)]
	src: String,
	/// Destination path, a local directory or an `s3://` URI.
	#[reflect(@RequiredField)]
	dst: String,
	/// Delete destination files not present in the source.
	delete: bool,
	/// Print what would change without syncing.
	dry_run: bool,
	/// AWS region, defaults to the CLI's configured region.
	region: Option<String>,
	/// Skip request signing, for anonymous access to public buckets.
	no_sign_request: bool,
}

/// Syncs a directory between the local filesystem and S3 via `aws s3 sync`.
///
/// `--src` and `--dst` are the endpoints, one local and one an `s3://` URI; the
/// direction follows whichever is the bucket. `--delete` prunes extra files in
/// the destination, `--dry-run` previews without writing, and `--region` /
/// `--no-sign-request` configure the AWS CLI, ie:
///
/// ```sh
/// beet s3-sync --src=s3://my-bucket/assets --dst=./assets
/// beet s3-sync --src=./assets --dst=s3://my-bucket/assets --delete
/// ```
#[action(route = "s3-sync", handler_only)]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(ParamsPartial = ParamsPartial::new::<SyncS3Params>())]
pub async fn SyncS3(parts: RequestParts) -> Result<String> {
	let params = parts.params().parse_reflect::<SyncS3Params>()?;
	if !is_s3_uri(&params.src) && !is_s3_uri(&params.dst) {
		bevybail!(
			"expected one of --src/--dst to be an s3:// URI, got {} and {}",
			params.src,
			params.dst
		);
	}

	let mut cli = AwsCli::new().with_no_sign_request(params.no_sign_request);
	if let Some(region) = &params.region {
		cli = cli.with_region(region.as_str());
	}

	S3Sync {
		cli,
		src: params.src.clone(),
		dst: params.dst.clone(),
		delete: params.delete,
		dry_run: params.dry_run,
		..default()
	}
	.send()
	.await?;
	Ok(format!("synced {} -> {}", params.src, params.dst))
}
