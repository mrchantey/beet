//! The multi-tenant SSH-TUI server: serves an independent, navigable terminal to
//! every SSH connection, all browsing the same router.
//!
//! The remote, multi-tenant sibling of the single-surface [`TuiServer`]. Where
//! `TuiServer` drives one local stdio terminal, this accepts many SSH connections
//! and gives each its own surface (a [`ChannelTerminal`] + [`page_host`] buffer +
//! an in-world [`Navigator`]), so sessions render, scroll, focus and navigate
//! independently in one process, alongside the [`HttpServer`].

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyCode;
use bevy::input::keyboard::KeyboardInput;
use bevy::math::UVec2;

/// A multi-tenant SSH-TUI server, spread on a router: the boot fan-out whose
/// `--server` selects `"ssh"` boots an [`SshServer`] on the router and serves every
/// connection its own navigable terminal browsing this router.
///
/// A long-running server: it never resolves the boot call, so the host's
/// [`Running<Response>`](beet_action::prelude::Running) parks the process up.
/// Reads `--port` / `--host` from the boot request (defaulting from
/// `BEET_SSH_PORT` / `BEET_HOST`) and the opening `--path` (default home `/`).
/// Add [`SshTuiPlugin`] once for the per-connection behavior. Coexists with an
/// [`HttpServer`] on the same router, so one process answers http and ssh at once.
#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
#[require(ContinueRun<Boot, Response>)]
#[component(on_add = on_add)]
pub struct SshTuiServer;

/// Registers the boot ([`StartRunning<Boot>`]) observer on the router, so the SSH
/// listener boots when the boot fan-out selects `"ssh"`.
fn on_add(mut world: DeferredWorld, cx: HookContext) {
	world.commands().entity(cx.entity).observe_any(on_action_in);
}

/// Boots the SSH listener on the boot fan-out, if `--server` selects `"ssh"`:
/// builds an [`SshServer`] from the request and inserts it on the router (its
/// `on_add` starts the listener), and records the opening route. Never resolves
/// the boot call, so its `Running` parks the process up.
fn on_action_in(ev: On<StartRunning<Boot>>, mut commands: Commands) -> Result {
	let (selected, port, host, opening) = ev.with(|boot| {
		(
			request_selects_server(boot, "ssh"),
			boot.get_param("port").and_then(|port| port.parse().ok()),
			boot.get_param("host").map(|host| {
				if host == "0.0.0.0" {
					[0, 0, 0, 0]
				} else {
					[127, 0, 0, 1]
				}
			}),
			OpeningRoute::from_request(boot),
		)
	})?;
	if !selected {
		return Ok(());
	}
	// the bind config from the request, defaulting from env.
	let mut server = SshServer::default();
	if let Some(port) = port {
		server.port = Some(port);
	}
	if let Some(host) = host {
		server.host = host;
	}
	// the opening route each session navigates to, recorded on the router (the
	// shared mechanism the local TUI server also reads).
	commands.entity(ev.entity).insert((server, opening));
	Ok(())
}

/// Per-connection behavior for the [`SshTuiServer`], added once by the app: spins
/// up a surface per connection, drains each surface's frame to its client, and
/// closes a session on ctrl+c.
///
/// The server component (on the router) boots the listener; this plugin provides
/// the connection lifecycle, mirroring how [`TuiServer`] pairs with the live
/// plugins ([`CharcellTuiPlugin`], [`NavigatorPlugin`], [`LivePagePlugin`]), which
/// an SSH-TUI app must also add.
#[derive(Default)]
pub struct SshTuiPlugin;

impl Plugin for SshTuiPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<SshTuiServer>()
			.add_observer(on_ssh_recv)
			// drain each surface's painted frame to its client after the render
			// pipeline (PostParseTree, which RealtimeParsePlugin runs after Update).
			.add_systems(PostUpdate, ssh_write)
			.add_systems(Update, close_session_on_ctrl_c);
	}
}

/// Observer: drive each SSH connection's surface lifecycle from its [`SshRecv`]
/// stream. Scoped to connections of an [`SshTuiServer`] router, so a plain
/// [`SshServer`] elsewhere is untouched.
fn on_ssh_recv(
	ev: On<SshRecv>,
	peers: Query<&ChildOf, With<SshPeerInfo>>,
	tui_servers: Query<&OpeningRoute, With<SshTuiServer>>,
	mut terminals: Query<&mut ChannelTerminal>,
	mut buffers: Query<&mut DoubleBuffer>,
	mut commands: Commands,
) -> Result {
	let connection = ev.target();
	// only handle connections (carry SshPeerInfo) whose router is an SshTuiServer.
	let Ok(router) = peers.get(connection).map(|child_of| child_of.parent())
	else {
		return Ok(());
	};
	let Ok(opening) = tui_servers.get(router) else {
		return Ok(());
	};
	match ev.event().inner() {
		// wait for the pty before building the surface; its size sizes the buffer.
		SshEvent::Connect => {}
		SshEvent::RequestPty(pty) => {
			// some clients (or a pty with no controlling terminal) report a
			// 0-sized window; fall back to a usable default that a later
			// window-change resizes, so the surface always renders.
			let size = if pty.window.cells.x == 0 || pty.window.cells.y == 0 {
				UVec2::new(80, 24)
			} else {
				pty.window.cells
			};
			// the surface: a channel terminal + the page-host buffer + the
			// in-world navigator, all co-located on the connection entity,
			// browsing this router from the recorded opening route.
			let (channel, terminal) =
				ChannelTerminal::new(TerminalConfig::default());
			commands.entity(connection).insert((
				channel,
				terminal,
				page_host(size),
				Navigator::in_world(router, opening.0.clone()),
			));
		}
		SshEvent::Data(bytes) => {
			if let Ok(mut terminal) = terminals.get_mut(connection) {
				terminal.send_input(bytes)?;
			}
		}
		SshEvent::Resize(size) => {
			// ignore a 0-sized resize (eg a detaching client) so the surface keeps
			// its last usable size rather than blanking.
			if size.cells.x > 0 && size.cells.y > 0 {
				if let Ok(mut buffer) = buffers.get_mut(connection) {
					buffer.resize(size.cells);
				}
			}
		}
		SshEvent::Close(_) => {
			// the navigator is co-located on the connection surface, so despawning
			// the connection tears down the whole session.
			commands.entity(connection).despawn();
		}
		_ => {}
	}
	Ok(())
}

/// System: drain each connection surface's painted frame and send it to the
/// client, after the render pipeline has written it to the channel.
fn ssh_write(
	mut commands: Commands,
	mut query: Query<(Entity, &mut ChannelTerminal)>,
) {
	for (entity, mut terminal) in query.iter_mut() {
		let output = terminal.drain_write();
		if !output.is_empty() {
			commands
				.entity(entity)
				.trigger_target(SshSend(SshEvent::bytes(output)));
		}
	}
}

/// System: ctrl+c closes only the originating session (sends [`SshEvent::Close`]),
/// never the process; the resulting [`SshRecv`] close despawns its surface.
fn close_session_on_ctrl_c(
	mut keys: MessageReader<KeyboardInput>,
	connections: Query<(), With<SshPeerInfo>>,
	mut commands: Commands,
) {
	// group this frame's pressed keys by window: (ctrl seen, c seen).
	let mut per_window = HashMap::<Entity, (bool, bool)>::default();
	for key in keys.read().filter(|key| key.state == ButtonState::Pressed) {
		let entry = per_window.entry(key.window).or_default();
		match key.key_code {
			KeyCode::ControlLeft | KeyCode::ControlRight => entry.0 = true,
			KeyCode::KeyC => entry.1 = true,
			_ => {}
		}
	}
	for (window, (ctrl, c)) in per_window {
		if ctrl && c && connections.contains(window) {
			commands
				.entity(window)
				.trigger_target(SshSend(SshEvent::Close(None)));
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use bevy::math::UVec2;

	/// The live-navigation stack plus the SSH-TUI per-connection plugin, minus the
	/// real socket: connections are simulated by triggering [`SshRecv`] on a child
	/// of an [`SshTuiServer`] router, exactly as the russh task would.
	fn ssh_tui_app() -> App {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			RouterPlugin,
			RealtimeParsePlugin,
			LivePagePlugin,
			NavigatorPlugin,
			SshTuiPlugin,
		));
		app
	}

	/// A router serving two routes, carrying an [`SshTuiServer`] + its opening home.
	fn spawn_router(app: &mut App) -> Entity {
		app.world_mut()
			.spawn((
				Router,
				SshTuiServer,
				OpeningRoute(Url::parse("alpha")),
				children![
					render_action::fixed_func_route("alpha", || {
						rsx! { <p>"Alpha page"</p> }
					}),
					render_action::fixed_func_route("beta", || {
						rsx! { <p>"Beta page"</p> }
					}),
				],
			))
			.flush()
	}

	/// Spawn a simulated SSH connection (a child of `router` with [`SshPeerInfo`])
	/// and request a pty of `size`, as the russh accept loop would.
	fn open_connection(app: &mut App, router: Entity, size: UVec2) -> Entity {
		let connection = app
			.world_mut()
			.spawn((SshPeerInfo { username: None }, ChildOf(router)))
			.id();
		app.world_mut()
			.entity_mut(connection)
			.trigger_target(SshRecv(SshEvent::RequestPty(RequestPty {
				terminal: "xterm".into(),
				window: SshWindowSize {
					cells: size,
					pixels: UVec2::ZERO,
				},
				terminal_modes: Vec::new(),
			})));
		app.update();
		connection
	}

	/// The frame painted into a connection surface's buffer, as plain text.
	///
	/// Reads the front buffer: a Terminal-backed surface paints into the back
	/// buffer then swaps, so after a step the rendered frame is the front one.
	fn frame(app: &mut App, connection: Entity) -> String {
		app.update();
		app.world()
			.get::<DoubleBuffer>(connection)
			.map(|buffer| buffer.front_buffer().render_plain())
			.unwrap_or_default()
	}

	/// Drive the app until `connection`'s frame contains `needle`.
	fn drive_until(app: &mut App, connection: Entity, needle: &str) -> String {
		for _ in 0..200 {
			let frame = frame(app, connection);
			if frame.contains(needle) {
				return frame;
			}
		}
		panic!("ssh surface frame never contained '{needle}'");
	}

	/// A pty request builds the surface (channel terminal + buffer) and an in-world
	/// navigator bound to it, and the opening route paints into the buffer.
	#[beet_core::test]
	async fn pty_request_serves_the_home_page() {
		let mut app = ssh_tui_app();
		let router = spawn_router(&mut app);
		let connection = open_connection(&mut app, router, UVec2::new(40, 8));
		// the surface components landed on the connection entity
		app.world()
			.entity(connection)
			.contains::<ChannelTerminal>()
			.xpect_true();
		app.world()
			.entity(connection)
			.contains::<DoubleBuffer>()
			.xpect_true();
		// and the home route paints into its buffer
		drive_until(&mut app, connection, "Alpha page");
	}

	/// Two concurrent connections each get their own surface and navigate
	/// independently: one stays on home while the other moves to a second route.
	#[beet_core::test]
	async fn two_sessions_navigate_independently() {
		let mut app = ssh_tui_app();
		let router = spawn_router(&mut app);
		let first = open_connection(&mut app, router, UVec2::new(40, 8));
		let second = open_connection(&mut app, router, UVec2::new(40, 8));
		drive_until(&mut app, first, "Alpha page");
		drive_until(&mut app, second, "Alpha page");

		// navigate only the second session to beta (its navigator is co-located on
		// the connection surface)
		let url = Url::parse("beta");
		app.world_mut()
			.entity_mut(second)
			.run_async_local(move |entity| Navigator::navigate_to(entity, url));
		drive_until(&mut app, second, "Beta page");

		// the first session is unchanged: still on alpha, never beta
		frame(&mut app, first)
			.xpect_contains("Alpha page")
			.xnot()
			.xpect_contains("Beta page");
	}
}
