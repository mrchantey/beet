//! The single place that turns the process request into a run: it calls the
//! entry's exchangeable action and writes the [`AppExit`].
//!
//! [`RunOnLoad`], spread on the entry root, observes its `LoadTemplate` and drives
//! [`bootstrap`]: it calls the entity's own [`Action<Request, Response>`] slot with
//! the process request. That slot is a `RunScript` (runs a script, returns its
//! output), an `ActionTrigger` (fans out to server observers), or a `Router`
//! (routes directly) — whatever the entry installed. A one-shot resolves and its
//! response streams to stdout before exit; a long-running server parks the call,
//! holding the process up until its keep-alive `Running` is removed.
//!
//! This is the one path that reads `CliArgs::parse_env()` and writes `AppExit`;
//! the servers are handed the request, never re-parse argv.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Generalizable load verb: on an entity's `LoadTemplate`, runs its exchangeable
/// action with the process request and handles the result.
///
/// Spread on the entry root alongside whatever fills its action slot:
///
/// ```bsx
/// <Router {(HttpServer, CliServer, RunOnLoad)}>
/// <script {RunScript}{RunOnLoad}>console.log("hi")</script>
/// ```
///
/// `on_add` registers the `LoadTemplate` observer on the marked entity, so it must
/// sit on the entry root where `LoadTemplate` fires once the whole subtree is
/// built. A failed build exits with an error and never runs; a [`DisableRunOnLoad`]
/// on the entity or an ancestor opts the subtree out (eg a render/inspect build).
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = on_add)]
pub struct RunOnLoad;

/// Opts an entity (and its subtree) out of [`RunOnLoad`]: the tree is built to
/// render or inspect (eg `export-static`, `check`), not to run. A component, not a
/// resource, so a single world can build some entries dormant and run others.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct DisableRunOnLoad;

fn on_add(mut world: DeferredWorld, cx: HookContext) {
	world
		.commands()
		.entity(cx.entity)
		.observe_any(on_load_template);
}

/// On the entry root's `LoadTemplate`, queue [`bootstrap`] with the process
/// request, unless the build failed (exit with an error) or the entity/subtree is
/// [`DisableRunOnLoad`]-disabled.
fn on_load_template(
	ev: On<LoadTemplate>,
	ancestors: Query<&ChildOf>,
	disabled: Query<(), With<DisableRunOnLoad>>,
	mut exit: MessageWriter<AppExit>,
	mut commands: Commands,
) {
	// a failed build never runs: exit with an error code.
	if ev.is_error {
		exit.write(AppExit::error());
		return;
	}
	let target = ev.event_target();
	// skip if this entity or any ancestor disables auto-run.
	if disabled.contains(target)
		|| ancestors.iter_ancestors(target).any(|e| disabled.contains(e))
	{
		return;
	}
	commands.entity(target).queue_async_local(|host| {
		bootstrap(host, Request::from_cli_args(CliArgs::parse_env()))
	});
}

/// Call the host's exchangeable action with `request` and, for the one-shot it
/// resolves, stream the response and write the [`AppExit`].
///
/// `host.call` invokes the entity's own `Action<Request, Response>` slot — a
/// regular exchangeable call, the same path any caller takes. A long-running
/// server's slot (an `ActionTrigger`) never resolves the call, so the await parks
/// here and the process stays up; a one-shot resolves, streams, and exits.
pub async fn bootstrap(host: AsyncEntity, request: Request) -> Result {
	let res = host.call::<Request, Response>(request).await?;
	// reached only for the one-shot; a long-running server parks the await above.
	let (parts, body) = res.into_parts();
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

/// Whether a server named `name` should boot for `request`, read from its
/// `--server` param (comma-separated globs). An absent/empty value matches every
/// present server; otherwise the name must pass the [`GlobFilter`].
pub fn request_selects_server(request: &Request, name: &str) -> bool {
	request
		.get_param("server")
		.into_iter()
		.flat_map(|value| value.split(','))
		.map(str::trim)
		.filter(|name| !name.is_empty())
		.fold(GlobFilter::default(), |filter, name| {
			filter.with_include(name)
		})
		.passes(name)
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

	/// End to end through the slot: `RunOnLoad` calls the entity's exchangeable
	/// action (a `RouteAction` fronted by an `ActionTrigger` slot) which resolves
	/// via `CliServer`, and `bootstrap` exits with the status's exit code.
	#[beet_core::test]
	#[cfg(feature = "http")]
	async fn one_shot_resolves_and_exits() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin)).add_systems(
			Startup,
			|mut commands: Commands| {
				commands
					.spawn((
						ActionTrigger::<Request, Response>::default(),
						RouteAction(exchange_handler(|_| {
							Response::ok().with_body("hi")
						})),
						CliServer,
						RunOnLoad,
					))
					.trigger(|entity| LoadTemplate {
						entity,
						is_error: false,
					});
			},
		);
		app.run_async().await.xpect_eq(AppExit::Success);
	}

	/// A long-running server parks the boot call: its `Running<Response>` keep-alive
	/// claim stays and no `AppExit` is written, so the process would persist. No
	/// backend stub is installed (the global `set_http_server` `OnceLock` is shared
	/// with `http_server`'s tests): the `Running` is inserted by the `ActionTrigger`
	/// slot before the backend runs, so the park holds whether or not a backend is
	/// present.
	#[beet_core::test]
	async fn server_parks_and_stays_up() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin));
		let entity = app
			.world_mut()
			.spawn((
				ActionTrigger::<Request, Response>::default(),
				HttpServer::new(0),
				RunOnLoad,
			))
			.trigger(|entity| LoadTemplate {
				entity,
				is_error: false,
			})
			.id();
		app.update_async().await;
		app.world()
			.entity(entity)
			.contains::<Running<Response>>()
			.xpect_true();
		app.world_mut()
			.run_system_once(|mut exits: MessageReader<AppExit>| {
				exits.read().count()
			})
			.unwrap()
			.xpect_eq(0);
	}
}
