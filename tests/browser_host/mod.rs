//! What the browser suites share: an on-disk example entry served the way the
//! binary serves it, and the console drained until the tab says what it did.
//!
//! An entry that forks is copied off disk into an in-memory store
//! ([`seeded_store`]), so the served page is always the authored one: the
//! fork the server writes on its first boot and the artifact its page boots
//! land in memory, and the on-disk fork left by a real session never leaks
//! into a case. Either way the entry is built through the plain one-shot path
//! the binary runs ([`serve_entry`]), its declared `<HttpServer>` booting on
//! an OS port, and the harness reads the url off the listener.
#![allow(dead_code)]

use beet::net::prelude::webdriver::*;
use beet::prelude::*;

/// The artifact the served pages boot, and the recipe that builds it.
pub const ARTIFACT: &str = "assets/wasm/beet-ui.wasm";
pub const BUILD_RECIPE: &str = "just build-wasm-ui";

/// The files at `paths` within the workspace-relative `dir`, copied off disk
/// into an in-memory store, a symlinked `assets` tree included.
pub async fn seeded_store(dir: &str, paths: &[&str]) -> BlobStore {
	let disk =
		BlobStore::new(FsStore::new(AbsPath::new_workspace_rel(dir).unwrap()));
	let store = BlobStore::temp();
	for path in paths {
		let path = RelPath::from(*path);
		let bytes = disk.get(&path).await.unwrap();
		store.insert(&path, bytes).await.unwrap();
	}
	store
}

/// Fail unless the artifact the pages boot is built, naming the recipe.
pub fn require_artifact() {
	let artifact = AbsPath::new_workspace_rel(ARTIFACT).unwrap();
	if !fs_ext::exists(&artifact).unwrap_or_default() {
		panic!("missing artifact `{artifact}`, run `{BUILD_RECIPE}`");
	}
}

/// Serve the entry at `entry` within `store` as `beet --main` would: the
/// entry resolved in its store, built through the plain one-shot path, its
/// declared `<HttpServer>` booting on an OS port. The launch, read once the
/// first server spawns: only the http server boots (a cli render would exit
/// the app), on an OS port rather than the default so a suite never fights a
/// running dev server.
pub async fn serve_entry(store: BlobStore, entry: &str) -> PageHarness {
	unsafe {
		env_ext::set_var("BEET_SERVER", "http").unwrap();
		env_ext::set_var("BEET_HTTP_PORT", "0").unwrap();
	}
	let entry = entry.to_string();
	PageHarness::serve_app(move |app| {
		app.add_plugins((
			RouterPlugin,
			material::MaterialStylePlugin::default(),
		));
		let formats = app
			.world_mut()
			.get_resource_or_init::<TemplateFormats>()
			.clone();
		app.world_mut().run_async_local(async move |world| {
			let ResolvedEntry {
				repo_store,
				entry_name,
				prescan,
				..
			} = entry_build::resolve_in_repo_store(store, entry).await?;
			let sources = entry_build::read_sources(
				&repo_store,
				formats,
				entry_name,
				prescan,
			)
			.await?;
			world
				.with(move |world: &mut World| {
					entry_build::build_root(
						world, repo_store, sources, RepoStore,
					)
					.map(|_| ())
				})
				.await
		});
	})
	.await
	.unwrap()
}

/// Drain the console into `log`, streaming each entry for the person
/// watching.
pub fn drain(console: &Collector<ConsoleEntry>, log: &mut String) {
	for entry in console.drain() {
		cross_log!("{}", entry.text);
		log.push_str(&entry.text);
		log.push('\n');
	}
}

/// Drain the console into `log` until `needle` appears, failing on a panic
/// or the deadline; `each` runs between drains, for a case sampling the page
/// while it boots.
pub async fn console_until_with(
	console: &Collector<ConsoleEntry>,
	log: &mut String,
	needle: &str,
	mut each: impl AsyncFnMut(),
) {
	let deadline = Instant::now() + Duration::from_secs(120);
	while !log.contains(needle) {
		drain(console, log);
		if log.contains("panicked") {
			panic!("the browser process panicked. console:\n{log}");
		}
		if Instant::now() > deadline {
			panic!("the console never said `{needle}`. console:\n{log}");
		}
		each().await;
		time_ext::sleep(Duration::from_millis(100)).await;
	}
}

/// Drain the console into `log` until `needle` appears.
pub async fn console_until(
	console: &Collector<ConsoleEntry>,
	log: &mut String,
	needle: &str,
) {
	console_until_with(console, log, needle, async || {}).await
}

/// The line the DOM host logs once it has painted `path`, carrying what it
/// adopted (see `DomHost`).
pub fn painted_line(log: &str, path: &str) -> String {
	let needle = format!("dom host painted {path} (");
	log.lines()
		.find(|line| line.contains(&needle))
		.unwrap_or_else(|| {
			panic!("no paint of `{path}` in the console:\n{log}")
		})
		.to_string()
}

/// Assert the tab painted `path` by adopting the served page whole: nothing
/// replaced, nothing patched, the conformance measure of an invisible boot.
pub fn assert_adopted_clean(log: &str, path: &str) {
	painted_line(log, path)
		.as_str()
		.xpect_contains("0 replaced, 0 patched");
}

/// The console's errors and the page's failed responses, empty when the boot
/// was clean.
pub fn assert_no_errors(log: &str, responses: &Collector<ResponseEvent>) {
	log.xnot().xpect_contains("ERROR");
	responses
		.drain()
		.into_iter()
		.filter(|response| response.is_error())
		.map(|response| format!("{} {}", response.status, response.url))
		.collect::<Vec<_>>()
		.xpect_empty();
}
