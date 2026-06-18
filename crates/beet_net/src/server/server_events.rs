//! The server model: the [`BootServer`] / [`StopServer`] entity events that drive
//! a server's lifecycle, the `StartServer` markup verb that fires `BootServer` on
//! load, and the [`KeepAlive`] resource that keeps a long-running server's process
//! up.
//!
//! # Boot is a load-lifecycle verb
//!
//! The binary forces no server. An entry declares its transports and a
//! `StartServer` in a router spread (`<Router {(HttpServer, StartServer)}>`); on
//! `LoadTemplate`, `StartServer` triggers a [`BootServer`] built from the entry
//! request, whose filter (`--server`) selects which declared servers boot. A
//! one-shot [`CliServer`] parses argv into a [`Request`], runs one exchange
//! against the router, and exits; a long-running [`HttpServer`] / `TuiServer`
//! inserts [`KeepAlive`] to persist the process.
//!
//! # Server owns its boot
//!
//! Each server component ([`CliServer`], [`HttpServer`], `TuiServer`) registers
//! entity-scoped observers in its `on_add` hook: a [`BootServer`] observer that
//! boots the server when the event's filter matches its name, and a
//! [`StopServer`] observer that tears it down. There is no central orchestrator:
//! a markup-spawned `<Router {(HttpServer{port:0})}>` boots exactly like an
//! explicit trigger, the moment a [`BootServer`] lands on it.
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
pub struct BootServer {
	/// The host entity to start servers on.
	pub entity: Entity,
	/// Selects which servers boot by name; empty matches all present.
	pub filter: GlobFilter,
	/// Boot config (eg `port`, `host`), keyed by name.
	pub params: MultiMap<SmolStr, SmolStr>,
}

impl BootServer {
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
	/// Most hosts carry a single server and trigger [`BootServer::all`]; this
	/// names `cli` explicitly when several servers share a host.
	pub fn cli(entity: Entity) -> Self { Self::named(entity, "cli") }

	/// A start built agnostically from a request: the `--server=` value selects
	/// the servers (comma-separated, empty/absent = all), the rest of the request
	/// `params` flow through as boot config.
	///
	/// This is how the `StartServer` verb boots an entry's declared servers from
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

/// When inserted, the process does not emit [`AppExit`] after the entrypoint's
/// one exchange completes, keeping it alive: a long-running server ([`HttpServer`],
/// the `beet_router` `TuiServer`) inserts it on start, and a `--watch` command
/// inserts it so a file watcher keeps firing.
#[derive(Default, Resource)]
pub struct KeepAlive;

#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	fn empty_filter_matches_all() {
		let start = BootServer::all(Entity::PLACEHOLDER);
		start.passes("cli").xpect_true();
		start.passes("http").xpect_true();
		start.passes("tui").xpect_true();
	}

	#[beet_core::test]
	fn named_filter_matches_one() {
		let start = BootServer::named(Entity::PLACEHOLDER, "http");
		start.passes("http").xpect_true();
		start.passes("cli").xpect_false();
		start.passes("tui").xpect_false();
	}

	#[beet_core::test]
	fn cli_filter_selects_cli() {
		let start = BootServer::cli(Entity::PLACEHOLDER);
		start.passes("cli").xpect_true();
		start.passes("http").xpect_false();
	}

	#[beet_core::test]
	fn from_request_builds_filter() {
		// no `--server`: matches all present servers.
		let start = BootServer::from_request(
			Entity::PLACEHOLDER,
			None,
			default(),
		);
		start.passes("cli").xpect_true();
		start.passes("http").xpect_true();
		// `--server=http`: only http.
		let start = BootServer::from_request(
			Entity::PLACEHOLDER,
			Some("http"),
			default(),
		);
		start.passes("http").xpect_true();
		start.passes("cli").xpect_false();
		// `--server=http,tui`: either.
		let start = BootServer::from_request(
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
