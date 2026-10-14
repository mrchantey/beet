use beet::prelude::*;

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
#[action(route = "s3-sync")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn SyncS3(
	/// Source path, a local directory or an `s3://` URI.
	#[param]
	src: String,
	/// Destination path, a local directory or an `s3://` URI.
	#[param]
	dst: String,
	/// Delete destination files not present in the source.
	#[param]
	delete: bool,
	/// Print what would change without syncing.
	#[param]
	dry_run: bool,
	/// AWS region, defaults to the CLI's configured region.
	#[param]
	region: Option<String>,
	/// Skip request signing, for anonymous access to public buckets.
	#[param]
	no_sign_request: bool,
) -> Result<String> {
	if !AwsCli::is_s3_uri(&src) && !AwsCli::is_s3_uri(&dst) {
		bevybail!(
			"expected one of --src/--dst to be an s3:// URI, got {} and {}",
			src,
			dst
		);
	}

	let mut cli = AwsCli::new().with_no_sign_request(no_sign_request);
	if let Some(region) = &region {
		cli = cli.with_region(region.as_str());
	}

	S3Sync {
		cli,
		src: src.clone(),
		dst: dst.clone(),
		delete,
		dry_run,
		..default()
	}
	.send()
	.await?;
	Ok(format!("synced {} -> {}", src, dst))
}
