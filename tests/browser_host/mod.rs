//! Shared harness for the browser acceptance suites over the on-disk example
//! entries: an entry served the way the binary serves it, opened in a headless
//! browser whose console is drained until the tab says what it did, and driven
//! with trusted webdriver input.
//!
//! The browser twin of `tui_host`: every piece an example is made of is
//! unit-tested in its own crate (the DOM sink and input path under
//! `#[beet_core::test(browser)]`), and what only a suite over the *authored*
//! entry can show is that the served page boots the same entry in the tab and
//! the pieces assemble there. An entry that forks is copied off disk into an
//! in-memory store ([`BrowserHost::seeded_store`], the same seeding
//! `TuiHost` does), so the fork the server writes on its first boot and the
//! artifact its page boots land in memory and the on-disk fork left by a real
//! session never leaks into a case; an entry that does not fork is served from
//! its own directory. Either way the entry is built through the plain one-shot
//! path the binary runs, its declared `<HttpServer>` booting on an OS port,
//! and the harness reads the url off the listener.
//!
//! Files in a `tests/` subdirectory are not compiled as their own test target,
//! so each suite `mod`-includes this module and layers its own document
//! helpers over the host.
#![allow(dead_code)]

use beet::net::prelude::webdriver::*;
use beet::prelude::*;
use core::ops::Deref;
use core::ops::DerefMut;

#[path = "../example_store/mod.rs"]
mod example_store;

/// The artifact the served pages boot, and the recipe that builds it.
pub const ARTIFACT: &str = "assets/wasm/beet-ui.wasm";
pub const BUILD_RECIPE: &str = "just build-wasm-ui";

/// How long a case waits for the tab to paint: the artifact fetched,
/// instantiated and booted through the served repo store, generous for a
/// loaded host.
const PAINT_DEADLINE: Duration = Duration::from_secs(120);

/// A served example in a driven tab: the entry built and served as the
/// binary serves it, the tab's console and responses collected, and the
/// gestures every suite makes. Derefs to the [`PageHarness`], so the whole
/// webdriver surface (auto-waiting finds, trusted input, async matchers) is
/// reachable on the host.
pub struct BrowserHost {
	pub page: PageHarness,
	console: Collector<ConsoleEntry>,
	responses: Collector<ResponseEvent>,
	/// Everything the console has said since the last [`Self::goto`],
	/// streamed as it arrives for the person watching.
	pub log: String,
	/// Every error the console has reported since the tab opened, whatever
	/// the load, which [`Self::kill`] refuses on.
	errors: Vec<String>,
	/// The store the entry was built from, so a suite can read back what the
	/// server persisted.
	pub store: BlobStore,
}

impl Deref for BrowserHost {
	type Target = PageHarness;
	fn deref(&self) -> &Self::Target { &self.page }
}

impl DerefMut for BrowserHost {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.page }
}

impl BrowserHost {
	/// The files at `paths` within the workspace-relative `dir`, copied off
	/// disk into an in-memory store, a symlinked `assets` tree included.
	pub async fn seeded_store(dir: &str, paths: &[&str]) -> BlobStore {
		example_store::seeded_store(dir, paths).await
	}

	/// The on-disk example at the workspace-relative `dir`, served as it
	/// stands: for an entry that writes nothing beside its sources.
	pub fn disk_store(dir: &str) -> BlobStore {
		BlobStore::new(FsStore::new(AbsPath::new_workspace_rel(dir).unwrap()))
	}

	/// Serve the entry at `entry` within `store` and open a tab on nothing
	/// yet, its collectors attached, so the first [`Self::goto`] is watched
	/// from its first byte. Fails unless the artifact the page boots is
	/// built, naming the recipe.
	pub async fn serve(store: BlobStore, entry: &str) -> Self {
		let artifact = AbsPath::new_workspace_rel(ARTIFACT).unwrap();
		if !fs_ext::exists(&artifact).unwrap_or_default() {
			panic!("missing artifact `{artifact}`, run `{BUILD_RECIPE}`");
		}
		let page = serve_entry(store.clone(), entry).await;
		let console = page.console().await.unwrap();
		let responses = page.responses().await.unwrap();
		Self {
			page,
			console,
			responses,
			log: String::new(),
			errors: Vec::new(),
			store,
		}
	}

	/// Navigate the tab to `path` on the served app, the log starting over so
	/// [`Self::painted`] reads this load's line and not a previous one's.
	pub async fn goto(&mut self, path: &str) {
		self.drain();
		self.log.clear();
		self.page.goto(path).await.unwrap();
	}

	/// Drain the console into the log, streaming each entry for the person
	/// watching and keeping every error aside.
	pub fn drain(&mut self) {
		for entry in self.console.drain() {
			cross_log!("{}", entry.text);
			if entry.is_error() || entry.text.contains("ERROR") {
				self.errors.push(entry.text.clone());
			}
			self.log.push_str(&entry.text);
			self.log.push('\n');
		}
	}

	/// Drain the console until `needle` appears in the log, failing on a
	/// panic or the deadline; `each` runs between drains, for a case
	/// sampling the page while it boots.
	pub async fn console_until_with(
		&mut self,
		needle: &str,
		mut each: impl AsyncFnMut(&PageHarness),
	) {
		let deadline = Instant::now() + PAINT_DEADLINE;
		while !self.log.contains(needle) {
			self.drain();
			if self.log.contains("panicked") {
				panic!("the browser process panicked. console:\n{}", self.log);
			}
			if Instant::now() > deadline {
				panic!(
					"the console never said `{needle}`. console:\n{}",
					self.log
				);
			}
			each(&self.page).await;
			time_ext::sleep(Duration::from_millis(100)).await;
		}
	}

	/// Drain the console until `needle` appears in the log.
	pub async fn console_until(&mut self, needle: &str) {
		self.console_until_with(needle, async |_| {}).await
	}

	/// Wait for the DOM host to paint `path`, returning the line it logged
	/// with what it adopted (see `DomHost`); `each` samples the page while it
	/// boots.
	pub async fn painted_with(
		&mut self,
		path: &str,
		each: impl AsyncFnMut(&PageHarness),
	) -> String {
		let needle = format!("dom host painted {path} (");
		self.console_until_with(&needle, each).await;
		self.log
			.lines()
			.find(|line| line.contains(&needle))
			.unwrap()
			.to_string()
	}

	/// Wait for the DOM host to paint `path`, returning the line it logged.
	pub async fn painted(&mut self, path: &str) -> String {
		self.painted_with(path, async |_| {}).await
	}

	/// Wait for the DOM host to paint `path` by adopting the served page
	/// whole: nothing replaced, nothing patched, the conformance measure of an
	/// invisible boot.
	pub async fn painted_clean(&mut self, path: &str) {
		self.painted(path)
			.await
			.as_str()
			.xpect_contains("0 replaced, 0 patched");
	}

	/// Close the tab and the served app, failing on anything the console
	/// reported as an error or any response the page had fail: a failed
	/// subresource (a 404 favicon) raises no console error, so a clean boot
	/// needs both empty.
	pub async fn kill(mut self) {
		self.drain();
		self.errors.xpect_empty();
		self.responses
			.drain()
			.into_iter()
			.filter(|response| response.is_error())
			.map(|response| format!("{} {}", response.status, response.url))
			.collect::<Vec<_>>()
			.xpect_empty();
		self.page.kill().await.unwrap();
	}
}

/// Serve the entry at `entry` within `store` as `beet --main` would: the
/// entry resolved in its store, built through the plain one-shot path, its
/// declared `<HttpServer>` booting on an OS port. The launch, read once the
/// first server spawns: only the http server boots (a cli render would exit
/// the app), on an OS port rather than the default so a suite never fights a
/// running dev server.
async fn serve_entry(store: BlobStore, entry: &str) -> PageHarness {
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
