//! The unified server model: the [`Server`] orchestrator, the [`ServerBackend`]
//! trait its backends implement, and the [`ServerBackends`] registry that keeps
//! [`beet_net`] ignorant of downstream backends (eg the `beet_router` TUI).
//!
//! # Every binary is a CLI server
//!
//! A formal beet binary boots as a CLI server at the top level: its entrypoint
//! is always a [`CliServer`] that parses argv into a [`Request`] and runs one
//! exchange against the router. A binary may expose a single action, but the
//! way in is always the CLI. Long-running backends ([`HttpServer`], the
//! `beet_router` `TuiServer`) are never self-booting in isolation; they are
//! started *by* the [`Server`] orchestrator, which each backend pulls in
//! declaratively via `#[require(Server)]`. `Server` selects which backends to
//! start, so a markup-spawned `<Router {(HttpServer{port})}>` boots exactly like
//! an explicit spawn.
use crate::prelude::*;
use beet_core::prelude::*;

/// A startable server backend. The orchestrating [`Server`] calls
/// [`start`](ServerBackend::start) on the selected backends; a backend reads its
/// config off the entity inside `start` (matching the [`HttpServerFn`] shape),
/// so selection is by component *presence* rather than a stored handle.
///
/// [`stop`](ServerBackend::stop) defaults to a no-op `Ok`: not every backend can
/// stop (the [`CliServer`] one-shot exits the process), so only backends that
/// own a cancellable listener override it.
pub trait ServerBackend {
	/// Start the backend on `entity`, reading any config (port, host, …) off it.
	/// Runs on the async layer with an [`AsyncEntity`], like the built-in
	/// `start_hyper_server` / `start_mini_http_server` backends.
	fn start(entity: AsyncEntity) -> MaybeSendBoxedFuture<'static, Result>;

	/// Stop a running backend. Defaults to a no-op `Ok` for backends with
	/// nothing to tear down (eg the one-shot [`CliServer`]).
	fn stop(entity: AsyncEntity) -> MaybeSendBoxedFuture<'static, Result> {
		let _ = entity;
		Box::pin(async { Ok(()) })
	}
}

/// The server orchestrator: selects and starts the backends spawned alongside
/// it, by precedence `--server=` param > present backend components > feature
/// default ([`ServerKind::Cli`]).
///
/// Each backend ([`CliServer`], [`HttpServer`], the `beet_router` `TuiServer`)
/// declares `#[require(Server)]`, so spawning one declaratively (eg the BSX
/// `<Router {(HttpServer{port:0})}>`) or explicitly brings `Server` in, and its
/// `on_add` boots the selected backends. When a long-running backend is started
/// (`HttpServer` / `TuiServer`), [`KeepAlive`] is inserted so the process
/// persists; the bare [`CliServer`] runs one exchange and exits.
///
/// [`Server::cli`] pins selection to the CLI entrypoint, used by a binary that
/// is itself the controller (eg the `beet` CLI) so the global `--server` param,
/// meant for a *spawned* long-running backend, never hijacks the entrypoint.
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = on_add)]
pub struct Server {
	/// When set, selection is forced to this kind, ignoring the `--server` param
	/// and present components. `None` resolves by the normal precedence.
	pub pin: Option<ServerKind>,
}

impl Server {
	/// A [`Server`] pinned to the CLI entrypoint: it always runs the one-shot
	/// [`CliServer`], regardless of the `--server` param or present components.
	pub fn cli() -> Self {
		Self {
			pin: Some(ServerKind::Cli),
		}
	}
}

/// When inserted, the process does not emit [`AppExit`] after the entrypoint's
/// one exchange completes, keeping it alive: the [`Server`] orchestrator inserts
/// it when a long-running backend ([`HttpServer`], the `beet_router` `TuiServer`)
/// starts, and a `--watch` command inserts it so a file watcher keeps firing.
#[derive(Default, Resource)]
pub struct KeepAlive;

/// A selectable server backend kind, the vocabulary of the `--server=` param.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum ServerKind {
	/// The one-shot entrypoint: parse argv, run one exchange, exit.
	#[default]
	Cli,
	/// A long-running HTTP listener.
	Http,
	/// A long-running interactive terminal app.
	Tui,
}

impl ServerKind {
	/// Parse a single `--server` token, eg `http`.
	pub fn parse(token: &str) -> Result<Self> {
		match token.trim().to_lowercase().as_str() {
			"cli" => Self::Cli.xok(),
			"http" => Self::Http.xok(),
			"tui" => Self::Tui.xok(),
			other => {
				bevybail!("invalid --server '{other}', expected cli, http or tui")
			}
		}
	}

	/// Parse a comma-separated `--server` value, eg `http,tui`.
	pub fn parse_list(value: &str) -> Result<Vec<Self>> {
		value
			.split(',')
			.filter(|token| !token.trim().is_empty())
			.map(Self::parse)
			.collect()
	}

	/// Long-running backends keep the process alive; the CLI one-shot does not.
	pub fn is_long_running(&self) -> bool {
		matches!(self, Self::Http | Self::Tui)
	}
}

/// A registered backend: how to detect its presence on an entity and how to
/// start it, keeping the orchestrator agnostic of backends defined downstream.
#[derive(Clone)]
pub struct ServerBackendEntry {
	/// Returns `true` if this backend's component is present on the entity.
	pub is_present: fn(&EntityRef) -> bool,
	/// Starts the backend on the entity (the [`ServerBackend::start`] shape).
	pub start: HttpServerFn,
}

/// Maps each [`ServerKind`] to how it is detected and started. [`beet_net`]
/// registers [`ServerKind::Cli`] and [`ServerKind::Http`]; downstream crates
/// register their own (eg `beet_router` registers [`ServerKind::Tui`]), so the
/// orchestrator never names a backend it does not own.
#[derive(Default, Resource)]
pub struct ServerBackends(HashMap<ServerKind, ServerBackendEntry>);

impl ServerBackends {
	/// Register a backend kind. The last registration for a kind wins, so a
	/// downstream crate can override a built-in backend.
	pub fn register(&mut self, kind: ServerKind, entry: ServerBackendEntry) {
		self.0.insert(kind, entry);
	}
	/// The entry for a kind, if registered.
	pub fn get(&self, kind: ServerKind) -> Option<&ServerBackendEntry> {
		self.0.get(&kind)
	}
}

fn on_add(mut world: DeferredWorld, cx: HookContext) {
	// queue_async_local hosts both the Send (cli/tui) and non-Send (hyper)
	// backend futures; the orchestrator boots each selected backend in turn.
	world
		.commands()
		.entity(cx.entity)
		.queue_async_local(start_selected);
}

/// Boot the backends selected for `entity`: resolve the selection against the
/// [`ServerBackends`] registry, call each `start`, and insert [`KeepAlive`] if a
/// long-running backend started.
async fn start_selected(entity: AsyncEntity) -> Result {
	// a briefly-spawned server (eg during scene serialization) has no business
	// booting; the entity is already gone by the time this runs.
	if !entity.is_alive().await {
		return Ok(());
	}

	// resolve the selection and the matching start fns inside one world access,
	// so presence is read against the live entity.
	let selected = entity
		.with_world(|world, entity| -> Result<Vec<(ServerKind, HttpServerFn)>> {
			let backends =
				world.get_resource::<ServerBackends>().ok_or_else(|| {
					bevyhow!(
						"ServerBackends registry missing; add the ServerPlugin"
					)
				})?;
			let entity_ref = world.entity(entity);
			ServerSelection::resolve(&entity_ref, backends)?
				.0
				.into_iter()
				.map(|kind| {
					backends
						.get(kind)
						.map(|entry| (kind, entry.start))
						.ok_or_else(|| {
							bevyhow!("no backend registered for {kind:?}")
						})
				})
				.collect()
		})
		.await??;

	// a long-running backend keeps the process alive; insert `KeepAlive` up
	// front so the CLI entrypoint (started last) sees it and does not exit.
	if selected.iter().any(|(kind, _)| kind.is_long_running()) {
		entity
			.world()
			.with(|world: &mut World| world.insert_resource(KeepAlive))
			.await;
	}

	for (_kind, start) in selected {
		start(entity.clone()).await?;
	}
	Ok(())
}

/// The resolved list of backends to start, in order. The entrypoint
/// [`ServerKind::Cli`] is started last so its one exchange runs against any
/// long-running backend already up.
pub struct ServerSelection(pub Vec<ServerKind>);

impl ServerSelection {
	/// Resolve which backends to start for a [`Server`] entity, by precedence:
	/// a [`Server::pin`], else the `--server=` argv param, else the registered
	/// backends present on the entity, else the feature default
	/// ([`ServerKind::Cli`]).
	pub fn resolve(
		entity: &EntityRef,
		backends: &ServerBackends,
	) -> Result<Self> {
		// 0. a pinned `Server` (eg the `beet` CLI entrypoint) forces its kind,
		// so the global `--server` param cannot hijack it.
		if let Some(kind) = entity.get::<Server>().and_then(|server| server.pin) {
			return Self(vec![kind]).xok();
		}
		// 1. explicit `--server=http,tui` wins (std only: no_std has no argv).
		#[cfg(feature = "std")]
		if let Some(value) = CliArgs::parse_env().params.get("server") {
			return Self(ServerKind::parse_list(value)?).xok();
		}
		// 2. else the registered backends present on the entity, the CLI
		// entrypoint last so it exchanges against a live long-running server.
		let mut present = [ServerKind::Http, ServerKind::Tui, ServerKind::Cli]
			.into_iter()
			.filter(|kind| {
				backends
					.get(*kind)
					.is_some_and(|entry| (entry.is_present)(entity))
			})
			.collect::<Vec<_>>();
		if present.is_empty() {
			// 3. feature default: the one-shot CLI entrypoint.
			present.push(ServerKind::Cli);
		}
		Self(present).xok()
	}
}

/// Resolve a value by precedence `param > component field > default`, the shape
/// shared by every server config knob (port, host, accept, color-scheme).
///
/// `param` is the parsed `--name=` argv value (already `Option`), `field` the
/// value read off the backend component (already `Option`), `default` the
/// fallback. Returns the first present in that order.
pub fn resolve_config<T>(param: Option<T>, field: Option<T>, default: T) -> T {
	param.or(field).unwrap_or(default)
}

#[cfg(test)]
mod test {
	use super::*;

	fn backends() -> ServerBackends {
		let mut backends = ServerBackends::default();
		backends.register(ServerKind::Cli, ServerBackendEntry {
			is_present: |entity| entity.contains::<CliServer>(),
			start: |_| Box::pin(async { Ok(()) }),
		});
		backends.register(ServerKind::Http, ServerBackendEntry {
			is_present: |entity| entity.contains::<HttpServer>(),
			start: |_| Box::pin(async { Ok(()) }),
		});
		backends
	}

	#[beet_core::test]
	fn parses_server_kinds() {
		ServerKind::parse("http").unwrap().xpect_eq(ServerKind::Http);
		ServerKind::parse(" TUI ").unwrap().xpect_eq(ServerKind::Tui);
		ServerKind::parse("nope").xpect_err();
		ServerKind::parse_list("http,tui")
			.unwrap()
			.xpect_eq(vec![ServerKind::Http, ServerKind::Tui]);
	}

	#[beet_core::test]
	fn long_running_kinds() {
		ServerKind::Http.is_long_running().xpect_true();
		ServerKind::Tui.is_long_running().xpect_true();
		ServerKind::Cli.is_long_running().xpect_false();
	}

	#[beet_core::test]
	fn resolves_config_by_precedence() {
		resolve_config(Some(1), Some(2), 3).xpect_eq(1);
		resolve_config(None, Some(2), 3).xpect_eq(2);
		resolve_config(None, None, 3).xpect_eq(3);
	}

	/// Resolve the selection for `bundle` spawned in an async world (so the
	/// `Server` require's queued boot has its runtime, though it never runs here).
	fn select(bundle: impl Bundle) -> Vec<ServerKind> {
		let mut world = AsyncPlugin::world();
		let entity = world.spawn(bundle).id();
		world.flush();
		ServerSelection::resolve(&world.entity(entity), &backends())
			.unwrap()
			.0
	}

	/// With no `--server` param, selection falls to the present backends: an
	/// `HttpServer` is selected, the CLI entrypoint ordered last when present.
	#[beet_core::test]
	fn selects_present_backends() {
		select(HttpServer::new(0)).xpect_eq(vec![ServerKind::Http]);
		select((HttpServer::new(0), CliServer))
			.xpect_eq(vec![ServerKind::Http, ServerKind::Cli]);
	}

	/// No backend present, no param: the feature default is the CLI one-shot.
	#[beet_core::test]
	fn selects_default_cli() {
		select(()).xpect_eq(vec![ServerKind::Cli]);
	}

	/// A pinned `Server` forces its kind even with a long-running backend present
	/// (the `beet` CLI entrypoint, which must stay CLI).
	#[beet_core::test]
	fn pin_overrides_presence() {
		select((HttpServer::new(0), Server::cli()))
			.xpect_eq(vec![ServerKind::Cli]);
	}
}
