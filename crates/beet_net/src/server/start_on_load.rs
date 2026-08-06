//! The boot verb: [`StartOnLoad`] parks the process on load and fans the
//! request out to the servers spread on its entity, with the dispatch host
//! (usually a `Router`) as a child.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Boots this entity's servers on load: the [`CallOnLoad`] call parks a
/// [`Running<Response>`] keep-alive and fans a [`StartRunning<Request>`] out to
/// every server observing it, so the process stays up for as long as one serves.
///
/// A server is config plus that observer, so one reads as `<HttpServer>` (which
/// requires this) and several as a spread, both with the router as a child:
///
/// ```bsx
/// <HttpServer>
///     <Router>..</Router>
/// </HttpServer>
///
/// <StartOnLoad {(HttpServer, TuiServer, SshTuiServer)}>
///     <Router>..</Router>
/// </StartOnLoad>
/// ```
///
/// A one-shot server ([`CliServer`]) resolves the parked call instead, so the
/// response streams and the process exits. When `--server` selects nothing, no
/// server boots and [`exit_if_no_server`] exits rather than parking forever.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(
	CallOnLoad,
	Action<Request, Response> =
		ContinueRun::<Request, Response>::start_running()
		.with_meta(ActionMeta::of::<Self, Request, Response>())
)]
#[component(
	on_add = Action::<Request, Response>::assert_provider::<Self>,
	on_add = hook_ext::observe(await_server_boot)
)]
pub struct StartOnLoad;

/// Marks a [`StartOnLoad`] whose fan-out has gone out but whose servers have not
/// yet been checked; [`exit_if_no_server`] clears it on the next frame, once
/// every observer's commands have flushed.
#[derive(Component)]
pub(crate) struct AwaitingServerBoot;

/// Inserted by a server that actually booted for this request, so a boot where
/// `--server` selected nothing is distinguishable from one that parked to serve.
///
/// Every server inserts it from its own [`StartRunning<Request>`] observer, past
/// its selection check: [`boot_server`] does it for the shared
/// [`BootServer`] lifecycle, and a bespoke server (eg [`CliServer`], `TuiServer`)
/// does it inline.
#[derive(Component)]
pub struct ServerBooted;

/// Flags the fan-out for the [`exit_if_no_server`] check.
fn await_server_boot(ev: On<StartRunning<Request>>, mut commands: Commands) {
	commands.entity(ev.entity).insert(AwaitingServerBoot);
}

/// Exits rather than parking forever when a boot brought up no server, ie
/// `beet serve site --server=nonexistent`.
///
/// Runs a frame after the fan-out, by which point every co-observer's commands
/// have flushed, so a missing [`ServerBooted`] means nothing selected this
/// request.
pub(crate) fn exit_if_no_server(
	mut commands: Commands,
	mut exit: MessageWriter<AppExit>,
	awaiting: Populated<(Entity, Has<ServerBooted>), With<AwaitingServerBoot>>,
) {
	for (entity, booted) in awaiting.iter() {
		commands.entity(entity).remove::<AwaitingServerBoot>();
		if booted {
			continue;
		}
		error!(
			"No server booted for {entity}, does --server (or BEET_SERVER) name \
			 a server this entry declares?"
		);
		exit.write(AppExit::error());
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;

	/// A `--server` naming nothing this entry declares must exit, not park the
	/// process on a `Running` no server will ever resolve.
	#[beet_core::test]
	async fn unselected_boot_exits() {
		crate::server::http_server::stub_backend();
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin));
		let entity = app
			.world_mut()
			.spawn((StartOnLoad, HttpServer::new(0)))
			.id();
		app.world_mut().entity_mut(entity).run_async_local(
			|server| async move {
				server
					.call::<Request, Response>(Request::from_cli_str(
						"--server=nonexistent",
					))
					.await?;
				Ok(())
			},
		);
		app.run_async().await.xpect_eq(AppExit::error());
	}

	/// Two servers spread on one `StartOnLoad` both hear the fan-out.
	#[beet_core::test]
	async fn boots_every_declared_server() {
		crate::server::http_server::stub_backend();
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin));
		let (channel_server, _client) = ChannelHttpServer::new();
		let entity = app
			.world_mut()
			.spawn((StartOnLoad, HttpServer::new(0), channel_server))
			.id();
		app.world_mut().entity_mut(entity).run_async_local(
			|server| async move {
				server.call::<Request, Response>(Request::get("/")).await?;
				Ok(())
			},
		);
		app_ext::update_until(&mut app, |world| {
			world
				.entity(entity)
				.get::<ServerShutdown<ChannelHttpServer>>()
				.is_some_and(ServerShutdown::is_live)
				&& world
					.entity(entity)
					.get::<ServerShutdown<HttpServer>>()
					.is_some_and(ServerShutdown::is_live)
		})
		.await
		.xpect_true();
	}
}
