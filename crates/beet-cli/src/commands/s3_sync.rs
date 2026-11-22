use beet::prelude::*;

/// Syncs a directory between the local filesystem and S3, copying only what
/// differs by size and digest ([`BlobSync`]).
///
/// `--src` and `--dst` are the endpoints, one local and one an `s3://` URI; the
/// direction follows whichever is the bucket. `--delete` prunes extra files in
/// the destination, `--dry-run` previews without writing, `--region` names the
/// bucket's region (else the launch's, and S3 redirects a miss) and
/// `--no-sign-request` reads a public bucket unsigned, ie:
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
	/// AWS region of the bucket, else the launch's.
	#[param]
	region: Option<String>,
	/// Skip request signing, for anonymous access to public buckets.
	#[param]
	no_sign_request: bool,
) -> Result<String> {
	let end = |path: &str| -> Result<BlobStore> {
		match path.starts_with("s3://") {
			true => {
				let store = S3Store::from_uri(&StoreUri::parse(path)?)?
					.with_anonymous(no_sign_request);
				match &region {
					Some(region) => store.with_region(region.as_str()),
					None => store,
				}
				.xmap(BlobStore::new)
				.xok()
			}
			false => BlobStore::new(FsStore::new(AbsPath::new(path)?)).xok(),
		}
	};
	if !src.starts_with("s3://") && !dst.starts_with("s3://") {
		bevybail!(
			"expected one of --src/--dst to be an s3:// URI, got {} and {}",
			src,
			dst
		);
	}
	let report = BlobSync::new(end(&src)?, end(&dst)?)
		.with_delete(delete)
		.with_dry_run(dry_run)
		.run()
		.await?;
	Ok(format!("{src} -> {dst}: {report}"))
}
