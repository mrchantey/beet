//! Mirroring one store into another from markup.
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// `<StoreSync src="fs:~/.claude/projects" dest="fs:store/claude-code"/>`:
/// mirror one store into another, additively: what the source holds the
/// destination holds too, and only objects whose size or digest differ are
/// sent. `delete` opts into a true mirror that prunes what the source lacks.
/// A relative `fs:` end roots at the workspace, so the declaration reads like
/// every other workspace path. Store-agnostic, so the same tag mirrors a
/// directory into a bucket or a memory store.
#[action]
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn StoreSync(
	/// The store read from.
	#[field(required)]
	src: StoreUri,
	/// The store written to.
	#[field(required)]
	dest: StoreUri,
	/// Prune destination objects absent from the source.
	#[field]
	delete: bool,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let root = AbsPath::workspace_root();
	let (src, dest) = (src.rooted_at(&root), dest.rooted_at(&root));
	let source = BlobStore::from_uri(&src)?;
	let target = BlobStore::from_uri(&dest)?;
	// only a mirror can destroy destination state, so only it is guarded
	if delete && source.list().await?.is_empty() {
		bevybail!(
			"refusing to mirror an empty store into {dest}: {src} is missing \
			 or empty"
		);
	}
	let report = BlobSync::new(source, target)
		.with_delete(delete)
		.run()
		.await?;
	info!("synced {src} -> {dest}: {report}");
	Pass(cx.input).xok()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_router::prelude::*;

	/// Run one `<StoreSync>` between two workspace-relative `fs:` uris, so
	/// the action's own rooting is what resolves them.
	async fn sync(src: &WsPath, dest: &WsPath, delete: bool) -> Result {
		let mut world =
			(AsyncPlugin, BootstrapPlugin, InfraPlugin).into_world();
		world
			.spawn((ExchangeSequence, children![StoreSync {
				src: Some(StoreUri::parse(&format!("fs:{src}"))?),
				dest: Some(StoreUri::parse(&format!("fs:{dest}"))?),
				delete,
			}]))
			.exchange(Request::get(""))
			.await
			.into_result()
			.await?;
		().xok()
	}

	async fn listed(store: &BlobStore) -> Vec<RelPath> {
		let mut paths = store.list().await.unwrap();
		paths.sort();
		paths
	}

	/// An additive sync leaves what the source lacks in place, and `delete`
	/// prunes it, so only a mirror can destroy destination state.
	#[beet_core::test]
	async fn mirrors_additively_then_prunes_on_delete() {
		let dir = TempDir::new_ws().unwrap();
		let (src, dest) = (dir.path().join("src"), dir.path().join("dest"));
		fs_ext::write(src.join("a.txt"), "a").unwrap();
		fs_ext::write(src.join("nested/b.txt"), "b").unwrap();
		fs_ext::write(dest.join("stale.txt"), "s").unwrap();
		let target = BlobStore::new(FsStore::new(dest.clone()));
		let (src, dest) =
			(src.into_ws_path().unwrap(), dest.into_ws_path().unwrap());

		sync(&src, &dest, false).await.unwrap();
		listed(&target).await.xpect_eq(vec![
			RelPath::new("a.txt"),
			RelPath::new("nested/b.txt"),
			RelPath::new("stale.txt"),
		]);

		sync(&src, &dest, true).await.unwrap();
		listed(&target).await.xpect_eq(vec![
			RelPath::new("a.txt"),
			RelPath::new("nested/b.txt"),
		]);
	}
}
