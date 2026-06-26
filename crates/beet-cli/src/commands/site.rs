//! Resolving and loading a no-code site, shared by the [`Serve`], [`Check`] and
//! [`ExportStatic`] commands.
//!
//! A site is a directory containing an entry document ([`ENTRY_NAMES`], eg
//! `main.bsx`) or the entry file itself, loaded as the app root with its declared
//! `<TemplateDir>`s registered. The site declares its own servers and app routes in
//! the entry; these helpers only resolve and load it.

use crate::prelude::*;
use beet::prelude::*;

/// Load the site onto the caller's world through its [`BlobStore`], returning its
/// root entity. The store comes from the request's `--store` param ([`resolve_store`],
/// default an `fs` store rooted at `site_dir`), so the same `--store` selector serves
/// the binary and these commands.
///
/// `check`/`export-static` render the site rather than serve it, so the root carries
/// [`DisableBootOnLoad`] to keep the entry's `BootOnLoad` verb dormant. The reads go
/// through the store ([`read_entry_sources`]/[`build_entry_root`]), the same agnostic
/// core the native binary and the wasm Worker use, so the command is store-driven
/// rather than filesystem-bound.
pub(crate) async fn build_site(
	caller: &AsyncEntity,
	params: &MultiMap<SmolStr, SmolStr>,
	site_dir: AbsPathBuf,
	entry: AbsPathBuf,
) -> Result<Entity> {
	let entry_name = entry
		.file_name()
		.and_then(|name| name.to_str())
		.ok_or_else(|| bevyhow!("entry `{entry}` has no file name"))?
		.to_string();
	let store = resolve_store(params, site_dir)?;
	let formats = caller
		.with_world(|world, _| {
			world.get_resource_or_init::<TemplateFormats>().clone()
		})
		.await?;
	let sources = read_entry_sources(&store, formats, entry_name).await?;
	let root = caller
		.with_world(move |world, _| {
			build_entry_root(world, store, sources, DisableBootOnLoad)
		})
		.await??;
	// the entry's `<RoutesDir/>` discovery runs as an async task; wait for it so
	// every discovered route exists before the caller renders/exports the site.
	RoutesDir::settle_all(caller.world()).await?;
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

/// A resolved site: its root directory and the entry-document file ([`ENTRY_NAMES`]).
pub(crate) struct SiteEntry {
	pub site_dir: AbsPathBuf,
	pub entry: AbsPathBuf,
}

/// Resolve a site path: a directory containing an entry document ([`ENTRY_NAMES`], in
/// priority order), or the entry file itself. Mirrors the binary's discovery, but
/// within the given dir rather than walking ancestors. Relative paths resolve against
/// the cwd; an absolute positional round-trips as absolute (the `*site` capture keeps
/// its leading `/`), so any cwd resolves it.
pub(crate) fn resolve_site(site: &str) -> Result<SiteEntry> {
	let path = AbsPathBuf::new(site)?;
	if !fs_ext::exists(&path)? {
		bevybail!("site not found: {site}");
	}
	if path.is_dir() {
		// the first entry name present in the dir wins, matching the binary's order.
		let entry = ENTRY_NAMES
			.iter()
			.map(|name| path.join(name))
			.find(|entry| fs_ext::exists(entry).unwrap_or(false))
			.ok_or_else(|| {
				bevyhow!("no entry document {ENTRY_NAMES:?} in {path}")
			})?;
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
/// `export-static` / site-load tests. It mirrors the running app's widget surface:
/// [`BsxDefaultsPlugin`] registers the beet_ui widget templates (`<Button>`/`<Form>`/…)
/// and the default `bx:` verb vocabulary, so a markup site using live widgets renders
/// here as it would under `BeetPlugins`. It is added *before* [`RouterPlugin`] so its
/// inner `BsxPlugin` registers once (the router's charcell stack reaches it through the
/// idempotent `init_plugin`). It also registers the markup-declarable `SshTuiServer`
/// *type* so a site's server spread resolves; the binary registers it through
/// `SshTuiPlugin`, but the type alone suffices here without pulling the ssh runtime
/// systems (which need an input backend), and `DisableBootOnLoad` keeps the declared
/// server dormant anyway.
#[cfg(test)]
pub(crate) fn render_world() -> World {
	let mut world = (
		AsyncPlugin,
		BsxDefaultsPlugin,
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
		// a dir resolves to its highest-priority `ENTRY_NAMES` entry (`main.bsx` here)
		let dir = resolve_site(site_path().to_string_lossy().as_ref()).unwrap();
		dir.entry.xpect_eq(site_path().join("main.bsx"));
		// passing the entry file itself resolves the dir as its parent
		let file = resolve_site(
			site_path().join("main.bsx").to_string_lossy().as_ref(),
		)
		.unwrap();
		file.site_dir.xpect_eq(site_path());
		// a non-`main.bsx` entry name is still discovered (the search spans ENTRY_NAMES)
		let tmp = TempDir::new().unwrap();
		fs_ext::write(tmp.path().join("main.json"), "{}").unwrap();
		resolve_site(tmp.path().to_string_lossy().as_ref())
			.unwrap()
			.entry
			.xpect_eq(tmp.path().join("main.json"));
		// a missing path errors, and a dir with no entry document errors with guidance
		resolve_site("not/a/site")
			.err()
			.unwrap()
			.to_string()
			.xpect_contains("site not found");
		let empty = TempDir::new().unwrap();
		resolve_site(empty.path().to_string_lossy().as_ref())
			.err()
			.unwrap()
			.to_string()
			.xpect_contains("no entry document");
	}

	/// The site declares its own server and app routes: loading its entry document
	/// through the resolved site store yields a root carrying the markup-declared
	/// `HttpServer` plus the default app routes it requested with `<DefaultAppRoutes/>`
	/// (eg `/js/reactivity.js`).
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
		// default params → the same `fs` store the command resolves
		let store = resolve_store(&default(), site_dir).unwrap();
		let formats = world.get_resource_or_init::<TemplateFormats>().clone();
		let sources = read_entry_sources(&store, formats, entry_name)
			.await
			.unwrap();
		let root =
			build_entry_root(&mut world, store, sources, DisableBootOnLoad)
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
