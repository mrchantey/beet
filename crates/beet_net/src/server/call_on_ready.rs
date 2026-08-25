//! Turning a load into a run: the [`CallOnReady`] verb calls an entity's action
//! when its template's [`Ready`] sweep reaches it, streaming the response and
//! writing the matching [`AppExit`].
//!
//! The canonical load signature is `Request -> Response`: argv/env is the
//! request ([`Request::from_cli_args`]), stdout and the exit code are the
//! response. Two fallbacks let a behavior scene load with no adapter, see
//! [`CallOnReady::call`].
//!
//! The verb fires on every load, so a document says exactly what happens when
//! it is loaded, wherever it is loaded. A loader building a document to render
//! or inspect rather than run (`check`, `export-static`, the wasm Worker)
//! disarms the subtree by inserting [`DisableCallOnReady`] on its root. A
//! synthesized request (serve forwarding a route, `export-pdf`) never rides the
//! sweep; it goes through the explicit [`CallOnReady::call`].
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Load verb: on this entity's [`Ready`], call its action with the process
/// request and stream the one-shot response (see [`CallOnReady::call`]).
///
/// Sits directly on the action it drives, so an entry root, a behavior root or a
/// script element each declares its own:
///
/// ```bsx
/// <CliServer always=true {CallOnReady}>     // a dispatcher root, exiting on resolve
///     <Router>..</Router>                   // the dispatch host, a child
/// </CliServer>
/// <Sequence {CallOnReady}> .. </Sequence>   // a behavior, exiting on resolve
/// <CallOnReady {(CliServer, HttpServer)}>   // a multi-server host root
///     <Router>..</Router>
/// </CallOnReady>
/// ```
///
/// It fires on every load: a scene carrying the verb runs wherever it is
/// loaded, so the file alone says what loading it does. A render-only build
/// (`check`, `export-static`) opts its subtree out with [`DisableCallOnReady`];
/// a failed build exits with an error and never runs.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = hook_ext::observe(on_ready_call))]
pub struct CallOnReady;

/// Opts an entity and its subtree out of [`CallOnReady`]: the tree is built to
/// render or inspect (`check`, `export-static`, the wasm Worker), not to run.
///
/// A component, not a resource, so a single world can build some documents
/// dormant while running others, and anything loaded *under* a disarmed root
/// later (a rendered route page) inherits the disarm through ancestry. An
/// explicit boot ([`CallOnReady::call_recursive`]) ignores it: an explicit
/// call is deliberate.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct DisableCallOnReady;

impl CallOnReady {
	/// Call this entity's action with the process request on spawn, as the load
	/// path would on [`Ready`].
	///
	/// The code counterpart of the markup load path, for an app that spawns its
	/// server directly instead of loading a document:
	///
	/// ```ignore
	/// world.spawn((
	///     HttpServer::default(),
	///     CallOnReady::on_spawn(),
	///     children![exchange_ext::handler(|req| req.mirror())],
	/// ));
	/// ```
	pub fn on_spawn() -> OnSpawn {
		OnSpawn::new_async_local(|entity| {
			Self::call(entity, Self::cli_request())
		})
	}

	/// The process request: argv/env as a [`Request`].
	///
	/// The one sanctioned ambient read, made at a process boundary (an
	/// `on_spawn` boot, an armed load's [`Ready`]).
	fn cli_request() -> Request { Request::from_cli_args(CliArgs::parse_env()) }

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
	/// A long-running action (a parked [`RunningSet`], an endless `Repeat`)
	/// never resolves, so the await parks here and the process stays up; a
	/// one-shot resolves, streams, and exits.
	///
	/// A failed call is the process's result too: it logs and exits nonzero
	/// rather than raising into the app's error handler, since there is no caller
	/// above a load to hear it.
	pub async fn call(entity: AsyncEntity, request: Request) -> Result {
		let response = match Self::response(&entity, request).await {
			Ok(response) => response,
			Err(err) => {
				error!("{err}");
				entity.world().write_message(AppExit::error()).await;
				return Ok(());
			}
		};
		// reached only for a one-shot; a long-running action parks the await.
		stream_and_exit(&entity, response).await
	}

	/// The load call itself, resolving whichever of the three shapes the entity
	/// serves.
	async fn response(
		entity: &AsyncEntity,
		request: Request,
	) -> Result<Response> {
		if entity
			.get(|meta: &ActionMeta| meta.matches::<Request, Response>())
			.await
			.unwrap_or(false)
		{
			entity.call::<Request, Response>(request).await
		} else if entity
			.get(|meta: &ActionMeta| meta.matches::<(), Outcome>())
			.await
			.unwrap_or(false)
		{
			match entity.call::<(), Outcome>(()).await? {
				Pass(()) => Response::ok(),
				Fail(()) => Response::internal_error(),
			}
			.xok()
		} else {
			entity.call::<(), ()>(()).await?;
			Response::ok().xok()
		}
	}

	/// Explicitly call every [`CallOnReady`] action at or under `host`,
	/// each with its own request from `make_request`, ignoring
	/// [`DisableCallOnReady`]: an explicit boot is deliberate (eg `serve`
	/// booting a disarmed-built site).
	///
	/// Resolves once every call resolves, so a parked server holds the await.
	///
	/// # Errors
	/// Errors if no target carries [`CallOnReady`], or any call fails.
	pub async fn call_recursive(
		host: AsyncEntity,
		make_request: impl Fn() -> Request,
	) -> Result {
		let targets = host
			.world()
			.run_system_cached_with(collect_call_on_ready, host.id())
			.await?;
		if targets.is_empty() {
			bevybail!("no CallOnReady actions at or under {:?}", host.id());
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

fn collect_call_on_ready(
	In(root): In<Entity>,
	children: Query<&Children>,
	call_on_ready: Query<(), With<CallOnReady>>,
) -> Vec<Entity> {
	children
		.iter_descendants_inclusive(root)
		.filter(|entity| call_on_ready.contains(*entity))
		.collect()
}

/// On the entity's [`Ready`], queue [`CallOnReady::call`] with the process
/// request, unless the subtree is disarmed or the build failed.
fn on_ready_call(
	ev: On<Ready>,
	ancestors: Query<&ChildOf>,
	disabled: Query<(), With<DisableCallOnReady>>,
	errors: Query<(), With<TemplateError>>,
	mut exit: MessageWriter<AppExit>,
	mut commands: Commands,
) {
	// a disarmed subtree runs nothing, and owes nothing on failure either:
	// whoever built it holds the result.
	if ancestors
		.iter_ancestors_inclusive(ev.entity)
		.any(|entity| disabled.contains(entity))
	{
		return;
	}
	// a failed build never runs: exit with an error code. The error sits on the
	// loaded root, an ancestor (or self) of every swept entity.
	if ancestors
		.iter_ancestors_inclusive(ev.entity)
		.any(|entity| errors.contains(entity))
	{
		exit.write(AppExit::error());
		return;
	}
	let request = CallOnReady::cli_request();
	commands
		.entity(ev.entity)
		.queue_async_local(|entity| CallOnReady::call(entity, request));
}

/// Stream a one-shot's [`Response`] to stdout and write the matching [`AppExit`].
///
/// The tail of the load path, reached once [`CallOnReady::call`]'s awaited call
/// resolves. A long-running action never gets here: its parked call is the
/// process.
async fn stream_and_exit(host: &AsyncEntity, response: Response) -> Result {
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

	/// Sweep a load over `entity`'s subtree, as a booting binary's build does.
	fn load(world: &mut World, entity: Entity) {
		world
			.entity_mut(entity)
			.trigger_target(|entity| Ready { entity });
	}

	/// A one-shot behavior action, recording into `ran` when it runs.
	fn recording_action(ran: Store<bool>) -> Action<(), Outcome> {
		Action::new_pure(move |_: ActionContext| -> Result<Outcome> {
			ran.set(true);
			Outcome::PASS.xok()
		})
	}

	/// End to end through the load path: `CallOnReady` calls the entity's parked
	/// action, whose `RunningSet` reaches `CliServer`; it routes the request
	/// through its dispatch child and resolves the parked call, and
	/// `CallOnReady::call` exits with the status's code.
	#[beet_core::test]
	#[cfg(feature = "http")]
	async fn one_shot_resolves_and_exits() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin)).add_systems(
			Startup,
			|mut commands: Commands| {
				let entity = commands
					.spawn((CliServer::default(), CallOnReady, children![
						exchange_ext::handler(|_| {
							Response::ok().with_body("hi")
						})
					]))
					.id();
				commands.queue(move |world: &mut World| load(world, entity));
			},
		);
		app.run_async().await.xpect_eq(AppExit::Success);
	}

	/// A disarmed build loads the tree and stops: [`DisableCallOnReady`] on the
	/// root is what every render-only command inserts.
	#[beet_core::test]
	#[cfg(feature = "http")]
	async fn a_disarmed_build_never_runs() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin, TemplatePlugin));
		let entity = app
			.world_mut()
			.spawn_template(Snippet::from_bundle((
				DisableCallOnReady,
				CliServer::default(),
				CallOnReady,
				children![exchange_ext::handler(|_| {
					Response::ok().with_body("hi")
				})],
			)))
			.unwrap()
			.id();
		for _ in 0..16 {
			app.update();
			AsyncRunner::tick().await;
		}
		app.world()
			.entity(entity)
			.contains::<Running<Response>>()
			.xpect_false();
	}

	/// The code boot: `CallOnReady::on_spawn` calls a hand-spawned server exactly
	/// as a loaded scene's verb would, so a `main.rs` app needs no template.
	#[beet_core::test]
	#[cfg(feature = "http")]
	async fn on_spawn_resolves_and_exits() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin)).add_systems(
			Startup,
			|mut commands: Commands| {
				commands.spawn((
					CliServer::default(),
					CallOnReady::on_spawn(),
					children![exchange_ext::handler(|_| {
						Response::ok().with_body("hi")
					})],
				));
			},
		);
		app.run_async().await.xpect_eq(AppExit::Success);
	}

	/// A long-running server parks the load call: its `Running<Response>`
	/// keep-alive stays and no `AppExit` is written, so the process persists.
	/// The `Running` is inserted by the entity's `RunningSet` before any facet
	/// runs, so the park holds regardless of what the backend does.
	#[beet_core::test]
	async fn server_parks_and_stays_up() {
		let log = Store::<Vec<&'static str>>::default();
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin));
		let entity = app
			.world_mut()
			.spawn((
				crate::server::http_server::tests::stub_server(0, log),
				CallOnReady,
			))
			.id();
		load(app.world_mut(), entity);
		// drive until the parking `Running` lands (a bounded condition, unlike
		// settling a parked server to the frame cap).
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

	/// `CallOnReady` on a behavior entity converts the load call through the
	/// `() -> Outcome` conversion: a one-shot behavior resolves (recorded here)
	/// and the process exits.
	#[beet_core::test]
	async fn runs_behavior() {
		let ran = Store::new(false);
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, TemplatePlugin, ActionPlugin));
		app.world_mut()
			.spawn_template(Snippet::from_bundle((
				CallOnReady,
				recording_action(ran),
			)))
			.unwrap();
		// the `Ready` observer queues `CallOnReady::call` onto the AsyncWorld;
		// `update_until` ticks the runtime between frames so the queued call runs.
		app_ext::update_until(&mut app, |_world| ran.get())
			.await
			.xpect_true();
	}

	/// The sweep reaches every entity of the loaded tree, so a behavior nested in
	/// a scene runs on its entry's load exactly as a root one does, with no
	/// ancestor lookup.
	#[beet_core::test]
	async fn runs_a_nested_behavior() {
		let ran = Store::new(false);
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, TemplatePlugin, ActionPlugin));
		app.world_mut()
			.spawn_template(Snippet::from_bundle(children![(
				CallOnReady,
				recording_action(ran)
			)]))
			.unwrap();
		app_ext::update_until(&mut app, |_world| ran.get())
			.await
			.xpect_true();
	}

	/// A later load under a live root runs itself too: a scene composing a
	/// runnable sub-scene boots it on the sub-scene's own `Ready`, no
	/// declaration required.
	#[beet_core::test]
	async fn a_nested_load_under_a_live_root_runs() {
		let ran = Store::new(false);
		let nested_ran = Store::new(false);
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, TemplatePlugin, ActionPlugin));
		let root = app
			.world_mut()
			.spawn_template(Snippet::from_bundle((
				CallOnReady,
				recording_action(ran),
			)))
			.unwrap()
			.id();
		app_ext::update_until(&mut app, |_world| ran.get())
			.await
			.xpect_true();

		// a second load under the running root.
		app.world_mut()
			.spawn(ChildOf(root))
			.insert_template(Snippet::from_bundle((
				CallOnReady,
				recording_action(nested_ran),
			)))
			.unwrap();
		app_ext::update_until(&mut app, |_world| nested_ran.get())
			.await
			.xpect_true();
	}

	/// A load under a disarmed root inherits the disarm through ancestry: a
	/// render command disarms its entry root once, and everything loaded under
	/// it later (a route page) stays dormant with no further bookkeeping.
	#[beet_core::test]
	async fn a_nested_load_under_a_disarmed_root_stays_dormant() {
		let nested_ran = Store::new(false);
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, TemplatePlugin, ActionPlugin));
		let root = app.world_mut().spawn(DisableCallOnReady).id();
		app.world_mut()
			.spawn(ChildOf(root))
			.insert_template(Snippet::from_bundle((
				CallOnReady,
				recording_action(nested_ran),
			)))
			.unwrap();
		for _ in 0..16 {
			app.update();
			AsyncRunner::tick().await;
		}
		nested_ran.get().xpect_false();
	}

	/// A failed build never runs and exits nonzero, so a broken entry fails the
	/// process rather than serving a half-built tree.
	#[beet_core::test]
	async fn a_failed_build_exits_nonzero() {
		let ran = Store::new(false);
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, TemplatePlugin, ActionPlugin));
		let root = app.world_mut().spawn(CallOnReady).id();
		app.world_mut()
			.entity_mut(root)
			.insert(recording_action(ran))
			.insert(TemplateError::new(bevyhow!("boom")))
			.trigger_target(|entity| Ready { entity });
		app.world_mut()
			.run_system_once(|mut exits: MessageReader<AppExit>| {
				exits.read().any(AppExit::is_error)
			})
			.unwrap()
			.xpect_true();
		ran.get().xpect_false();
	}
}
