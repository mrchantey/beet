//! The server model: the [`StartServer`] / [`StopServer`] entity events that drive
//! a server's lifecycle, the `ServeOnLoad` markup verb that fires `StartServer` on
//! load, and the [`KeepAlive`] resource that keeps a long-running server's process
//! up.
//!
//! # Boot is a load-lifecycle verb
//!
//! The binary forces no server. An entry declares its transports and a
//! `ServeOnLoad` in a router spread (`<Router {(HttpServer, ServeOnLoad)}>`); on
//! `LoadTemplate`, `ServeOnLoad` triggers a [`StartServer`] built from the entry
//! request, whose filter (`--server`) selects which declared servers boot. A
//! one-shot [`CliServer`] parses argv into a [`Request`], runs one exchange
//! against the router, and exits; a long-running [`HttpServer`] / `TuiServer`
//! inserts [`KeepAlive`] to persist the process.
//!
//! # Server owns its boot
//!
//! Each server component ([`CliServer`], [`HttpServer`], `TuiServer`) registers
//! entity-scoped observers in its `on_add` hook: a [`StartServer`] observer that
//! boots the server when the event's filter matches its name, and a
//! [`StopServer`] observer that tears it down. There is no central orchestrator:
//! a markup-spawned `<Router {(HttpServer{port:0})}>` boots exactly like an
//! explicit trigger, the moment a [`StartServer`] lands on it.
use beet_core::prelude::*;

/// Start the servers on an entity whose name passes the [`filter`](Self::filter).
///
/// Triggered on a host carrying one or more server components; each component's
/// `on_add`-registered observer boots it only if [`passes`](Self::passes)
/// returns `true` for its name (`"cli"`, `"http"`, `"tui"`). An empty/default
/// filter matches every present server.
///
/// [`params`](Self::params) carry the config the server reads at boot (eg
/// `--port`, `--host` for [`HttpServer`]); they flow from whoever triggered the
/// start, never from `CliArgs::parse_env` inside the server.
#[derive(Debug, Clone, EntityEvent, Get)]
pub struct StartServer {
	/// The host entity to start servers on.
	pub entity: Entity,
	/// Selects which servers boot by name; empty matches all present.
	pub filter: GlobFilter,
	/// Boot config (eg `port`, `host`), keyed by name.
	pub params: MultiMap<SmolStr, SmolStr>,
}

impl StartServer {
	/// A start targeting `entity` whose filter matches every present server.
	pub fn all(entity: Entity) -> Self {
		Self {
			entity,
			filter: default(),
			params: default(),
		}
	}

	/// A start targeting `entity` whose filter selects a single server by name.
	pub fn named(entity: Entity, name: &str) -> Self {
		Self {
			entity,
			filter: GlobFilter::default().with_include(name),
			params: default(),
		}
	}

	/// The CLI entrypoint start: targets `entity`, filtered to the `cli` server.
	/// Most hosts carry a single server and trigger [`StartServer::all`]; this
	/// names `cli` explicitly when several servers share a host.
	pub fn cli(entity: Entity) -> Self { Self::named(entity, "cli") }

	/// A start built agnostically from a request: the `--server=` value selects
	/// the servers (comma-separated, empty/absent = all), the rest of the request
	/// `params` flow through as boot config.
	///
	/// This is how the `ServeOnLoad` verb boots an entry's declared servers from
	/// the entry request, without naming any backend itself.
	pub fn from_request(
		entity: Entity,
		server: Option<&str>,
		params: MultiMap<SmolStr, SmolStr>,
	) -> Self {
		let filter = server
			.into_iter()
			.flat_map(|value| value.split(','))
			.map(str::trim)
			.filter(|name| !name.is_empty())
			.fold(GlobFilter::default(), |filter, name| {
				filter.with_include(name)
			});
		Self {
			entity,
			filter,
			params,
		}
	}

	/// Whether a server named `name` should boot for this event. An empty filter
	/// matches all; otherwise the name must pass the [`GlobFilter`].
	pub fn passes(&self, name: &str) -> bool { self.filter.passes(name) }

	/// Sets the boot config params.
	pub fn with_params(mut self, params: MultiMap<SmolStr, SmolStr>) -> Self {
		self.params = params;
		self
	}
}

/// Stop the running servers on an entity whose name passes the
/// [`filter`](Self::filter).
///
/// Targets the host entity and does not descend. An empty/default filter stops
/// every present server. Anyone may trigger it.
#[derive(Debug, Clone, EntityEvent, Get)]
pub struct StopServer {
	/// The host entity to stop servers on.
	pub entity: Entity,
	/// Selects which servers stop by name; empty matches all.
	pub filter: GlobFilter,
}

impl StopServer {
	/// A stop targeting `entity` whose filter matches every present server.
	pub fn all(entity: Entity) -> Self {
		Self {
			entity,
			filter: default(),
		}
	}

	/// Whether a server named `name` should stop for this event.
	pub fn passes(&self, name: &str) -> bool { self.filter.passes(name) }
}

/// Refcounts the live claims keeping the process alive. While the count is above
/// zero the binary does not exit; when the last claim drops and it reaches zero the
/// exit system emits [`AppExit::Success`].
///
/// Claims are taken as [`KeepAliveGuard`] components, not by hand: a long-running
/// [`HttpServer`] / `TuiServer` holds one on its host, a running [`CliServer`]
/// exchange holds one for its duration, and the binary holds one across the build.
/// A refcount, not a unit flag, so sibling servers on one host (the multi-server
/// site) each hold their own: a one-shot `CliServer` dropping its guard never tears
/// down a live `HttpServer`.
#[derive(Debug, Default, Resource)]
pub struct KeepAlive(usize);

impl KeepAlive {
	/// Takes a ref: the process stays alive until it is released. Prefer a
	/// [`KeepAliveGuard`] component, which calls this on insert.
	pub fn acquire(&mut self) { self.0 += 1; }
	/// Drops a ref previously taken with [`acquire`](Self::acquire), saturating at
	/// zero so an extra release can never underflow.
	pub fn release(&mut self) { self.0 = self.0.saturating_sub(1); }
	/// The number of outstanding refs; zero means nothing is keeping the process up.
	pub fn count(&self) -> usize { self.0 }
}

/// A process-lifetime claim held as a component: inserting it acquires a [`KeepAlive`]
/// ref, removing it (or despawning its entity) releases that ref. Every long-running
/// claimant holds one (a booted server on its host, the binary's load scope on a
/// scratch entity), so the accounting is uniform and a despawn can never leak a ref.
/// Removing an absent guard is a no-op, so stop paths are idempotent.
#[derive(Default, Component)]
#[component(on_add = on_guard_add, on_remove = on_guard_remove)]
pub struct KeepAliveGuard;

/// Acquire a [`KeepAlive`] ref when a guard is inserted.
fn on_guard_add(mut world: DeferredWorld, _cx: HookContext) {
	if let Some(mut keep_alive) = world.get_resource_mut::<KeepAlive>() {
		keep_alive.acquire();
	}
}

/// Release the [`KeepAlive`] ref when a guard is removed or its entity despawned.
fn on_guard_remove(mut world: DeferredWorld, _cx: HookContext) {
	if let Some(mut keep_alive) = world.get_resource_mut::<KeepAlive>() {
		keep_alive.release();
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	fn empty_filter_matches_all() {
		let start = StartServer::all(Entity::PLACEHOLDER);
		start.passes("cli").xpect_true();
		start.passes("http").xpect_true();
		start.passes("tui").xpect_true();
	}

	#[beet_core::test]
	fn named_filter_matches_one() {
		let start = StartServer::named(Entity::PLACEHOLDER, "http");
		start.passes("http").xpect_true();
		start.passes("cli").xpect_false();
		start.passes("tui").xpect_false();
	}

	#[beet_core::test]
	fn cli_filter_selects_cli() {
		let start = StartServer::cli(Entity::PLACEHOLDER);
		start.passes("cli").xpect_true();
		start.passes("http").xpect_false();
	}

	#[beet_core::test]
	fn from_request_builds_filter() {
		// no `--server`: matches all present servers.
		let start = StartServer::from_request(
			Entity::PLACEHOLDER,
			None,
			default(),
		);
		start.passes("cli").xpect_true();
		start.passes("http").xpect_true();
		// `--server=http`: only http.
		let start = StartServer::from_request(
			Entity::PLACEHOLDER,
			Some("http"),
			default(),
		);
		start.passes("http").xpect_true();
		start.passes("cli").xpect_false();
		// `--server=http,tui`: either.
		let start = StartServer::from_request(
			Entity::PLACEHOLDER,
			Some("http, tui"),
			default(),
		);
		start.passes("http").xpect_true();
		start.passes("tui").xpect_true();
		start.passes("cli").xpect_false();
	}

	#[beet_core::test]
	fn stop_filter_matches() {
		StopServer::all(Entity::PLACEHOLDER).passes("http").xpect_true();
		let stop = StopServer {
			entity: Entity::PLACEHOLDER,
			filter: GlobFilter::default().with_include("tui"),
		};
		stop.passes("tui").xpect_true();
		stop.passes("http").xpect_false();
	}
}
