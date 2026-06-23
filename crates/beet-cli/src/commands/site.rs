//! Resolving and loading a no-code BSX site, shared by the [`Check`] and
//! [`ExportStatic`] commands.
//!
//! A site is a directory containing `main.bsx` (or the file itself), loaded as
//! the app root with its `templates/` registered. The site declares its own
//! servers and app routes in `main.bsx`; these helpers only resolve and load it.

use crate::prelude::*;
use beet::prelude::*;

/// Load the site onto the caller's world through its [`BlobStore`], returning its
/// root entity.
///
/// `check`/`export-static` render the site rather than serve it, so the root carries
/// [`DisableBootOnLoad`] to keep the entry's `BootOnLoad` verb dormant. The reads go
/// through the store ([`read_site_sources`]/[`build_site_root`]), the same agnostic
/// core the native binary and the wasm Worker use, so the command is store-driven
/// rather than filesystem-bound.
pub(crate) async fn build_site(
	caller: &AsyncEntity,
	site_dir: AbsPathBuf,
	entry: AbsPathBuf,
) -> Result<Entity> {
	let entry_name = entry
		.file_name()
		.and_then(|name| name.to_str())
		.ok_or_else(|| bevyhow!("entry `{entry}` has no file name"))?
		.to_string();
	let store = BlobStore::new(FsStore::new(site_dir));
	let formats = caller
		.with_world(|world, _| {
			world.get_resource_or_init::<TemplateFormats>().clone()
		})
		.await?;
	let sources = read_site_sources(&store, formats, entry_name).await?;
	let root = caller
		.with_world(move |world, _| {
			build_site_root(world, store, sources, DisableBootOnLoad)
		})
		.await??;
	// the entry's `<RoutesDir/>` discovery runs as an async task; wait for it so
	// every discovered route exists before the caller renders/exports the site.
	settle_routes_dirs(caller.world()).await?;
	Ok(root)
}

/// The `*site` path argument, joined back into a path. Errors with usage if empty.
pub(crate) fn site_arg(parts: &RequestParts) -> Result<String> {
	let site = parts
		.get_params("site")
		.map(|segments| segments.join("/"))
		.unwrap_or_default();
	if site.is_empty() {
		bevybail!("usage: beet <command> <site-dir>");
	}
	site.xok()
}

/// A resolved site: its root directory and the `main.bsx` entry file.
pub(crate) struct SiteEntry {
	pub site_dir: AbsPathBuf,
	pub entry: AbsPathBuf,
}

/// Resolve a site path: a directory containing `main.bsx`, or the file itself.
/// Relative paths resolve against the cwd; an absolute positional round-trips as
/// absolute (the `*site` capture keeps its leading `/`), so any cwd resolves it.
pub(crate) fn resolve_site(site: &str) -> Result<SiteEntry> {
	let path = AbsPathBuf::new(site)?;
	if !fs_ext::exists(&path)? {
		bevybail!("site not found: {site}");
	}
	if path.is_dir() {
		let entry = path.join("main.bsx");
		if !fs_ext::exists(&entry)? {
			bevybail!("no main.bsx in {path}");
		}
		SiteEntry {
			site_dir: path,
			entry,
		}
		.xok()
	} else {
		let site_dir = path
			.parent()
			.ok_or_else(|| bevyhow!("site file has no parent dir: {path}"))?;
		SiteEntry {
			site_dir,
			entry: path,
		}
		.xok()
	}
}

/// A world able to load and render a no-code site for the `check` /
/// `export-static` / site-load tests. Beyond the render plugins it registers the
/// markup-declarable `SshTuiServer` *type* so a site's server spread resolves;
/// the binary registers it through `SshTuiPlugin`, but the type alone suffices
/// here without pulling the ssh runtime systems (which need an input backend),
/// and `DisableBootOnLoad` keeps the declared server dormant anyway.
#[cfg(test)]
pub(crate) fn render_world() -> World {
	let mut world = (
		AsyncPlugin,
		RouterPlugin,
		material::MaterialStylePlugin::default(),
	)
		.into_world();
	#[cfg(feature = "ssh")]
	world
		.resource_mut::<AppTypeRegistry>()
		.write()
		.register::<SshTuiServer>();
	world
}

#[cfg(test)]
mod test {
	use super::*;

	fn site_path() -> AbsPathBuf {
		AbsPathBuf::new_workspace_rel("examples/bsx_site").unwrap()
	}

	#[beet::test]
	fn resolves_dir_and_entry_file() {
		let dir = resolve_site(site_path().to_string_lossy().as_ref()).unwrap();
		dir.entry.xpect_eq(site_path().join("main.bsx"));
		let file = resolve_site(
			site_path().join("main.bsx").to_string_lossy().as_ref(),
		)
		.unwrap();
		file.site_dir.xpect_eq(site_path());
		resolve_site("not/a/site")
			.err()
			.unwrap()
			.to_string()
			.xpect_contains("site not found");
	}

	/// The site declares its own server and app routes: loading `main.bsx` through
	/// the site store yields a root carrying the markup-declared `HttpServer` plus
	/// the default app routes it requested with `<DefaultAppRoutes/>` (eg
	/// `/js/reactivity.js`).
	#[beet::test]
	async fn site_declares_server_and_app_routes() {
		let mut world = render_world();
		let SiteEntry { site_dir, entry } =
			resolve_site(site_path().to_string_lossy().as_ref()).unwrap();
		let entry_name = entry
			.file_name()
			.and_then(|name| name.to_str())
			.unwrap()
			.to_string();
		let store = BlobStore::new(FsStore::new(site_dir));
		let formats = world.get_resource_or_init::<TemplateFormats>().clone();
		let sources = read_site_sources(&store, formats, entry_name)
			.await
			.unwrap();
		let root =
			build_site_root(&mut world, store, sources, DisableBootOnLoad)
				.unwrap();
		// the markup `<Router {(.., HttpServer{..})}>` declared a server
		world.entity(root).contains::<HttpServer>().xpect_true();
		// and `<DefaultAppRoutes/>` wired the reactivity-runtime route
		world
			.entity(root)
			.get::<RouteTree>()
			.unwrap()
			.find(&["js", "reactivity.js"])
			.xpect_some();
	}
}
