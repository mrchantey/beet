//! Turning the process request into a run: the [`CallOnLoad`] verb calls an
//! entity's action with the process request once its template loads, streaming
//! the response and writing the matching [`AppExit`].
//!
//! The canonical load signature is `Request -> Response`: argv/env is the
//! request ([`Request::from_cli_args`]), stdout and the exit code are the
//! response. Two fallbacks let a behavior scene load with no adapter, see
//! [`CallOnLoad::call`].
//!
//! A [`StartOnLoad`] entity parks the call and fans it out to its servers,
//! holding the process open; a one-shot (a [`CliServer`], a finite behavior)
//! resolves it and the process exits with the response status. This is the one
//! path that reads `CliArgs::parse_env()` and writes [`AppExit`]; servers are
//! handed the request, never re-parsing argv.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Load verb: on this entity's `LoadTemplate`, call its action with the
/// process request and stream the one-shot response (see [`CallOnLoad::call`]).
///
/// Sits directly on the action it drives. `LoadTemplate` fires on every
/// descendant of a built entry, so a server root, a behavior root, or a
/// script element all hear their own:
///
/// ```bsx
/// <HttpServer>                            // requires StartOnLoad, parking on load
///     <Router>..</Router>                 // the dispatch host, a child
/// </HttpServer>
/// <Sequence {CallOnLoad}> .. </Sequence>  // a behavior, exiting on resolve
/// ```
///
/// A failed build exits with an error and never runs; a [`DisableCallOnLoad`]
/// on the entity or an ancestor opts the subtree out (eg a render/inspect
/// build).
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = hook_ext::observe(on_load_call))]
pub struct CallOnLoad;

/// Opts an entity (and its subtree) out of [`CallOnLoad`]: the tree is built
/// to render or inspect (eg `export-static`, `check`), not to run. A
/// component, not a resource, so a single world can build some entries
/// dormant and run others.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct DisableCallOnLoad;

impl CallOnLoad {
	/// Call the entity's action with `request` and, once the call resolves,
	/// stream the response and write the matching [`AppExit`].
	///
	/// Three shapes serve a load, tried in order, so a behavior scene needs no
	/// adapter component:
	///
	/// - `Request -> Response`, the canonical one (itself resolving the entity's
	///   canonical action then any [`ActionOverload`])
	/// - `() -> Outcome`, a behavior: `Pass` exits zero, `Fail` nonzero
	/// - `() -> ()`, a plain action, always zero
	///
	/// A long-running action (a parked [`StartOnLoad`], an endless `Repeat`)
	/// never resolves, so the await parks here and the process stays up; a
	/// one-shot resolves, streams, and exits.
	pub async fn call(entity: AsyncEntity, request: Request) -> Result {
		let response = if entity
			.get(|meta: &ActionMeta| meta.matches::<Request, Response>())
			.await
			.unwrap_or(false)
		{
			entity.call::<Request, Response>(request).await?
		} else if entity
			.get(|meta: &ActionMeta| meta.matches::<(), Outcome>())
			.await
			.unwrap_or(false)
		{
			match entity.call::<(), Outcome>(()).await? {
				Pass(()) => Response::ok(),
				Fail(()) => Response::internal_error(),
			}
		} else {
			entity.call::<(), ()>(()).await?;
			Response::ok()
		};
		// reached only for a one-shot; a long-running action parks the await.
		stream_and_exit(&entity, response).await
	}

	/// Explicitly call every [`CallOnLoad`] action at or under `host`,
	/// each with its own request from `make_request`, ignoring
	/// [`DisableCallOnLoad`] (an explicit boot is deliberate, eg `beet serve`
	/// booting a dormant-built site).
	///
	/// Resolves once every call resolves, so a parked server holds the await.
	///
	/// # Errors
	/// Errors if no target carries [`CallOnLoad`], or any call fails.
	pub async fn call_recursive(
		host: AsyncEntity,
		make_request: impl Fn() -> Request,
	) -> Result {
		let targets = host
			.world()
			.run_system_cached_with(collect_call_on_load, host.id())
			.await?;
		if targets.is_empty() {
			bevybail!("no CallOnLoad actions at or under {:?}", host.id());
		}
		let world = host.world().clone();
		async_ext::try_join_all(
			targets
				.into_iter()
				.map(|target| Self::call(world.entity(target), make_request())),
		)
		.await?;
		Ok(())
	}
}

fn collect_call_on_load(
	In(root): In<Entity>,
	children: Query<&Children>,
	call_on_load: Query<(), With<CallOnLoad>>,
) -> Vec<Entity> {
	children
		.iter_descendants_inclusive(root)
		.filter(|entity| call_on_load.contains(*entity))
		.collect()
}

/// On the entity's `LoadTemplate`, queue [`CallOnLoad::call`] with the process
/// request.
fn on_load_call(
	ev: On<LoadTemplate>,
	ancestors: Query<&ChildOf>,
	disabled: Query<(), With<DisableCallOnLoad>>,
	mut exit: MessageWriter<AppExit>,
	mut commands: Commands,
) {
	let target = ev.event_target();
	if !should_load(target, ev.is_error, &ancestors, &disabled, &mut exit) {
		return;
	}
	commands.entity(target).queue_async_local(|entity| {
		CallOnLoad::call(entity, Request::from_cli_args(CliArgs::parse_env()))
	});
}

/// Whether a `LoadTemplate` on `target` should drive a load: the build succeeded
/// and neither the target nor an ancestor opts out via [`DisableCallOnLoad`]. A
/// failed build writes `AppExit::error()` so the process exits instead of running.
fn should_load(
	target: Entity,
	is_error: bool,
	ancestors: &Query<&ChildOf>,
	disabled: &Query<(), With<DisableCallOnLoad>>,
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

/// The process request as a boot event: fire it on a server entity to boot it
/// directly, skipping the load path.
#[extend::ext(name = StartRunningRequestExt)]
pub impl StartRunning<Request> {
	/// The process request as a boot event.
	fn from_cli(entity: Entity) -> Self {
		Self::new(entity, Request::from_cli_args(CliArgs::parse_env()))
	}
}

/// Stream a one-shot's [`Response`] to stdout and write the matching [`AppExit`].
///
/// The shared tail of the load path: [`CallOnLoad::call`] after its awaited
/// call resolves, and [`CliServer`] when it boots via a direct `StartRunning`
/// with no `Running` to resolve.
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

	/// End to end through the load path: `CallOnLoad` (required via `StartOnLoad`)
	/// calls the server's parking action, which fans out a `StartRunning<Request>`;
	/// `CliServer` routes it through its dispatch child and resolves the parked
	/// call, and `CallOnLoad::call` exits with the status's code.
	#[beet_core::test]
	#[cfg(feature = "http")]
	async fn one_shot_resolves_and_exits() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin)).add_systems(
			Startup,
			|mut commands: Commands| {
				commands
					.spawn((CliServer::default(), children![exchange_handler(
						|_| { Response::ok().with_body("hi") }
					)]))
					.trigger(|entity| LoadTemplate {
						entity,
						is_error: false,
					});
			},
		);
		app.run_async().await.xpect_eq(AppExit::Success);
	}

	/// The lightweight boot: `trigger(StartRunning::from_cli)` fires a
	/// `StartRunning<Request>` straight at `CliServer` with no `Running`
	/// keep-alive, so the server streams the response and writes the `AppExit`
	/// itself.
	#[beet_core::test]
	#[cfg(feature = "http")]
	async fn boot_event_resolves_and_exits() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin)).add_systems(
			Startup,
			|mut commands: Commands| {
				commands
					.spawn((CliServer::default(), children![exchange_handler(
						|_| { Response::ok().with_body("hi") }
					)]))
					.trigger(StartRunning::from_cli);
			},
		);
		app.run_async().await.xpect_eq(AppExit::Success);
	}

	/// A long-running server parks the load call: its `Running<Response>`
	/// keep-alive stays and no `AppExit` is written, so the process persists.
	/// The `Running` is inserted by the server's `ContinueRun<Request, Response>`
	/// before the backend runs, so the park holds regardless of what the backend
	/// does.
	#[beet_core::test]
	async fn server_parks_and_stays_up() {
		// the global backend hook is first-install-wins for the whole test
		// binary; installing the shared stub keeps this case order-independent
		crate::server::http_server::stub_backend();
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin));
		let entity = app
			.world_mut()
			.spawn(HttpServer::new(0))
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

	/// `CallOnLoad` on a behavior entity converts the load call through the
	/// `() -> Outcome` conversion: a one-shot behavior resolves (recorded here)
	/// and the process exits.
	#[beet_core::test]
	async fn runs_behavior() {
		let ran = Store::new(false);
		let recorder = ran.clone();
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, TemplatePlugin, ActionPlugin));
		// a behavior root that runs on `LoadTemplate`, recording when its action runs.
		let action: Action<(), Outcome> =
			Action::new_pure(move |_: ActionContext| -> Result<Outcome> {
				recorder.set(true);
				Outcome::PASS.xok()
			});
		app.world_mut()
			.spawn_template(Snippet::from_bundle((CallOnLoad, action)))
			.unwrap();
		// the `LoadTemplate` observer queues `CallOnLoad::call` onto the
		// AsyncWorld; `update_until` ticks the runtime between frames so the
		// queued call runs.
		app_ext::update_until(&mut app, |_world| ran.get())
			.await
			.xpect_true();
	}
}
