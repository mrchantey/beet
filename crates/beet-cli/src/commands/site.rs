//! Resolving and loading a no-code BSX site, shared by the [`Check`] and
//! [`ExportStatic`] commands.
//!
//! A site is a directory containing `main.bsx` (or the file itself), loaded as
//! the app root with its `templates/` registered. The site declares its own
//! servers and app routes in `main.bsx`; these helpers only resolve and load it.

use beet::prelude::*;

/// Load the site onto the caller's world, returning its root entity.
pub(crate) async fn build_site(
	caller: &AsyncEntity,
	site_dir: AbsPathBuf,
	entry: AbsPathBuf,
) -> Result<Entity> {
	caller
		.with_world(move |world, _caller| load_site(world, site_dir, entry))
		.await?
}

/// The synchronous site load: register the site's `templates/`, set the
/// [`SiteRoot`] (which `<RoutesDir/>` resolves against), and spawn the
/// `main.bsx` entry as the app root.
pub(crate) fn load_site(
	world: &mut World,
	site_dir: AbsPathBuf,
	entry: AbsPathBuf,
) -> Result<Entity> {
	let entry_name = entry
		.file_name()
		.and_then(|name| name.to_str())
		.ok_or_else(|| bevyhow!("entry `{entry}` has no file name"))?;
	let site_root = SiteRoot::new_fs(site_dir);
	// register `templates/` and read the entry document through the site store
	// (the same path a deployed task takes against S3).
	site_root.register_templates(world)?;
	let bytes =
		async_ext::block_on(site_root.0.get(&SmolPath::from(entry_name)))?;
	let source = core::str::from_utf8(&bytes)?;
	// insert the `SiteRoot` resource before spawning so the route-discovery
	// observer and `<Template src>` includes resolve against it.
	world.insert_resource(site_root);
	let template = BsxTemplate::parse_entry(world, source)?;
	// `check`/`export-static` render the site, never serve it: build into a root
	// carrying `DisableBootOnLoad` so the entry's `BootOnLoad` verb stays dormant.
	let root = world.spawn(DisableBootOnLoad).id();
	world.entity_mut(root).insert_template(template)?;
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

	/// The site declares its own server and app routes: loading `main.bsx` yields
	/// a root carrying the markup-declared `HttpServer` plus the default app
	/// routes it requested with `<DefaultAppRoutes/>` (eg `/js/reactivity.js`).
	#[beet::test]
	fn site_declares_server_and_app_routes() {
		let mut world = render_world();
		let SiteEntry { site_dir, entry } =
			resolve_site(site_path().to_string_lossy().as_ref()).unwrap();
		let root = load_site(&mut world, site_dir, entry).unwrap();
		world.flush();
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
