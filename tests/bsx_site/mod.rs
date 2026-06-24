//! Shared loader for the on-disk `examples/bsx_site` no-code example, used by the
//! project-level `bsx_site` render and `bsx_site_tui` live-terminal test suites.
//!
//! Files in a `tests/` subdirectory are not compiled as their own test target, so
//! this module is `#[path]`-included by both suites rather than run on its own.
use beet::prelude::*;

/// Build the on-disk `examples/bsx_site` site into a router root on `world`,
/// returning it. The store-backed load the `beet` binary's `build_site_root`
/// runs, in miniature: register the site `templates/`, parse the `main.bsx`
/// entry, build it onto a root carrying the store (so `<RoutesDir>`/`<Template
/// src>` resolve it by ancestry), then settle the async `<RoutesDir>` scan so
/// every discovered route exists before the first request.
///
/// The root carries [`DisableBootOnLoad`], so the entry's declared servers stay
/// dormant: callers render or navigate the site rather than boot it.
/// `RouterPlugin` registers the spread server/middleware types (`CliServer`,
/// `TuiServer`, `BsxLayout`, ...); `SshTuiServer` needs the ssh transport and is
/// absent, so its spread is skipped, exactly as a lean serve build would.
pub async fn build_site(world: &mut World) -> Entity {
	world.insert_resource(pkg_config!());
	let store = BlobStore::new(FsStore::new(
		AbsPathBuf::new_workspace_rel("examples/bsx_site").unwrap(),
	));
	let formats = world.get_resource_or_init::<TemplateFormats>().clone();
	let templates = read_site_templates(&store, &formats).await.unwrap();
	register_site_templates(world, &formats, templates).unwrap();

	let entry = store.get_media(&SmolPath::from("main.bsx")).await.unwrap();
	// the entry is always `.bsx`, so parse it through the core BSX engine (the
	// `.bsx` arm of the binary's format-dispatching `EntryTemplate::from_bytes`).
	let template =
		BsxTemplate::parse_entry(world, entry.as_utf8().unwrap()).unwrap();
	let root = world.spawn((DisableBootOnLoad, store)).id();
	world.entity_mut(root).insert_template(template).unwrap();
	AsyncRunner::settle_async_tasks(world).await;
	root
}
