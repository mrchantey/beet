//! Shared loader for the on-disk `examples/bsx_site` no-code example, used by the
//! project-level `bsx_site` render and `bsx_site_tui` live-terminal test suites.
//!
//! Files in a `tests/` subdirectory are not compiled as their own test target, so
//! this module is `#[path]`-included by both suites rather than run on its own.
use beet::prelude::*;

/// Build the on-disk `examples/bsx_site` site into a router root on `world`,
/// returning it. The store-backed load the `beet` binary's `build_entry_root`
/// runs, in miniature: pre-scan the entry's declared `<TemplateDir>`s and register
/// their templates (so entry-level tags like `<Styles/>` resolve), parse the
/// `main.bsx` entry, build it onto a root carrying the store (so `<RoutesDir>`/
/// `<Template src>` resolve it by ancestry), then settle the async `<RoutesDir>` /
/// `<TemplateDir>` scans so every route/template exists before the first request.
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
	let entry = store.get_media(&SmolPath::from("main.bsx")).await.unwrap();
	// the entry is always `.bsx`, so parse it through the core BSX engine (the
	// `.bsx` arm of the binary's format-dispatching `EntryTemplate::from_bytes`).
	let source = entry.as_utf8().unwrap();
	// pre-scan: register the entry's declared `<TemplateDir>`s before parsing, so
	// entry-level tags resolve against them.
	let nodes = parse_document(source, &BsxParseConfig::bsx()).unwrap();
	for dir in TemplateDir::extract_dirs(&nodes) {
		let sources = TemplateDir::read_sources(&store, &dir, &formats)
			.await
			.unwrap();
		TemplateDir::register_sources(world, &formats, sources).unwrap();
	}
	let template = BsxTemplate::parse_entry(world, source).unwrap();
	let root = world.spawn((DisableBootOnLoad, store)).id();
	world.entity_mut(root).insert_template(template).unwrap();
	AsyncRunner::settle_async_tasks(world).await;
	root
}
