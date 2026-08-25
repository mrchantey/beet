//! Resolving and loading a no-code entry, shared by the [`Serve`], [`Check`] and
//! [`ExportStatic`] commands.
//!
//! An entry is a directory containing an entry document ([`entry_build::ENTRY_NAMES`], eg
//! `main.bsx`) or the entry file itself, loaded as the app root with its declared
//! `<TemplateDir>`s registered. The entry declares its own servers and app routes;
//! these helpers only resolve and load it.

use crate::prelude::*;
use beet::prelude::*;

/// Load the entry onto the caller's world through its [`BlobStore`], returning its
/// root entity. Resolution is the binary's own [`entry_build::resolve_main`] (the
/// command's [`EntryParams`] `--store` picks the backend, the entry's
/// `<StoreRoot src>` widens the root), so the same resolution serves
/// `beet --main=..` and these commands.
///
/// `check`/`export-static` render the entry rather than serve it, so the build
/// disarms the root ([`DisableCallOnReady`]) and the entry's `CallOnReady` verb
/// stays dormant.
/// The reads go through the store ([`entry_build::read_sources`]/[`entry_build::build_root`]),
/// the same agnostic core the native binary and the wasm Worker use, so the
/// command is store-driven rather than filesystem-bound.
///
/// `settle_deadline` bounds the wait on the entry's discovery tasks. A one-shot
/// command passes [`ONE_SHOT_SETTLE_DEADLINE`]: it produces a result and exits,
/// so a dependency that never resolves must fail it rather than hang it. `serve`
/// passes [`None`] and waits indefinitely, since a slow dependency there is a
/// stall to wait out, not a failed run.
pub(crate) async fn build_entry(
	caller: &AsyncEntity,
	store_uri: Option<&StoreUri>,
	entry_path: &str,
	settle_deadline: Option<Duration>,
) -> Result<Entity> {
	let ResolvedEntry {
		store,
		entry_name,
		prescan,
		..
	} = entry_build::resolve_main(store_uri, entry_path).await?;
	let formats = caller
		.with_world(|world, _| {
			world.get_resource_or_init::<TemplateFormats>().clone()
		})
		.await?;
	let sources =
		entry_build::read_sources(&store, formats, entry_name, prescan).await?;
	let root = caller
		.with_world(move |world, _| {
			entry_build::build_root(world, store, sources, DisableCallOnReady)
		})
		.await??;
	// the entry's `<RoutesDir/>` discovery runs as an async task; wait for it so
	// every discovered route exists before the caller renders/exports the entry.
	match settle_deadline {
		Some(deadline) => {
			TemplatePending::settle_before(caller.world(), deadline).await?
		}
		None => TemplatePending::settle(caller.world()).await,
	}
	Ok(root)
}

/// How long a one-shot command (`check`, `export-static`, `export-pdf`) waits on
/// the entry's dependencies before failing.
///
/// Generous, since it covers a whole `<RoutesDir>` scan or `<TemplateDir>` read
/// over a large site on a cold cache; it exists to turn a genuinely stuck
/// dependency into a named error rather than a hung command, not to police build
/// speed.
pub(crate) const ONE_SHOT_SETTLE_DEADLINE: Duration = Duration::from_secs(60);

/// The entry's own directory (an entry file's parent, or the dir itself), the
/// default home for command outputs like `dist/` and `site.pdf`. Deliberately
/// not the `<StoreRoot>`-widened root, so outputs land beside the entry.
pub(crate) fn entry_dir(entry_path: &str) -> Result<AbsPathBuf> {
	let path = AbsPathBuf::new(entry_path)?;
	if path.extension().is_some() {
		path.parent()
			.ok_or_else(|| bevyhow!("entry `{path}` has no parent directory"))
	} else {
		path.xok()
	}
}

/// The store selector every entry-loading command shares (`serve`, `check`,
/// `export-static`, `export-pdf`), naming the backend the entry loads through.
///
/// A command loads an entry into *this* process rather than launching one, so
/// it reads its own `--store` param (documented in its `--help`) rather than
/// parsing a whole [`BootstrapConfig`] out of the request to reach one field.
#[derive(Reflect, Default)]
#[reflect(Default)]
pub(crate) struct EntryParams {
	/// The store the entry loads through, eg `--store=s3://my-bucket`. Defaults
	/// to the filesystem, rooted at the entry directory.
	store: Option<String>,
}

impl EntryParams {
	/// The entry store `parts` select, `None` for the default filesystem store
	/// rooted at the entry dir.
	pub fn store(parts: &RequestParts) -> Result<Option<StoreUri>> {
		parts
			.params()
			.parse_reflect::<Self>()?
			.store
			.map(|uri| StoreUri::parse(&uri))
			.transpose()
	}
}

/// The `*entry` path argument, joined back into a path. Errors with usage if empty.
pub(crate) fn entry_arg(parts: &RequestParts) -> Result<String> {
	let entry = parts
		.get_params("entry")
		.map(|segments| segments.join("/"))
		.unwrap_or_default();
	if entry.is_empty() {
		bevybail!("usage: beet <command> <entry-dir>");
	}
	entry.xok()
}

/// A world able to load and render a no-code entry for the `check` /
/// `export-static` / entry-load tests. It mirrors the running app's widget surface:
/// [`BsxDefaultsPlugin`] registers the beet_ui widget templates (`<Button>`/`<Form>`/…)
/// and the default `bx:` verb vocabulary, so a markup entry using live widgets renders
/// here as it would under `BeetPlugins`. It is added *before* [`RouterPlugin`] so its
/// inner `BsxPlugin` registers once (the router's charcell stack reaches it through the
/// idempotent `init_plugin`). [`RouterPlugin`] registers the markup-declarable
/// server *types* (`TuiServer`, `SshTuiServer`) by feature, so an entry's server
/// spread resolves without the ssh runtime systems (which need an input backend);
/// the disarmed build keeps the declared servers dormant anyway.
#[cfg(test)]
pub(crate) fn render_world() -> World {
	(
		AsyncPlugin,
		BsxDefaultsPlugin,
		RouterPlugin,
		material::MaterialStylePlugin::default(),
	)
		.into_world()
}

#[cfg(test)]
mod test {
	use super::*;

	fn entry_path() -> AbsPathBuf {
		AbsPathBuf::new_workspace_rel("examples/bsx_site").unwrap()
	}

	/// The shared [`entry_build::resolve_main`] serves the commands' positional: a dir resolves
	/// to its highest-priority [`entry_build::ENTRY_NAMES`] entry through the store, an entry
	/// file names itself, and a dir with no entry document errors with guidance.
	#[beet::test]
	async fn resolves_dir_and_entry_file() {
		// a dir resolves to its highest-priority `entry_build::ENTRY_NAMES` entry (`main.bsx` here)
		let dir = entry_build::resolve_main(
			None,
			entry_path().to_string_lossy().as_ref(),
		)
		.await
		.unwrap();
		dir.entry_name.xpect_eq("main.bsx");
		// passing the entry file itself roots the store at its parent
		let file = entry_build::resolve_main(
			None,
			entry_path().join("main.bsx").to_string_lossy().as_ref(),
		)
		.await
		.unwrap();
		file.entry_name.xpect_eq("main.bsx");
		file.watch_dir.xpect_eq(Some(entry_path()));
		// a non-`main.bsx` entry name is still discovered (the search spans entry_build::ENTRY_NAMES)
		let tmp = TempDir::new().unwrap();
		fs_ext::write(tmp.path().join("main.json"), "{}").unwrap();
		entry_build::resolve_main(None, tmp.path().to_string_lossy().as_ref())
			.await
			.unwrap()
			.entry_name
			.xpect_eq("main.json");
		// a dir with no entry document errors with guidance
		let empty = TempDir::new().unwrap();
		entry_build::resolve_main(
			None,
			empty.path().to_string_lossy().as_ref(),
		)
		.await
		.err()
		.unwrap()
		.to_string()
		.xpect_contains("no entry document");
	}

	/// The entry declares its own servers and app routes: loading its entry document
	/// through the resolved store yields a markup-declared `<HttpServer>` root
	/// carrying the router, plus the default app routes it requested with
	/// `<DefaultAppRoutes/>` (eg `/js/reactivity.js`).
	#[beet::test]
	async fn entry_declares_server_and_app_routes() {
		let mut world = render_world();
		let ResolvedEntry {
			store,
			entry_name,
			prescan,
			..
		} = entry_build::resolve_main(
			None,
			entry_path().to_string_lossy().as_ref(),
		)
		.await
		.unwrap();
		let formats = world.get_resource_or_init::<TemplateFormats>().clone();
		let sources =
			entry_build::read_sources(&store, formats, entry_name, prescan)
				.await
				.unwrap();
		let root = entry_build::build_root(
			&mut world,
			store,
			sources,
			DisableCallOnReady,
		)
		.unwrap();
		// the markup `<HttpServer>` owns the boot, with the router as its child
		world.entity(root).contains::<HttpServer>().xpect_true();
		// and `<DefaultAppRoutes/>` wired the reactivity-runtime route
		// the tree lives on the router's own url space, a child of the server
		RouteTree::of(&world, root)
			.unwrap()
			.find(&["js", "reactivity.js"])
			.xpect_some();
	}
}

/// Wait for the process [`CanonicalPort`] to register, ie the freshly booted
/// canonical http server's bound port. Shared by the commands that boot a
/// server on an ephemeral port and then drive it (`export-pdf`, the
/// browser-hosted `run-wasm`).
pub(crate) async fn wait_for_port() -> Result<u16> {
	for _ in 0..200 {
		if let Ok(port) = CanonicalPort::get() {
			return Ok(port);
		}
		time_ext::sleep_millis(25).await;
	}
	bevybail!("http server did not bind within 5s")
}
