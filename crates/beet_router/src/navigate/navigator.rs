use crate::prelude::*;
use alloc::borrow::Cow;
use alloc::collections::VecDeque;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Maximum number of history entries to retain.
const HISTORY_LIMIT: usize = 100;

const DEFAULT_HOME: &str = "about:blank";

/// How a [`Navigator`] request travels to reach a page.
///
/// Two independent transports, chosen at construction: a real network fetch for
/// remote URLs, or an in-world dispatch to a local router entity for browsing the
/// app's own routes without a socket. The transport decides only *how the request
/// travels*, not what becomes of the result (that fork lives in the render step).
#[derive(Debug, Clone, Default)]
pub enum NavigatorTransport {
	/// Normal network fetch via [`Request::send`], for remote URLs.
	#[default]
	Http,
	/// Dispatch the path straight to a local `router` entity in-world, no socket.
	/// The live TUI browsing its own site uses this.
	InWorld {
		/// The router entity requests are dispatched to.
		router: Entity,
	},
}

/// A browser-style navigation component that manages page history and fetches
/// pages through its [`NavigatorTransport`].
///
/// History works like a browser: navigating to a new URL truncates any
/// forward entries and appends the new URL.  [`Navigator::back`] and
/// [`Navigator::forward`] move through that stack without making new
/// requests unless the cursor actually moves.
#[derive(Debug, Clone, Component)]
#[component(on_add = on_add)]
pub struct Navigator {
	user_agent: Cow<'static, str>,
	/// Media types accepted by this navigator, in preference order.
	accepts: Vec<MediaType>,
	/// How requests travel: a network fetch, or in-world router dispatch.
	transport: NavigatorTransport,
	/// `true` while a request is in-flight.
	loading: bool,
	home_url: Url,
	/// Visited URLs, oldest first.
	history: VecDeque<Url>,
	/// Index into `history` for the currently displayed page.
	history_cursor: usize,
}

fn on_add(mut world: DeferredWorld, cx: HookContext) {
	world
		.commands()
		.entity(cx.entity)
		.queue_async(async |entity| {
			let navigator = entity.id();
			// a session can close (despawning its co-located navigator) before this
			// boot task runs or between its await points, eg a multi-tenant SSH
			// client that connects and immediately disconnects. A despawned entity is
			// a clean exit, not an error to surface to the command error handler.
			let Ok(home_url) =
				entity.get(|nav: &Navigator| nav.home_url.clone()).await
			else {
				return Ok(());
			};
			// world handle kept before `navigate_to` consumes `entity`, for the
			// error-page render below.
			let world = entity.world().clone();
			// a failed home load (eg network down) renders a user-facing error
			// page in place of the missing page rather than crashing the host or
			// leaving it blank, while still logging the cause.
			if let Err(err) = Navigator::navigate_to(entity, home_url).await {
				error!("Navigator failed to load home page: {err}");
				let message = err.to_string();
				world
					.with(move |world| match PageHost::of(world, navigator) {
						Some(host) => set_error_page(world, host, message),
						None => error!(
							"navigator {navigator} has no page host for the error page"
						),
					})
					.await;
			}
			Ok(())
		});
}

impl Default for Navigator {
	fn default() -> Self {
		let home = Url::parse(DEFAULT_HOME);
		// let mut history = VecDeque::new();
		// history.push_back(home.clone());
		Self {
			// user_agent: "curl/8.17.0".into(),
			user_agent: "Mozilla/5.0 Beet/0.1".into(),
			home_url: home.clone(),
			accepts: vec![
				// prefer markdown — beet skips scripts/styles so less content
				// means faster rendering
				MediaType::Markdown,
				MediaType::Html,
				// MediaType::Markdown,
				// MediaType::other("*/*"),
			],
			transport: NavigatorTransport::Http,
			loading: false,
			// home navigated to by on_add
			history: default(),
			history_cursor: 0,
		}
	}
}

impl Navigator {
	pub fn new(home_url: impl Into<Url>) -> Self {
		Self {
			home_url: home_url.into(),
			..default()
		}
	}

	/// An in-world navigator: requests dispatch to the local `router` entity
	/// (no socket), for browsing the app's own routes. Starts at `home_url`.
	///
	/// Accepts terminal media types (no HTML), so a layout's content negotiation
	/// treats it as the non-web target: the document chrome seeds the dark scheme
	/// itself rather than relying on the web `color-scheme` script.
	pub fn in_world(router: Entity, home_url: impl Into<Url>) -> Self {
		Self {
			home_url: home_url.into(),
			transport: NavigatorTransport::InWorld { router },
			accepts: vec![
				MediaType::AnsiTerm,
				MediaType::Text,
				MediaType::Markdown,
			],
			..default()
		}
	}

	/// The transport this navigator uses to reach pages.
	pub fn transport(&self) -> &NavigatorTransport { &self.transport }

	/// The URL currently being displayed (or loading).
	pub fn current_url(&self) -> &Url {
		// if history is empty (eg before on_add runs), fall back to home_url
		self.history
			.get(self.history_cursor)
			.unwrap_or(&self.home_url)
	}

	pub fn is_loading(&self) -> bool { self.loading }

	/// `true` when there is at least one entry behind the cursor.
	pub fn can_go_back(&self) -> bool { self.history_cursor > 0 }

	/// `true` when there is at least one entry ahead of the cursor.
	pub fn can_go_forward(&self) -> bool {
		self.history_cursor + 1 < self.history.len()
	}

	/// Resolve `url` against the current page, handling relative paths.
	fn resolve(&self, url: Url) -> Url { self.current_url().join(url) }

	/// Push a resolved URL onto the history stack, truncating any forward
	/// entries and enforcing [`HISTORY_LIMIT`].
	fn push_history(&mut self, url: Url) {
		// drop forward entries
		self.history.truncate(self.history_cursor + 1);
		self.history.push_back(url);
		// evict oldest entries when the limit is exceeded
		while self.history.len() > HISTORY_LIMIT {
			self.history.pop_front();
		}
		self.history_cursor = self.history.len() - 1;
	}

	/// Navigate to `url`, pushing it onto the history stack.
	///
	/// ## Errors
	/// - No [`Navigator`] found on entity
	/// - Network request failed
	/// - No [`RenderTargets`] found
	pub async fn navigate_to(
		entity: AsyncEntity,
		url: impl Into<Url>,
	) -> Result {
		let url = url.into();
		// resolve relative url and push history
		let (transport, user_agent, resolved, accepts) = entity
			.get_mut(move |mut nav: Mut<Navigator>| {
				nav.loading = true;
				let resolved = nav.resolve(url);
				nav.push_history(resolved.clone());
				(
					nav.transport.clone(),
					nav.user_agent.clone(),
					resolved,
					nav.accepts.clone(),
				)
			})
			.await?;

		Self::fetch_and_render(entity, transport, user_agent, resolved, accepts)
			.await
	}

	/// Navigate one step back in history, if possible.
	pub async fn back(entity: AsyncEntity) -> Result {
		let nav_state = entity
			.get_mut(|mut nav: Mut<Navigator>| {
				if !nav.can_go_back() {
					return None;
				}
				nav.history_cursor -= 1;
				nav.loading = true;
				Some((
					nav.transport.clone(),
					nav.user_agent.clone(),
					nav.current_url().clone(),
					nav.accepts.clone(),
				))
			})
			.await?;

		if let Some((transport, user_agent, url, accepts)) = nav_state {
			Self::fetch_and_render(entity, transport, user_agent, url, accepts)
				.await?;
		}
		Ok(())
	}

	/// Navigate one step forward in history, if possible.
	pub async fn forward(entity: AsyncEntity) -> Result {
		let nav_state = entity
			.get_mut(|mut nav: Mut<Navigator>| {
				if !nav.can_go_forward() {
					return None;
				}
				nav.history_cursor += 1;
				nav.loading = true;
				Some((
					nav.transport.clone(),
					nav.user_agent.clone(),
					nav.current_url().clone(),
					nav.accepts.clone(),
				))
			})
			.await?;

		if let Some((transport, user_agent, url, accepts)) = nav_state {
			Self::fetch_and_render(entity, transport, user_agent, url, accepts)
				.await?;
		}
		Ok(())
	}

	/// Re-fetch and re-render the current page without touching history, ie the
	/// browser's refresh.
	///
	/// Dev-mode live reload drives this on the in-world TUI navigator after a
	/// watched edit respawns the routes: re-running the current URL through
	/// [`build_live_page`] rebuilds the page from the fresh route tree and the
	/// page host repaints, so the terminal updates live (the web client reloads via
	/// its own [`ClientIo`](crate::prelude::ClientIo) broadcast instead).
	pub async fn reload(entity: AsyncEntity) -> Result {
		let (transport, user_agent, url, accepts) = entity
			.get_mut(|mut nav: Mut<Navigator>| {
				nav.loading = true;
				(
					nav.transport.clone(),
					nav.user_agent.clone(),
					nav.current_url().clone(),
					nav.accepts.clone(),
				)
			})
			.await?;
		Self::fetch_and_render(entity, transport, user_agent, url, accepts)
			.await
	}

	/// Shared fetch → render → clear-loading path used by all navigation
	/// methods.
	///
	/// The transport decides how the request travels (network vs in-world
	/// router dispatch); both end by binding a living page tree to the navigator's
	/// surface, which the page host paints. The static serialize-and-despawn path
	/// is untouched.
	async fn fetch_and_render(
		entity: AsyncEntity,
		transport: NavigatorTransport,
		user_agent: Cow<'static, str>,
		url: Url,
		accepts: Vec<MediaType>,
	) -> Result {
		// `about:` urls (eg the default `about:blank` home) are empty documents:
		// render nothing without touching the network or router.
		if url.scheme() == &Scheme::About {
			let page = entity
				.world()
				.with(|world| {
					parse_page(
						world,
						MediaBytes::new(MediaType::Text, Vec::new()),
					)
				})
				.await?;
			Self::bind_page(&entity, page).await;
			return entity
				.get_mut(|mut nav: Mut<Navigator>| nav.loading = false)
				.await;
		}

		let page = match transport {
			NavigatorTransport::Http => {
				// a real network fetch, then parse the bytes into a living tree
				let bytes = Self::http_fetch(user_agent, url, accepts).await?;
				entity
					.world()
					.with(move |world| parse_page(world, bytes))
					.await?
			}
			NavigatorTransport::InWorld { router } => {
				// dispatch in-world to the local router, keeping the built tree
				let request = Request::get(&url)
					.with_header::<header::UserAgent>(user_agent)
					.with_header::<header::Accept>(accepts);
				build_live_page(&entity.world().entity(router), request).await?
			}
		};

		// bind the new tree to this navigator's surface (the host repaints) and
		// clear loading
		Self::bind_page(&entity, page).await;
		entity
			.get_mut(|mut nav: Mut<Navigator>| nav.loading = false)
			.await?;
		Ok(())
	}

	/// Bind `page` to this navigator's surface (its [`PageHost`], resolved
	/// structurally from the navigator entity), logging if it has none.
	async fn bind_page(entity: &AsyncEntity, page: Entity) {
		let navigator = entity.id();
		entity
			.world()
			.with(move |world| match PageHost::of(world, navigator) {
				Some(host) => bind_surface_page(world, host, page),
				None => {
					error!(
						"navigator {navigator} has no page host to render into"
					)
				}
			})
			.await;
	}

	/// Fetch the page at `url` over the network, returning its bytes.
	///
	/// A pure data fetch with no UI side effect; a 404 renders its error body
	/// rather than bailing.
	async fn http_fetch(
		user_agent: Cow<'static, str>,
		url: Url,
		accepts: Vec<MediaType>,
	) -> Result<MediaBytes> {
		Request::get(&url)
			.with_header::<header::UserAgent>(user_agent)
			.with_header::<header::Accept>(accepts)
			.send()
			.await
			.unwrap_or_else(|err| {
				let mut err = err.to_string();
				if err.is_empty() {
					err = "unknown error".to_string();
				}
				Response::from_status_body(
					StatusCode::BAD_REQUEST,
					err,
					MediaType::Text,
				)
			})
			.into_media_bytes()
			.await
	}
}

/// The route a freshly-opened TUI surface navigates to, recorded on the router by
/// a starting TUI server.
///
/// The one opening-route mechanism both TUI servers read: the local [`TuiServer`]
/// (one stdio surface) and the multi-tenant SSH server (one surface per
/// connection) each store this on their router from the boot request, then read
/// it back when they spawn a surface's navigator. The SSH boot is two-phase (the
/// boot fans out once, but connections are set up later by an observer that cannot
/// see it), so the route must live on the router rather than be re-derived per
/// connection.
#[derive(Debug, Clone, Component)]
pub struct OpeningRoute(pub Url);

impl OpeningRoute {
	/// The opening route from the boot request: an explicit `--path` param (eg
	/// `beet serve <dir> --server=tui --path=docs/form`), else the request path (a
	/// compiled binary's own args, eg a deployed site opening at its home route).
	pub fn from_request(request: &Request) -> Self {
		let url = match request.get_param("path") {
			Some(path) => Url::parse(path),
			None => Url::parse(request.path_string()),
		};
		Self(url)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A default navigator uses the [`NavigatorTransport::Http`] transport.
	#[beet_core::test]
	fn defaults_to_http_transport() {
		matches!(Navigator::default().transport(), NavigatorTransport::Http)
			.xpect_true();
	}
}
