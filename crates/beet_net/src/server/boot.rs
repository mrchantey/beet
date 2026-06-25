//! Turning the process request into a run: the load verbs that call an entry's
//! action, and the [`AppExit`] writers. The [`Boot`] exchange newtype itself and
//! the [`request_selects_server`] predicate are the no_std core, in
//! [`boot_exchange`](super::boot_exchange).
//!
//! [`BootOnLoad`], spread on a server entry root, observes its `LoadTemplate`
//! and calls the entry's `Action<Boot, Response>` boot slot with `Boot(request)`
//! (via [`Boot::boot`]). That slot (a server-provided `ContinueRun<Boot, Response>`)
//! parks on a [`Running<Response>`] keep-alive and fires a `StartRunning<Boot>` the
//! servers observe. A one-shot [`CliServer`] resolves the call (its response streams
//! to stdout and the process exits); a long-running [`HttpServer`] parks the call,
//! holding the process up until its `Running` is removed.
//!
//! [`ExchangeOnLoad`] is the plain counterpart for entries with no boot
//! machinery (eg an `ExchangeScriptElement`): via [`Boot::exchange_boot`] it calls
//! the entry's `Action<Request, Response>` slot directly and streams the response.
//!
//! This is the one path that reads `CliArgs::parse_env()` and writes `AppExit`;
//! the servers are handed the request, never re-parse argv.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

impl Boot {
	/// Call the host's boot slot (`Action<Boot, Response>`) with `Boot(request)`
	/// and, for the one-shot it resolves, stream the response and write the
	/// [`AppExit`].
	///
	/// A long-running server's boot slot never resolves the call, so the await
	/// parks here and the process stays up; a one-shot [`CliServer`] resolves it,
	/// streams, and exits. Used by [`BootOnLoad`].
	pub async fn boot(host: AsyncEntity, request: Request) -> Result {
		let response = host.call::<Boot, Response>(Boot(request)).await?;
		// reached only for the one-shot; a long-running server parks the await.
		stream_and_exit(&host, response).await
	}

	/// Call the host's `Action<Request, Response>` slot directly and stream the
	/// one-shot response. The plain counterpart to [`Boot::boot`] for entries with
	/// no server/boot machinery, eg an `ExchangeScriptElement`. Used by
	/// [`ExchangeOnLoad`].
	pub async fn exchange_boot(host: AsyncEntity, request: Request) -> Result {
		let response = host.call::<Request, Response>(request).await?;
		stream_and_exit(&host, response).await
	}

	/// Run the host's `Action<(), Outcome>` (a behaviour, eg a `Sequence`) and, once
	/// it resolves, write [`AppExit::Success`]. The behaviour counterpart of
	/// [`Boot::boot`]/[`Boot::exchange_boot`]: it dispatches the entry's own slot
	/// rather than a server's. A long-running behaviour (eg a `Repeat`, or a windowed
	/// render scene) never resolves, so the await parks here and the process stays up;
	/// a one-shot behaviour resolves and the process exits. Used by [`RunOnLoad`].
	pub async fn run(host: AsyncEntity) -> Result {
		host.call::<(), Outcome>(()).await?;
		// reached only for a one-shot behaviour; a long-running one parks the await.
		host.world().write_message(AppExit::Success).await;
		Ok(())
	}
}

/// Load verb for a server entry: on its `LoadTemplate`, calls the entity's
/// `Action<Boot, Response>` boot slot with the process request (via [`Boot::boot`]).
///
/// Spread on the entry root alongside the servers that fill the boot slot:
///
/// ```bsx
/// <Router {(HttpServer, CliServer, BootOnLoad)}>
/// ```
///
/// `on_add` registers the `LoadTemplate` observer on the marked entity, so it must
/// sit on the entry root where `LoadTemplate` fires once the whole subtree is
/// built. A failed build exits with an error and never runs; a [`DisableBootOnLoad`]
/// on the entity or an ancestor opts the subtree out (eg a render/inspect build).
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = on_add_boot)]
pub struct BootOnLoad;

/// Load verb for a plain `Request -> Response` entry with no boot machinery (eg
/// an `ExchangeScriptElement`): on its `LoadTemplate`, calls the entity's own
/// `Action<Request, Response>` slot directly (via [`Boot::exchange_boot`]) and
/// streams the one-shot response.
///
/// Identical to [`BootOnLoad`] except for the slot it calls; kept as a separate
/// type so the two load verbs read side by side.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = on_add_exchange)]
pub struct ExchangeOnLoad;

/// Load verb for a behaviour entry with no server/request slot: on its
/// `LoadTemplate`, runs the entity's own `Action<(), Outcome>` (eg a `Sequence`)
/// via [`Boot::run`], exiting once it resolves.
///
/// The behaviour-tree member of the load-verb family ([`BootOnLoad`] calls an
/// `Action<Boot, Response>` slot, [`ExchangeOnLoad`] an `Action<Request, Response>`
/// slot, this the entry's own behaviour). Sits on any node whose subtree should
/// start on load, eg a render scene's behaviour-tree root or a headless one-shot.
/// A long-running behaviour (a `Repeat`, a windowed scene) never resolves, so the
/// process stays up; a one-shot resolves and exits. A [`DisableBootOnLoad`] on the
/// entity or an ancestor opts the subtree out (eg a render/inspect build).
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = on_add_run)]
pub struct RunOnLoad;

/// Opts an entity (and its subtree) out of [`BootOnLoad`] / [`ExchangeOnLoad`] /
/// [`RunOnLoad`]:
/// the tree is built to render or inspect (eg `export-static`, `check`), not to
/// run. A component, not a resource, so a single world can build some entries
/// dormant and run others.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct DisableBootOnLoad;

fn on_add_boot(mut world: DeferredWorld, cx: HookContext) {
	world.commands().entity(cx.entity).observe_any(on_load_boot);
}

fn on_add_exchange(mut world: DeferredWorld, cx: HookContext) {
	world
		.commands()
		.entity(cx.entity)
		.observe_any(on_load_exchange);
}

fn on_add_run(mut world: DeferredWorld, cx: HookContext) {
	world.commands().entity(cx.entity).observe_any(on_load_run);
}

/// On the entry root's `LoadTemplate`, queue [`Boot::boot`] with the process request.
fn on_load_boot(
	ev: On<LoadTemplate>,
	ancestors: Query<&ChildOf>,
	disabled: Query<(), With<DisableBootOnLoad>>,
	boot_slots: Query<(), With<Action<Boot, Response>>>,
	mut exit: MessageWriter<AppExit>,
	mut commands: Commands,
) {
	let target = ev.event_target();
	if !should_load(target, ev.is_error, &ancestors, &disabled, &mut exit) {
		return;
	}
	// `BootOnLoad` with no `Action<Boot, Response>` slot is a misconfiguration: a
	// render scene should carry `RunOnLoad`, a server should spread a server
	// component (eg `HttpServer`) that installs the boot slot. Fail loudly rather
	// than silently doing nothing.
	if !boot_slots.contains(target) {
		error!(
			"`BootOnLoad` on {target} has no `Action<Boot, Response>` boot slot; a \
			 render scene should use `RunOnLoad`, a server entry should spread a \
			 server component such as `HttpServer`"
		);
		exit.write(AppExit::error());
		return;
	}
	commands.entity(target).queue_async_local(|host| {
		Boot::boot(host, Request::from_cli_args(CliArgs::parse_env()))
	});
}

/// On the entry root's `LoadTemplate`, queue [`Boot::exchange_boot`] with the
/// process request.
fn on_load_exchange(
	ev: On<LoadTemplate>,
	ancestors: Query<&ChildOf>,
	disabled: Query<(), With<DisableBootOnLoad>>,
	mut exit: MessageWriter<AppExit>,
	mut commands: Commands,
) {
	let target = ev.event_target();
	if !should_load(target, ev.is_error, &ancestors, &disabled, &mut exit) {
		return;
	}
	commands.entity(target).queue_async_local(|host| {
		Boot::exchange_boot(host, Request::from_cli_args(CliArgs::parse_env()))
	});
}

/// On the entity's `LoadTemplate`, queue [`Boot::run`] to start its behaviour.
fn on_load_run(
	ev: On<LoadTemplate>,
	ancestors: Query<&ChildOf>,
	disabled: Query<(), With<DisableBootOnLoad>>,
	mut exit: MessageWriter<AppExit>,
	mut commands: Commands,
) {
	let target = ev.event_target();
	if !should_load(target, ev.is_error, &ancestors, &disabled, &mut exit) {
		return;
	}
	commands.entity(target).queue_async_local(|host| Boot::run(host));
}

/// Whether a `LoadTemplate` on `target` should drive a load: the build succeeded
/// and neither the target nor an ancestor opts out via [`DisableBootOnLoad`]. A
/// failed build writes `AppExit::error()` so the process exits instead of running.
fn should_load(
	target: Entity,
	is_error: bool,
	ancestors: &Query<&ChildOf>,
	disabled: &Query<(), With<DisableBootOnLoad>>,
	exit: &mut MessageWriter<AppExit>,
) -> bool {
	// a failed build never runs: exit with an error code.
	if is_error {
		exit.write(AppExit::error());
		return false;
	}
	// skip if this entity or any ancestor disables auto-run.
	!(disabled.contains(target)
		|| ancestors
			.iter_ancestors(target)
			.any(|ancestor| disabled.contains(ancestor)))
}

/// The process request as an exchange event: fire it on an entity to dispatch
/// its `Action<Request, Response>` slot.
#[extend::ext(name = StartRunningRequestExt)]
pub impl StartRunning<Request> {
	/// The process request as an exchange event.
	fn exchange_boot(entity: Entity) -> Self {
		Self::new(entity, Request::from_cli_args(CliArgs::parse_env()))
	}
}

/// The process request as a boot event: fire it on a host to boot its servers.
#[extend::ext(name = StartRunningBootExt)]
pub impl StartRunning<Boot> {
	/// The process request as a boot event.
	fn boot(entity: Entity) -> Self {
		Self::new(entity, Request::from_cli_args(CliArgs::parse_env()).into())
	}
}

/// Stream a one-shot's [`Response`] to stdout and write the matching [`AppExit`].
///
/// The shared tail of both boot paths: [`Boot::boot`] after its awaited call
/// resolves, and [`CliServer`] when it boots via a direct `StartRunning` with no
/// `Running` to resolve.
pub(crate) async fn stream_and_exit(
	host: &AsyncEntity,
	response: Response,
) -> Result {
	let (parts, body) = response.into_parts();
	stream_body_to_stdout(body).await?;
	match parts.status_to_exit_code() {
		Ok(()) => host.world().write_message(AppExit::Success).await,
		Err(code) => {
			error!("Command failed\nStatus code: {code}");
			host.world().write_message(AppExit::Error(code)).await;
		}
	}
	Ok(())
}

/// Streams a [`Response`] body to stdout chunk-by-chunk.
pub(crate) async fn stream_body_to_stdout(mut body: Body) -> Result {
	while let Some(chunk) = body.next().await? {
		cross_log_noline!("{}", String::from_utf8_lossy(&chunk));
	}
	Ok(())
}

#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	fn boot_derefs_to_request() {
		let boot = Boot::from(Request::get("api/users"));
		// Deref reaches the inner Request (and its RequestParts methods)
		(*boot.method()).xpect_eq(HttpMethod::Get);
		boot.path_string().xpect_eq("/api/users");
		// and the inner Request is recoverable by value
		boot.0.path_string().xpect_eq("/api/users");
	}

	/// End to end through the boot slot: `BootOnLoad` calls the host's
	/// `Action<Boot, Response>` slot (provided by `CliServer`), which fans out a
	/// `StartRunning<Boot>`; `CliServer` routes it through the host's dispatch slot
	/// and resolves the parked boot call, and `Boot::boot` exits with the status's
	/// code.
	#[beet_core::test]
	#[cfg(feature = "http")]
	async fn one_shot_resolves_and_exits() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin)).add_systems(
			Startup,
			|mut commands: Commands| {
				commands
					.spawn((
						exchange_handler(|_| Response::ok().with_body("hi")),
						CliServer,
						BootOnLoad,
					))
					.trigger(|entity| LoadTemplate {
						entity,
						is_error: false,
					});
			},
		);
		app.run_async().await.xpect_eq(AppExit::Success);
	}

	/// The lightweight boot: `trigger(StartRunning::boot)` fires a
	/// `StartRunning<Boot>` straight at `CliServer` with no `Running` keep-alive, so
	/// the server streams the response and writes the `AppExit` itself.
	#[beet_core::test]
	#[cfg(feature = "http")]
	async fn boot_event_resolves_and_exits() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin)).add_systems(
			Startup,
			|mut commands: Commands| {
				commands
					.spawn((
						exchange_handler(|_| Response::ok().with_body("hi")),
						CliServer,
					))
					.trigger(StartRunning::boot);
			},
		);
		app.run_async().await.xpect_eq(AppExit::Success);
	}

	/// A long-running server parks the boot call: its `Running<Response>` keep-alive
	/// stays and no `AppExit` is written, so the process persists. The `Running` is
	/// inserted by the server's `ContinueRun<Boot, Response>` slot before the backend
	/// runs, so the park holds whether or not a backend is present.
	#[beet_core::test]
	async fn server_parks_and_stays_up() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin));
		let entity = app
			.world_mut()
			.spawn((HttpServer::new(0), BootOnLoad))
			.trigger(|entity| LoadTemplate {
				entity,
				is_error: false,
			})
			.id();
		// drive until the boot fan-out lands the parking `Running` (a bounded
		// condition, unlike settling a parked server to the frame cap).
		app_ext::update_until(&mut app, |world| {
			world.entity(entity).contains::<Running<Response>>()
		})
		.await
		.xpect_true();
		app.world_mut()
			.run_system_once(|mut exits: MessageReader<AppExit>| {
				exits.read().count()
			})
			.unwrap()
			.xpect_eq(0);
	}

	/// `RunOnLoad` runs the entity's own `Action<(), Outcome>` on its `LoadTemplate`:
	/// a one-shot behaviour resolves (recorded here) and `Boot::run` exits.
	#[beet_core::test]
	async fn run_on_load_runs_behaviour() {
		let ran = Store::new(false);
		let recorder = ran.clone();
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, TemplatePlugin, ActionPlugin));
		// a behaviour root that runs on `LoadTemplate`, recording when its action runs.
		let action: Action<(), Outcome> =
			Action::new_pure(move |_: ActionContext| -> Result<Outcome> {
				recorder.set(true);
				Outcome::PASS.xok()
			});
		app.world_mut()
			.spawn_template(Snippet::from_bundle((RunOnLoad, action)))
			.unwrap();
		// the `LoadTemplate` observer queues `Boot::run` onto the AsyncWorld;
		// `update_until` ticks the runtime between frames so the queued call runs.
		app_ext::update_until(&mut app, |_world| ran.get())
			.await
			.xpect_true();
	}
}
