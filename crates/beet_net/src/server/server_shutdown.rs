//! The shared park-and-shutdown machinery every bootable server uses.
//!
//! A long-running server (http, socket, and their channel variants) boots on the
//! [`StartRunning<Boot>`] fan-out, parks on the host's [`Running<Response>`]
//! keep-alive, and tears down when that `Running` is removed. The only per-server
//! differences are the `--server` selector and the serve-loop launcher, so this
//! captures the rest once: [`BootServer`] supplies those two seams (plus an
//! optional boot-request hook) and [`ServerShutdown<S>`] holds the teardown signal,
//! keyed by the server marker so co-resident servers never clobber a shared one.

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::component::Mutable;
use core::marker::PhantomData;

/// A bootable, parking server: supplies the `--server` selector and the serve-loop
/// launcher the shared [`ServerShutdown<Self>`] machinery drives.
///
/// The four built-in servers ([`HttpServer`], [`SocketServer`] and their channel
/// variants) implement it; a downstream server does too rather than re-deriving the
/// boot/teardown observer pair.
pub trait BootServer: Component<Mutability = Mutable> {
	/// The `--server` selector value that boots this server (eg `"http"`).
	const SELECTOR: &'static str;

	/// Launch the serve loop on a started host, handed the `shutdown` receiver it
	/// owns its teardown on (it stops accepting and drops its listener when the
	/// signal resolves). Local (never `Send`): the loop is thread-bound.
	fn serve(
		entity: AsyncEntity,
		shutdown: OnceValueRx<()>,
	) -> LocalBoxedFuture<'static, Result>;

	/// Overlay the boot request onto the server config before the backend reads it.
	/// Default: a no-op; [`HttpServer`] overrides it to apply `--port` / `--host`.
	fn apply_boot(&mut self, _boot: &Request) {}
}

/// Shutdown signal for a running server of marker `S`: [`boot_server`] stores the
/// sender on the host and hands the receiver to the serve loop, and
/// [`teardown_server`] signals it when the host's [`Running<Response>`] is removed so
/// the backend stops accepting and drops its listener. A no_std one-shot channel, so
/// an embedded backend tears down the same way.
///
/// Keyed by the marker `S` so co-resident servers (eg `HttpServer` + `SocketServer`
/// on one Router) each hold their own, never clobbering a shared signal. Replaced on
/// each boot, so a reboot installs a fresh one.
#[derive(Component)]
pub struct ServerShutdown<S: BootServer> {
	signal: Option<OnceValue<()>>,
	_marker: PhantomData<fn() -> S>,
}

impl<S: BootServer> ServerShutdown<S> {
	/// Register the shared boot + teardown observers on the host: the one place the
	/// observer pair is wired. Each server's `on_add` hook calls this with its marker.
	pub fn add_observers(world: &mut DeferredWorld, entity: Entity) {
		world
			.commands()
			.entity(entity)
			.observe_any(boot_server::<S>)
			.observe_any(teardown_server::<S>);
	}

	/// Whether the teardown signal is still live (booted, not yet torn down). The
	/// bounded booted condition the server tests drive to.
	pub fn is_live(&self) -> bool { self.signal.is_some() }
}

/// Boots `S` on the boot fan-out, if `--server` selects [`BootServer::SELECTOR`].
/// Overlays the boot request onto the server config, stores the shutdown sender on
/// the host, then queues the serve loop, handing it the receiver. Reads the boot
/// without consuming it (never the taker), and never resolves the boot call, so the
/// host's [`Running<Response>`] parks the process.
fn boot_server<S: BootServer>(
	ev: On<StartRunning<Boot>>,
	mut servers: Query<&mut S>,
	mut commands: Commands,
) -> Result {
	let entity = ev.entity;
	let selected = ev.with(|boot| {
		let selected = request_selects_server(boot, S::SELECTOR);
		if selected && let Ok(mut server) = servers.get_mut(entity) {
			server.apply_boot(boot);
		}
		selected
	})?;
	if !selected {
		return Ok(());
	}
	// store the shutdown sender on the host; hand the receiver to the serve loop.
	let (signal, shutdown) = oneshot::<()>();
	commands
		.entity(entity)
		.insert(ServerShutdown::<S> {
			signal: Some(signal),
			_marker: PhantomData,
		})
		.queue_async_local(move |entity| S::serve(entity, shutdown));
	Ok(())
}

/// Tears down `S` when the host's [`Running<Response>`] is removed (a reload, an
/// interrupt, or a despawn, since Bevy runs remove hooks on despawn): signals the
/// shutdown channel so the backend stops accepting and drops its listener.
/// Cause-agnostic, so any teardown closes the socket. Idempotent: a missing handle
/// is a no-op.
fn teardown_server<S: BootServer>(
	ev: On<Remove, Running<Response>>,
	mut shutdowns: Query<&mut ServerShutdown<S>>,
) {
	if let Ok(mut shutdown) = shutdowns.get_mut(ev.event().event_target())
		&& let Some(signal) = shutdown.signal.take()
	{
		signal.signal(());
	}
}
