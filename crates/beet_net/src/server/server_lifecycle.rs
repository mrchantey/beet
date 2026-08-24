//! What a listener-backed server contributes to its entity's [`RunningSet`].
//!
//! A server is config plus a contribution: [`ServerFacet::contribute`] appends
//! the start that opens its listener and the stop that closes it, so several
//! servers spread on one entity share that entity's single parked action. The
//! only per-server differences are the `--server` selection (with any bind knobs
//! it overlays) and the serve-loop launcher, so this captures the rest once,
//! including the [`ShutdownSignal`] the two halves share.
//!
//! A server with no listener to open (the one-shot [`CliServer`], the terminal
//! servers) contributes its own pair through [`RunningSet::contribute`] directly.

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::component::Mutable;
use bevy::platform::sync::Arc;
use bevy::platform::sync::Mutex;
use core::net::IpAddr;

/// One listener-backed server's contribution to its entity's [`RunningSet`].
///
/// The four built-in servers ([`HttpServer`], [`SocketServer`] and their channel
/// variants) join through this rather than re-deriving the selection, the
/// shutdown handoff and the outcome shape; a downstream server does too.
pub struct ServerFacet;

/// Launches a server's serve loop on a started host, handed the shutdown
/// receiver it owns its teardown on: it stops accepting and drops its listener
/// when the signal resolves.
///
/// Local (never `Send`): the loop is thread-bound, so the start always drives it
/// with a local task.
pub type ServeFn =
	fn(AsyncEntity, OnceValueRx<()>) -> LocalBoxedFuture<'static, Result>;

impl ServerFacet {
	/// Append `S`'s start/stop pair to its entity's [`RunningSet`].
	///
	/// `boot` answers whether this boot selects the server, overlaying any bind
	/// knobs the request carries onto its config as it does, and `serve` launches
	/// its loop. Those two are the whole per-server difference.
	pub fn contribute<S>(
		entity: &mut EntityCommands,
		boot: fn(&mut S, &Request) -> Result<bool>,
		serve: ServeFn,
	) where
		S: Component<Mutability = Mutable>,
	{
		let shutdown = ShutdownSignal::default();
		RunningSet::<Request, Response>::contribute(
			entity,
			Self::start::<S>(shutdown.clone(), boot, serve),
			Some(Self::stop(shutdown)),
		);
	}

	/// The start half: decide selection against the server's own config, then
	/// hand the serve loop a fresh shutdown receiver. Never resolves the parked
	/// call, so the host's `Running<Response>` keeps the process up.
	///
	/// Synchronous rather than async: selection reads the server's own component
	/// and the launch is a queued task, neither of which needs to await, and an
	/// entry that never awaits keeps the walk off the world bridge. Built by hand
	/// because [`Action::new_system`] caches its system, which rules out the
	/// per-server data these entries carry.
	fn start<S>(
		shutdown: ShutdownSignal,
		boot: fn(&mut S, &Request) -> Result<bool>,
		serve: ServeFn,
	) -> Action<Request, StartOutcome<Request>>
	where
		S: Component<Mutability = Mutable>,
	{
		Action::new(
			ActionMeta::of::<ServerFacet, Request, StartOutcome<Request>>(),
			move |ActionCall {
			          mut commands,
			          caller,
			          input,
			          out_handler,
			      }| {
				let shutdown = shutdown.clone();
				commands.commands.queue(move |world: &mut World| -> Result {
					let outcome = Self::start_now(
						world, caller, input, &shutdown, boot, serve,
					);
					out_handler.call_world(world, outcome)
				});
				Ok(())
			},
		)
	}

	/// Ask the server whether this boot selects it, then launch its serve loop.
	/// Never resolves the parked call, so the host's `Running<Response>` keeps the
	/// process up.
	fn start_now<S>(
		world: &mut World,
		caller: Entity,
		request: Request,
		shutdown: &ShutdownSignal,
		boot: fn(&mut S, &Request) -> Result<bool>,
		serve: ServeFn,
	) -> Result<StartOutcome<Request>>
	where
		S: Component<Mutability = Mutable>,
	{
		// the request threads on to the next entry either way, so the config
		// overlay borrows it rather than consuming it.
		let selected = match world.get_mut::<S>(caller) {
			Some(mut server) => boot(&mut server, &request)?,
			None => false,
		};
		if !selected {
			return StartOutcome::Declined(request).xok();
		}
		let receiver = shutdown.open();
		// the loop outlives this entry, so its failure (a port already bound)
		// comes back through the run it was started for: the parked call resolves
		// with the error rather than the detached task raising into the app's
		// error handler.
		world.commands().entity(caller).queue_async_local(
			move |entity: AsyncEntity| async move {
				match serve(entity.clone(), receiver).await {
					Ok(()) => Ok(()),
					Err(err) => {
						entity.queue(FailRun::<Response>::new(err)).await?
					}
				}
			},
		);
		StartOutcome::Started(request).xok()
	}

	/// The stop half: signal the shutdown its start opened, so the backend stops
	/// accepting and drops its listener. Cause-agnostic, so a reload, an
	/// interrupt and a despawn all close the socket.
	fn stop(shutdown: ShutdownSignal) -> Action<(), ()> {
		Action::new_pure(move |_: ActionContext| shutdown.close())
	}
}

/// The teardown signal a server's start opens and its stop closes.
///
/// Shared by both halves of one server's contribution rather than stored on the
/// entity, so a co-resident server never clobbers it and a reboot can never
/// orphan the live listener's teardown: opening closes the previous signal
/// first. A no_std one-shot, so an embedded backend tears down the same way.
#[derive(Clone, Default)]
struct ShutdownSignal(Arc<Mutex<Option<OnceValue<()>>>>);

impl ShutdownSignal {
	/// Open a fresh signal, closing any live one first, and return the receiver
	/// the serve loop races its accept loop against.
	fn open(&self) -> OnceValueRx<()> {
		self.close();
		let (signal, receiver) = OnceValue::<()>::oneshot();
		*self.0.lock().unwrap() = Some(signal);
		receiver
	}

	/// Signal and clear the live signal. Idempotent: a missing one is a no-op.
	fn close(&self) {
		// take and drop the guard before signalling: a single-threaded executor
		// polls the woken serve loop inline, and it may reach this slot again.
		let signal = self.0.lock().unwrap().take();
		if let Some(signal) = signal {
			signal.signal(());
		}
	}
}

/// The bind knobs a booting server overlays onto its own declared config, read
/// from the boot request.
///
/// The request alone, never the environment: env already fed each server's
/// [`Default`] through [`BootstrapConfig::get`], and a markup-declared field
/// out-ranks env, so consulting it again here would invert that precedence. A
/// boot request is not a process launch, so this is a plain params type rather
/// than a second [`BootstrapConfig`]; `server_params_match_bootstrap_knobs` pins
/// its names to the flags a deploy renders, which is the drift the process config
/// exists to prevent.
#[derive(Debug, Default, Clone, PartialEq, Reflect)]
#[reflect(Default)]
pub struct ServerParams {
	/// The address to bind, overriding the declared host.
	pub host: Option<String>,
	/// The http listener port, overriding the declared port.
	pub port: Option<u16>,
	/// The ssh listener port, overriding the declared port.
	pub ssh_port: Option<u16>,
	/// The route a freshly-opened tui/ssh surface navigates to, overriding the
	/// request path.
	pub path: Option<String>,
}

impl ServerParams {
	/// The boot knobs `request` carries.
	pub fn from_request(request: &Request) -> Result<Self> {
		request.params().parse_reflect()
	}

	/// The `--host` override as IPv4 octets, the form the server components hold.
	/// A malformed address errors; an IPv6 one warns and yields `None`, per
	/// [`BootstrapConfig::ipv4_octets`].
	pub fn host_octets(&self) -> Result<Option<[u8; 4]>> {
		self.host
			.as_deref()
			.map(|host| {
				host.parse::<IpAddr>()
					.map_err(|err| bevyhow!("invalid --host `{host}`: {err}"))
			})
			.transpose()?
			.and_then(BootstrapConfig::ipv4_octets)
			.xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;

	/// The invariant that lets a boot read plain params instead of a second
	/// [`BootstrapConfig`]: every flag the deploy renders for a bind knob is a
	/// flag [`ServerParams`] reads back. A renamed knob fails here rather than
	/// silently leaving a deployed server on the wrong port.
	#[beet_core::test]
	fn server_params_match_bootstrap_knobs() {
		let argv = BootstrapConfig {
			host: Some("0.0.0.0".parse().unwrap()),
			http_port: Some(9090),
			ssh_port: Some(2222),
			path: Some("docs/form".into()),
			..default()
		}
		.to_argv()
		.unwrap();
		let request = Request::from_cli_args(CliArgs::parse_tokens(argv));
		ServerParams::from_request(&request)
			.unwrap()
			.xpect_eq(ServerParams {
				host: Some("0.0.0.0".into()),
				port: Some(9090),
				ssh_port: Some(2222),
				path: Some("docs/form".into()),
			});
	}

	/// Two servers spread on one entity contribute to the one [`RunningSet`], so
	/// a single call starts both rather than either clobbering the other's slot.
	// see the NATIVE_ONLY note in `http_server`
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test]
	async fn starts_every_declared_server() {
		crate::server::http_server::stub_backend();
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin));
		let (channel_server, _client) = ChannelHttpServer::new();
		let entity = app
			.world_mut()
			.spawn((HttpServer::new(0), channel_server))
			.id();
		let served = Store::new(false);
		let recorder = served;
		app.world_mut().entity_mut(entity).observe_any(
			move |ev: On<RunningSetStarted>| {
				recorder.set(ev.started == 2 && ev.declined == 0)
			},
		);
		app.world_mut().entity_mut(entity).run_async_local(
			|server| async move {
				server.call::<Request, Response>(Request::get("/")).await?;
				Ok(())
			},
		);
		app_ext::update_until(&mut app, |_| served.get())
			.await
			.xpect_true();
	}

	/// `--server` selects through the same [`ServerFilter`] grammar the process
	/// config renders, and an unselected boot falls back to the server's own
	/// `default_boot`.
	#[beet_core::test]
	fn selects_reads_the_filter() {
		let selects = |args: &str, name: &str, default_boot: bool| {
			ServerFilter::selects(
				Request::from_cli_str(args).params(),
				name,
				default_boot,
			)
		};
		selects("--server=http,ssh", "http", false).xpect_true();
		selects("--server=http", "cli", true).xpect_false();
		// a bare `--server` is present but unconstrained
		selects("--server", "cli", false).xpect_true();
		// absent, the server's own `default_boot` decides
		selects("", "http", true).xpect_true();
		selects("", "http", false).xpect_false();
	}
}
