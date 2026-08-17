//! The shared boot, park and shutdown lifecycle every bootable server uses.
//!
//! A server is an entity carrying its marker component plus the
//! [`ContinueRun<Request, Response>`] boot action and [`CallOnLoad`], usually a
//! child of its dispatch host. A long-running server (http, socket, and their
//! channel variants) boots on the [`StartRunning<Request>`] fan-out, parks on
//! its own [`Running<Response>`] keep-alive, and tears down when that `Running`
//! is removed. The only per-server differences are the `--server` selector and
//! the serve-loop launcher, so this captures the rest once: [`BootServer`]
//! supplies those two seams (plus an optional boot-request hook) and
//! [`ServerShutdown<S>`] holds the teardown signal, keyed by the server marker
//! so co-resident servers never clobber a shared one.

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::component::Mutable;
use core::marker::PhantomData;
use core::net::IpAddr;

impl Request {
	/// Whether a server named `name` should boot for `request`, from the boot
	/// request's [`ServerFilter`] (`--server`), else the process
	/// [`BootstrapConfig`]'s (`--server`, else `BEET_SERVER`).
	///
	/// The process-config fallback is why a deployed binary launched with no args
	/// (a lambda bootstrap, a lightsail systemd unit) still selects its
	/// transport: its `BEET_SERVER` reaches the boot even though the synthesized
	/// request carries no flag. Absent both, the server's own `default_boot`
	/// decides: it defaults to `true`, so a bare `beet` brings up every declared
	/// server, and an entry clears it on one (eg a secondary [`HttpServer`]) that
	/// should boot only when `--server` names it.
	pub fn selects_server(
		request: &Request,
		name: &str,
		default_boot: bool,
	) -> Result<bool> {
		ServerFilter::from_params(request.params())
			.as_ref()
			.or(BootstrapConfig::get().server.as_ref())
			.map(|filter| filter.passes(name))
			.unwrap_or(default_boot)
			.xok()
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
	fn apply_boot(&mut self, _request: &Request) -> Result { Ok(()) }

	/// Whether this server boots when neither `--server` nor `BEET_SERVER` is
	/// given. Defaults to `true` — a bare `beet` brings up the declared server;
	/// override to `false` for one that must be named explicitly.
	fn default_boot(&self) -> bool { true }
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
	/// Register the shared boot + teardown observers on the server entity: the one
	/// place the observer pair is wired. Each server's `on_add` hook is
	/// `hook_ext::entity_hook(ServerShutdown::<Marker>::add_observers)`.
	pub fn add_observers(entity: &mut EntityCommands) {
		entity
			.observe_any(boot_server::<S>)
			.observe_any(teardown_server::<S>);
	}

	/// Whether the teardown signal is still live (booted, not yet torn down). The
	/// bounded booted condition the server tests drive to.
	pub fn is_live(&self) -> bool { self.signal.is_some() }
}

/// Boots `S` on the boot fan-out, if `--server` selects [`BootServer::SELECTOR`].
/// Overlays the boot request onto the server config, stores the shutdown sender on
/// the server entity, then queues the serve loop, handing it the receiver. Reads the
/// boot without consuming it (never the taker), and never resolves the boot call, so
/// the server's [`Running<Response>`] parks the process.
fn boot_server<S: BootServer>(
	ev: On<StartRunning<Request>>,
	mut servers: Query<&mut S>,
	mut commands: Commands,
) -> Result {
	let entity = ev.entity;
	let selected = ev.with(|request| -> Result<bool> {
		let Ok(mut server) = servers.get_mut(entity) else {
			return Ok(false);
		};
		let selected =
			Request::selects_server(request, S::SELECTOR, server.default_boot())?;
		if selected {
			server.apply_boot(request)?;
		}
		selected.xok()
	})??;
	if !selected {
		return Ok(());
	}
	// store the shutdown sender on the server entity; hand the receiver to the serve loop.
	let (signal, shutdown) = OnceValue::<()>::oneshot();
	commands
		.entity(entity)
		// flags the boot as served, so `exit_if_no_server` lets it park
		.insert((ServerBooted, ServerShutdown::<S> {
			signal: Some(signal),
			_marker: PhantomData,
		}))
		.queue_async_local(move |entity| S::serve(entity, shutdown));
	Ok(())
}

/// Tears down `S` when the server's [`Running<Response>`] is removed (a reload, an
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

#[cfg(test)]
mod test {
	use crate::prelude::*;
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

	/// `--server` selects through the same [`ServerFilter`] grammar the process
	/// config renders, and an unselected boot falls back to the server's own
	/// `default_boot`.
	#[beet_core::test]
	fn selects_server_reads_the_filter() {
		let selects = |args: &str, name: &str, default_boot: bool| {
			Request::selects_server(
				&Request::from_cli_str(args),
				name,
				default_boot,
			)
			.unwrap()
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
