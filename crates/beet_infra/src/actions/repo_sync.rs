//! Publishing the staged document into the repo store.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// `<RepoSync/>` — mirror the staging dir into this launch's version of the
/// repo store, `<root>/<id>/repo/`.
///
/// One [`BlobSync`] with `delete`, so the store holds exactly what
/// [`RepoStage`] assembled: a page renamed or removed at the source is
/// removed here rather than lingering (two routes for `/` is a boot panic),
/// and an object already matching by size and digest is not re-sent. Which
/// version it publishes into is the launch's answer: `deploy` mints one, and
/// a content-only `sync` runs [`AdoptCurrentDeploy`] first to publish into
/// the version being served.
///
/// The mirror root is `repo/` under the version, never the version itself, so
/// the binary and ledger beside it are structurally out of a content sync's
/// reach. Store-agnostic: the declaration decides the backend, so a test
/// declares a memory store and reads the mirror back.
#[action]
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn RepoSync(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let (dir, dest) = cx
		.caller
		.with_state::<(StackQuery, RepoStoreQuery), _>(
			|entity, (stacks, repos)| -> Result<_> {
				(RepoStage::dir(&stacks, entity)?, repos.store_uri(entity)?)
					.xok()
			},
		)
		.await??;
	if fs_ext::is_dir_empty(&dir)? {
		bevybail!(
			"nothing staged at {dir}: declare a `<RepoStage src=\"..\"/>` \
			 before this sync"
		);
	}
	let report = BlobSync::new(
		BlobStore::new(FsStore::new(dir.clone())),
		BlobStore::from_uri(&dest)?,
	)
	.with_delete(true)
	.run()
	.await?;
	info!("synced {dir} -> {dest}: {report}");
	Pass(cx.input).xok()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_router::prelude::*;

	/// A site tree and a workspace tree it borrows from, both under `target/`
	/// so the stage and the copy can name them workspace-relative. The site
	/// carries a `target/` of its own, as a checkout-root entry does, which a
	/// staged allowlist leaves behind.
	fn trees() -> (TempDir, WsPath, WsPath) {
		let dir = TempDir::new_ws().unwrap();
		let site = dir.path().join("site");
		let workspace = dir.path().join("assets");
		fs_ext::write(site.join("main.bsx"), "<div/>").unwrap();
		fs_ext::write(site.join("routes/index.md"), "# hi").unwrap();
		fs_ext::write(site.join("target/debug/junk"), "junk").unwrap();
		fs_ext::write(workspace.join("wasm/app.wasm"), "wasm").unwrap();
		let (site, workspace) = (
			site.into_ws_path().unwrap(),
			workspace.into_ws_path().unwrap(),
		);
		(dir, site, workspace)
	}

	/// Run the deploy's publish steps of `deployment` against a memory repo
	/// store, returning the store's document root for that launch and the
	/// staging dir.
	async fn publish(
		deployment: &Deployment,
		site: &WsPath,
		workspace: &WsPath,
	) -> Result<(BlobStore, AbsPath)> {
		publish_paths(deployment, site, workspace, "main.bsx,routes").await
	}

	/// [`publish`] staging the `paths` under `site` alone.
	async fn publish_paths(
		deployment: &Deployment,
		site: &WsPath,
		workspace: &WsPath,
		paths: &str,
	) -> Result<(BlobStore, AbsPath)> {
		let deploy_id = *deployment.deploy_id();
		let root = StoreUri::parse("memory://repo-sync-test")?;
		// held open before the sync runs: a named memory store lives only
		// while a handle on it does
		let document = BlobStore::from_uri(
			&root.with_subdir(ArtifactLedger::version_repo_dir(&deploy_id)),
		)?;
		let mut world =
			(AsyncPlugin, BootstrapPlugin, InfraPlugin).into_world();
		world.insert_resource(deployment.clone());
		world
			.spawn((Stack::new("app"), ExchangeSequence, children![
				(
					StoreUriBlock::new("repo", root.clone())
						.with_deploy_versioned(true),
					RepoStoreBlock
				),
				(RepoStage::new(site.clone()).with_paths(paths), children![
					DirCopy::new(workspace.clone(), "assets", "wasm/app.wasm")
				]),
				RepoSync::default(),
			]))
			.exchange(Request::get(""))
			.await
			.into_result()
			.await?;
		let staging = deployment
			.work_directory(
				&Stack::new("app").resolve(&PackageConfig::default()),
			)
			.into_abs()
			.join("repo");
		(document, staging).xok()
	}

	fn sorted_stats(
		mut stats: Vec<(RelPath, BlobStat)>,
	) -> Vec<(RelPath, BlobStat)> {
		stats.sort_by(|a, b| a.0.cmp(&b.0));
		stats
	}

	/// The staging dir IS the store's contents: the site plus what it borrows
	/// lands under this launch's `<id>/repo/`, the allowlist keeps the site's
	/// own `target/` out, nothing is written into the source tree, and a file
	/// removed from the source leaves the mirror.
	#[beet_core::test]
	async fn the_staging_dir_is_the_published_document() {
		let (dir, site, workspace) = trees();
		let (deployment, _work_dir) = Deployment::default_local();
		let (document, staging) =
			publish(&deployment, &site, &workspace).await.unwrap();
		let mut published = document.list().await.unwrap();
		published.sort();
		published.xpect_eq(vec![
			RelPath::new("assets/wasm/app.wasm"),
			RelPath::new("main.bsx"),
			RelPath::new("routes/index.md"),
		]);
		sorted_stats(document.list_stats().await.unwrap()).xpect_eq(
			sorted_stats(
				BlobStore::new(FsStore::new(staging))
					.list_stats()
					.await
					.unwrap(),
			),
		);
		// the borrow landed in the stage, never in the checkout
		fs_ext::exists(dir.path().join("site/assets"))
			.unwrap()
			.xpect_false();

		// a content sync into the same version mirrors: the removed page is
		// gone from the store
		fs_ext::remove(dir.path().join("site/routes/index.md")).unwrap();
		let (document, _) =
			publish(&deployment, &site, &workspace).await.unwrap();
		document
			.exists(&RelPath::new("routes/index.md"))
			.await
			.unwrap()
			.xpect_false();
	}

	/// With no allowlist the source is staged whole, and an allowlisted path
	/// that is absent fails the stage rather than publishing without it.
	#[beet_core::test]
	async fn a_stage_without_paths_publishes_the_source_whole() {
		let (_dir, site, workspace) = trees();
		let (deployment, _work_dir) = Deployment::default_local();
		let (document, _) = publish_paths(&deployment, &site, &workspace, "")
			.await
			.unwrap();
		document
			.exists(&RelPath::new("target/debug/junk"))
			.await
			.unwrap()
			.xpect_true();
		// the error flows back as the exchange's failed response
		publish_paths(&deployment, &site, &workspace, "main.bsx,missing")
			.await
			.unwrap_err();
	}
}
