//! Turning a load into a run: the [`CallOnLoad`] verb calls an entity's action
//! when its template loads, with the request the load was given, streaming the
//! response and writing the matching [`AppExit`].
//!
//! The canonical load signature is `Request -> Response`: argv/env is the
//! request ([`Request::from_cli_args`]), stdout and the exit code are the
//! response. Two fallbacks let a behavior scene load with no adapter, see
//! [`CallOnLoad::call`].
//!
//! Loading a template never implies running it. The request is part of the load
//! context, declared by whoever builds the tree ([`LoadRequest`]) rather than
//! read from argv at fire time, so a command building a document inside another
//! process builds it dormant by simply providing none.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Load verb: on this entity's `LoadTemplate`, call its action with the load's
/// request and stream the one-shot response (see [`CallOnLoad::call`]).
///
/// Sits directly on the action it drives, so an entry root, a behavior root or a
/// script element each declares its own:
///
/// ```bsx
/// <CliServer always=true {CallOnLoad}>    // a dispatcher root, exiting on resolve
///     <Router>..</Router>                 // the dispatch host, a child
/// </CliServer>
/// <Sequence {CallOnLoad}> .. </Sequence>  // a behavior, exiting on resolve
/// ```
///
/// It fires only when the load carried a [`LoadRequest`] (on this entity or an
/// ancestor), which is what makes a render-only build (`check`, `export-static`)
/// dormant without an opt-out component. A failed build exits with an error and
/// never runs.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = hook_ext::observe(on_load_call))]
pub struct CallOnLoad;

/// The request a load fires with: the load context a [`CallOnLoad`] under it
/// calls with.
///
/// Provided by whoever builds the tree, never read ambiently at fire time. The
/// binary supplies its argv request when loading the process entry (the one
/// sanctioned ambient read, made once at the process boundary); a command
/// building a document to render or inspect supplies none, and nothing fires.
///
/// Held as [`RequestParts`] rather than a [`Request`] so one load context serves
/// every `CallOnLoad` beneath it: a boot request is argv-shaped and carries no
/// body.
#[derive(Debug, Clone, Component)]
pub struct LoadRequest(RequestParts);

impl LoadRequest {
	/// The load context for `request`.
	pub fn new(request: &Request) -> Self {
		Self(request.request_parts().clone())
	}

	/// The process request as a load context: the binary's own argv, read once
	/// at the process boundary.
	pub fn from_cli() -> Self {
		Self::new(&Request::from_cli_args(CliArgs::parse_env()))
	}

	/// A fresh [`Request`] for one call.
	pub fn request(&self) -> Request {
		Request::from_parts(self.0.clone(), default())
	}

	/// Deliver this load context to the entity on spawn, calling its action as a
	/// document load would.
	///
	/// The code counterpart of the markup load path, for an app that spawns its
	/// server directly instead of loading a document:
	///
	/// ```ignore
	/// world.spawn((
	///     HttpServer::default(),
	///     LoadRequest::from_cli().on_spawn(),
	///     children![exchange_ext::handler(|req| req.mirror())],
	/// ));
	/// ```
	pub fn on_spawn(self) -> OnSpawn {
		OnSpawn::new_async_local(move |entity| {
			CallOnLoad::call(entity, self.request())
		})
	}
}

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
	async fn response(entity: &AsyncEntity, request: Request) -> Result<Response> {
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

	/// Explicitly call every [`CallOnLoad`] action at or under `host`,
	/// each with its own request from `make_request`.
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

/// On the entity's `LoadTemplate`, queue [`CallOnLoad::call`] with the load's
/// request, resolved from the nearest [`LoadRequest`] at or above it. A load
/// given no request builds the tree and stops there.
fn on_load_call(
	ev: On<LoadTemplate>,
	load_requests: AncestorQuery<&LoadRequest>,
	mut exit: MessageWriter<AppExit>,
	mut commands: Commands,
) {
	let target = ev.event_target();
	// a failed build never runs: exit with an error code.
	if ev.is_error {
		exit.write(AppExit::error());
		return;
	}
	let Ok(load) = load_requests.get(target) else {
		return;
	};
	let request = load.request();
	commands
		.entity(target)
		.queue_async_local(|entity| CallOnLoad::call(entity, request));
}

/// The process request as a start notification: fire it on an entity to reach
/// the [`StartRunning<Request>`] observers a real start would, without a
/// [`RunningSet`] to walk. Booting a server is [`LoadRequest::on_spawn`].
#[extend::ext(name = StartRunningRequestExt)]
pub impl StartRunning<Request> {
	/// The process request as a start notification.
	fn from_cli(entity: Entity) -> Self {
		Self::new(entity, Request::from_cli_args(CliArgs::parse_env()))
	}
}

/// Stream a one-shot's [`Response`] to stdout and write the matching [`AppExit`].
///
/// The tail of the load path, reached once [`CallOnLoad::call`]'s awaited call
/// resolves. A long-running action never gets here: its parked call is the
/// process.
async fn stream_and_exit(
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

	/// Fire a load on `entity`, with the load context a running binary provides.
	fn load(world: &mut World, entity: Entity) {
		world
			.entity_mut(entity)
			.insert(LoadRequest::new(&Request::get("/")))
			.trigger(|entity| LoadTemplate {
				entity,
				is_error: false,
			});
	}

	/// End to end through the load path: `CallOnLoad` calls the entity's parked
	/// action, whose `RunningSet` walk reaches `CliServer`; it routes the request
	/// through its dispatch child and resolves the parked call, and
	/// `CallOnLoad::call` exits with the status's code.
	#[beet_core::test]
	#[cfg(feature = "http")]
	async fn one_shot_resolves_and_exits() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin)).add_systems(
			Startup,
			|mut commands: Commands| {
				let entity = commands
					.spawn((
						CliServer::default(),
						CallOnLoad,
						children![exchange_ext::handler(|_| {
							Response::ok().with_body("hi")
						})],
					))
					.id();
				commands.queue(move |world: &mut World| load(world, entity));
			},
		);
		app.run_async().await.xpect_eq(AppExit::Success);
	}

	/// A load with no `LoadRequest` in scope builds the tree and stops: the
	/// dormant build every render-only command relies on.
	#[beet_core::test]
	#[cfg(feature = "http")]
	async fn no_load_request_never_runs() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin));
		let entity = app
			.world_mut()
			.spawn((
				CliServer::default(),
				CallOnLoad,
				children![exchange_ext::handler(|_| {
					Response::ok().with_body("hi")
				})],
			))
			.trigger(|entity| LoadTemplate {
				entity,
				is_error: false,
			})
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

	/// The code boot: `LoadRequest::on_spawn` calls a hand-spawned server exactly
	/// as a document load would, so a `main.rs` app needs no template.
	#[beet_core::test]
	#[cfg(feature = "http")]
	async fn spawned_load_request_resolves_and_exits() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin)).add_systems(
			Startup,
			|mut commands: Commands| {
				commands.spawn((
					CliServer::default(),
					LoadRequest::from_cli().on_spawn(),
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
	/// The `Running` is inserted by the entity's `RunningSet` before any entry
	/// runs, so the park holds regardless of what the backend does.
	// see the NATIVE_ONLY note in `http_server`
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test]
	async fn server_parks_and_stays_up() {
		// the global backend hook is first-install-wins for the whole test
		// binary; installing the shared stub keeps this case order-independent
		crate::server::http_server::stub_backend();
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin));
		let entity = app
			.world_mut()
			.spawn((HttpServer::new(0), CallOnLoad))
			.id();
		load(app.world_mut(), entity);
		// drive until the walk lands the parking `Running` (a bounded condition,
		// unlike settling a parked server to the frame cap).
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
			.spawn_template(Snippet::from_bundle((
				CallOnLoad,
				LoadRequest::from_cli(),
				action,
			)))
			.unwrap();
		// the `LoadTemplate` observer queues `CallOnLoad::call` onto the
		// AsyncWorld; `update_until` ticks the runtime between frames so the
		// queued call runs.
		app_ext::update_until(&mut app, |_world| ran.get())
			.await
			.xpect_true();
	}

	/// The load context resolves by ancestry, so a behavior nested in a scene
	/// runs on the entry's load exactly as a root one does.
	#[beet_core::test]
	async fn runs_a_nested_behavior() {
		let ran = Store::new(false);
		let recorder = ran.clone();
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, TemplatePlugin, ActionPlugin));
		let action: Action<(), Outcome> =
			Action::new_pure(move |_: ActionContext| -> Result<Outcome> {
				recorder.set(true);
				Outcome::PASS.xok()
			});
		app.world_mut()
			.spawn_template(Snippet::from_bundle((
				LoadRequest::from_cli(),
				children![(CallOnLoad, action)],
			)))
			.unwrap();
		app_ext::update_until(&mut app, |_world| ran.get())
			.await
			.xpect_true();
	}
}
