//! Shared loader for the on-disk `examples/beetmania` no-code example, used by the
//! project-level `beetmania` render, `beetmania_tui` live-terminal and
//! `beetmania_browser` test suites.
//!
//! Files in a `tests/` subdirectory are not compiled as their own test target, so
//! this module is `#[path]`-included by both suites rather than run on its own.
use beet::prelude::*;

/// Build the on-disk `examples/beetmania` site into a router root on `world`,
/// returning it. The store-backed load the `beet` binary runs, in miniature:
/// pre-scan the entry, build its body onto a root carrying the store (so
/// `<RoutesDir>` and the documents the site instantiates resolve it by ancestry),
/// then settle so every route and document is there before the first request.
///
/// The load declares no run, so the entry's declared servers stay
/// dormant: callers render or navigate the site rather than boot it.
/// `RouterPlugin` registers the spread server/middleware types (`CliServer`,
/// `TuiServer`, `Layout`, ...); `SshTuiServer` needs the ssh transport and is
/// absent, so its spread is skipped, exactly as a lean serve build would.
pub async fn build_site(world: &mut World) -> Entity {
	world.insert_resource(pkg_config!());
	let store = BlobStore::new(FsStore::new(
		AbsPath::new_workspace_rel("examples/beetmania").unwrap(),
	));
	let entry = store.get_media(&RelPath::from("main.bsx")).await.unwrap();
	let template =
		EntryPrescan::parse(&entry, world.resource::<PrescanRegistry>())
			.unwrap()
			.body(&DocParsers::of(world))
			.unwrap()
			.unwrap();
	let root = world.spawn_empty().id();
	world
		.entity_mut(root)
		.insert(store)
		.insert_template(template)
		.unwrap();
	AsyncRunner::settle_async_tasks(world).await;
	root
}
