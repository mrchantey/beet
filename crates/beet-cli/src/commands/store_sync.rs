use beet::prelude::*;

/// Syncs one store into another, copying only what differs by size and digest
/// ([`BlobSync`]).
///
/// `--src` and `--dest` are any two store uris, so the direction is which is
/// which. `--delete` prunes extra files in the destination, `--dry-run`
/// previews without writing, `--follow-symlinks` reads through the links of a
/// filesystem end, `--region` names an S3 end's region (else the launch's, and
/// S3 redirects a miss) and `--no-sign-request` reads a public bucket unsigned,
/// ie:
///
/// ```sh
/// beet store-sync --src=s3://my-bucket/assets --dest=fs:assets
/// beet store-sync --src=fs:assets --dest=s3://my-bucket/assets --delete
/// ```
#[action(route = "store-sync")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn SyncStores(
	/// The store read from.
	#[param]
	src: StoreUri,
	/// The store written to.
	#[param]
	dest: StoreUri,
	/// Delete destination files not present in the source.
	#[param]
	delete: bool,
	/// Print what would change without syncing.
	#[param]
	dry_run: bool,
	/// Read through the symbolic links of a filesystem end.
	#[param]
	follow_symlinks: bool,
	/// AWS region of an S3 end, else the launch's.
	#[param]
	region: Option<String>,
	/// Skip request signing, for reading a public bucket.
	#[param]
	no_sign_request: bool,
) -> Result<String> {
	let sync =
		StoreMirror::new(SyncEnd::from_uri(&src)?, SyncEnd::from_uri(&dest)?)
			.with_delete(delete)
			.with_dry_run(dry_run)
			.with_follow_symlinks(follow_symlinks)
			.with_anonymous(no_sign_request);
	let report = match region {
		Some(region) => sync.with_region(region),
		None => sync,
	}
	.run()
	.await?;
	Ok(format!("{src} -> {dest}: {report}"))
}

#[cfg(test)]
mod test {
	use super::*;

	/// The command is one route over any two uris, its flags the sync's.
	#[beet::test]
	async fn syncs_two_uris() {
		let uri = |part: &str| format!("memory://cli-store-sync-{part}");
		let store = |part: &str| {
			BlobStore::from_uri(&StoreUri::parse(&uri(part)).unwrap()).unwrap()
		};
		let (src, dest) = (store("src"), store("dest"));
		src.insert(&RelPath::new("a.txt"), "a").await.unwrap();
		dest.insert(&RelPath::new("stale.txt"), "s").await.unwrap();
		let run = async |flags: &str| {
			let mut world = crate::commands::render_world();
			let router = world.spawn((Router, children![SyncStores])).id();
			world
				.entity_mut(router)
				.call::<Request, Response>(
					Request::from_cli_args(CliArgs::parse(&format!(
						"store-sync --src={} --dest={} {flags}",
						uri("src"),
						uri("dest")
					)))
					.with_header::<header::Accept>(vec![MediaType::Text]),
				)
				.await
				.unwrap()
		};
		run("--delete --dry-run")
			.await
			.unwrap_str()
			.await
			.xpect_contains("would have copied 1");
		dest.list()
			.await
			.unwrap()
			.xpect_eq(vec![RelPath::new("stale.txt")]);
		run("--delete").await.status().is_success().xpect_true();
		dest.list()
			.await
			.unwrap()
			.xpect_eq(vec![RelPath::new("a.txt")]);
		// a flag for another kind of store is refused, not ignored
		run("--no-sign-request")
			.await
			.status()
			.is_success()
			.xpect_false();
	}
}
