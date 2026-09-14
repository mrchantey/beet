//! Define a layout, serve it, drive it: the cypress/playwright loop as one
//! value.
//!
//! [`PageHarness`] serves any exchange-shaped bundle (a `Router` subtree, a
//! plain handler) on an OS-assigned port in a background app, opens a headless
//! [`Browser`] beside it, and [`Deref`]s to it so the whole webdriver surface —
//! auto-waiting finds, trusted input, async matchers, collectors — runs
//! against the served page:
//!
//! ```no_run
//! # use beet_core::prelude::*;
//! # use beet_net::prelude::*;
//! # use beet_net::prelude::webdriver::*;
//! # async fn demo() -> Result {
//! let mut page = PageHarness::visit(
//! 	(),
//! 	exchange_ext::handler(|_| {
//! 		Response::ok_body("<button id=\"go\">go</button>", MediaType::Html)
//! 	}),
//! 	"/",
//! )
//! .await?;
//! page.find("#go").await.click().await.unwrap();
//! page.kill().await?;
//! # Ok(())
//! # }
//! ```
//!
//! The served app lives on its own thread with the harness baking in
//! `MinimalPlugins` + [`ServerPlugin`]; `plugins` carries only the extras the
//! layout needs (eg `RouterPlugin`, a style plugin, an
//! `|app: &mut App|` closure seeding resources). The listener is pre-bound to
//! port 0 (no port race, parallel-test safe) and the driver rides
//! [`Client::unique`], so harnesses stack freely within one test binary.
//!
//! A tree that boots its own listener (a no-code entry declaring
//! `<HttpServer>`) goes through [`PageHarness::serve_app`] instead: the harness
//! learns the url from the first [`Listening`] the served app records.

use super::*;
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::app::Plugins;
use core::net::SocketAddr;

/// How long [`PageHarness::serve_app`] waits for the served app's listener,
/// generous for a debug-build entry that scans its routes before binding.
const LISTENER_DEADLINE: Duration = Duration::from_secs(30);

/// One served beet app plus one driven browser: serve a bundle, visit it,
/// assert on the live page. See the [module docs](self).
pub struct PageHarness {
	browser: Browser,
	/// The served app's base url, `http://127.0.0.1:{port}`.
	url: String,
	/// Flags the served app to write `AppExit` on its next frame.
	exit: Store<bool>,
	/// The served app's thread, joined on [`Self::kill`].
	thread: std::thread::JoinHandle<AppExit>,
}

impl PageHarness {
	/// Serve `host` as the dispatch child of a port-0 [`HttpServer`] in a
	/// background app, and open a headless browser page (not yet navigated, so
	/// collectors can attach first).
	///
	/// `plugins` extends the baked-in `MinimalPlugins` + [`ServerPlugin`], eg
	/// `RouterPlugin` plus a style plugin for a full document layout.
	pub async fn serve<M>(
		plugins: impl 'static + Send + Plugins<M>,
		host: impl Bundle,
	) -> Result<Self> {
		let (mut server, on_spawn) =
			HttpServer::new_test(HttpServer::start_mini_with_tcp);
		// leave the process-global loopback port to the app under test: several
		// harnesses may serve concurrently in one test binary.
		server.canonical = false;
		Self::serve_app(move |app| {
			app.add_plugins(plugins);
			// the server owns the boot, its dispatch host is the child
			app.world_mut().spawn((server, on_spawn, children![host]));
		})
		.await
	}

	/// Serve an app of the caller's own making: `setup` runs on the harness
	/// thread against the baked-in app (`MinimalPlugins` paced + [`ServerPlugin`])
	/// and boots a listener however it likes; the harness reads the url off the
	/// first [`Listening`] the app records, awaited before the browser opens.
	/// The shape behind [`Self::serve`]; reach for it when the served tree boots
	/// its own server, eg a no-code entry declaring `<HttpServer port=0>`.
	pub async fn serve_app(
		setup: impl 'static + Send + FnOnce(&mut App),
	) -> Result<Self> {
		let exit = Store::new(false);
		let listening = Store::<Option<SocketAddr>>::default();
		let thread = std::thread::spawn({
			let listening = listening.clone();
			move || {
				let mut app = App::new();
				app.add_plugins(MinimalPlugins.set(
					// a paced loop rather than the default spin: the app only relays
					// requests, so a millisecond cadence costs latency nobody notices
					// and spares a test-suite of harnesses burning cores.
					bevy::app::ScheduleRunnerPlugin::run_loop(
						Duration::from_millis(1),
					),
				))
				.add_plugins(ServerPlugin)
				.add_systems(
					bevy::app::Update,
					move |mut writer: MessageWriter<AppExit>| {
						if exit.get() {
							writer.write(AppExit::Success);
						}
					},
				)
				// the first bind is the harness url; a rebind (a live reload
				// rebuild on a declared port) lands on the same address.
				.add_systems(
					bevy::app::Update,
					move |bound: Populated<&Listening, Added<Listening>>| {
						if listening.get().is_none() {
							listening.set(Some(bound.iter().next().unwrap().0));
						}
					},
				);
				setup(&mut app);
				app.run()
			}
		});
		let url = Self::await_listening(&listening).await?.local_url();
		let browser = Browser::new_with(Client::unique()).await?;
		Self {
			browser,
			url,
			exit,
			thread,
		}
		.xok()
	}

	/// Wait for the served app to record its first [`Listening`]: a pre-bound
	/// listener answers at once, a tree booting its own server on its `Ready` a
	/// little later (its whole entry build first).
	async fn await_listening(
		listening: &Store<Option<SocketAddr>>,
	) -> Result<Listening> {
		poll_ext::poll_async_with(
			async || {
				listening
					.get()
					.map(Listening)
					.ok_or_else(|| bevyhow!("the served app has not bound yet"))
			},
			LISTENER_DEADLINE,
			poll_ext::DEFAULT_INTERVAL,
		)
		.await
	}

	/// [`Self::serve`] then [`Self::goto`] `path`: the one-line entry for tests
	/// that need no pre-navigation setup.
	pub async fn visit<M>(
		plugins: impl 'static + Send + Plugins<M>,
		host: impl Bundle,
		path: &str,
	) -> Result<Self> {
		let mut harness = Self::serve(plugins, host).await?;
		harness.goto(path).await?;
		Ok(harness)
	}

	/// Navigate the page to `path` on the served app.
	pub async fn goto(&mut self, path: &str) -> Result<()> {
		let url = self.route_url(path);
		self.browser.navigate(&url).await
	}

	/// The served app's base url, eg `http://127.0.0.1:41234`.
	pub fn url(&self) -> &str { &self.url }

	/// The absolute url of `path` on the served app.
	pub fn route_url(&self, path: &str) -> String {
		format!("{}/{}", self.url, path.trim_start_matches('/'))
	}

	/// Close the browser (driver process included), stop the served app and
	/// join its thread. A dropped harness instead leaks the app thread until
	/// process exit (the browser still dies with its `kill_on_drop` child).
	pub async fn kill(self) -> Result<()> {
		let Self {
			browser,
			exit,
			thread,
			..
		} = self;
		browser.kill().await?;
		exit.set(true);
		let app_exit = thread
			.join()
			.map_err(|_| bevyhow!("the served app thread panicked"))?;
		exit.remove();
		match app_exit {
			AppExit::Success => Ok(()),
			AppExit::Error(code) => {
				bevybail!("the served app exited with code {code}")
			}
		}
	}
}

impl core::ops::Deref for PageHarness {
	type Target = Browser;
	fn deref(&self) -> &Self::Target { &self.browser }
}

impl core::ops::DerefMut for PageHarness {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.browser }
}

#[cfg(test)]
mod test {
	use super::*;

	/// End to end through the harness: serve a handler bundle, drive the page
	/// it returns with a trusted click, and assert the patched DOM — then tear
	/// the whole rig down, proving the served port closes.
	#[beet_core::test(timeout_ms = 30_000)]
	#[ignore = "smoketest"]
	async fn serves_and_drives() {
		let page = PageHarness::visit(
			(),
			exchange_ext::handler(|_| {
				Response::ok_body(
					r#"<html><body>
					<p id="count">0</p>
					<button id="inc" onclick="count.textContent = Number(count.textContent) + 1">+</button>
					</body></html>"#,
					MediaType::Html,
				)
			}),
			"/",
		)
		.await
		.unwrap();

		let url = page.url().to_string();
		page.find("#count").await.xpect_text("0").await;
		page.find("#inc").await.click().await.unwrap();
		page.find("#count").await.xpect_text("1").await;

		page.kill().await.unwrap();
		// the served app is down: a fresh request is refused
		Request::get(&url).send().await.xpect_err();
	}
}
