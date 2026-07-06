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
use bevy::ecs::schedule::common_conditions;
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
			request_selects_server(boot, "ssh", true),
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
			.add_systems(Update, close_session_on_ctrl_c)
			// on a graceful shutdown restore every client terminal before the
			// process exits, the multi-tenant counterpart of `restore_terminals`.
			.add_systems(
				PostUpdate,
				restore_sessions_on_exit
					.after(ssh_write)
					.run_if(common_conditions::on_message::<AppExit>),
			);
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
			// diagnostic: everything the pty request carries about the client
			// terminal, plus the resulting graphics detection. The `terminal` name
			// and pixel window size are the only signals a kitty/ghostty client can
			// forward over SSH, so dump them to tune `KittyGraphicsSupport`.
			let graphics =
				KittyGraphicsSupport::from_pty(&pty.terminal, pty.window.pixels);
			info!(
				"ssh pty request: terminal={:?} cells={:?} pixels={:?} \
				 terminal_modes={:?} → kitty_graphics={}",
				pty.terminal,
				pty.window.cells,
				pty.window.pixels,
				pty.terminal_modes,
				graphics.enabled
			);
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
				// graphics support is the *client's* capability: detect it from the
				// pty's forwarded terminal name, not the server's own env, so a kitty
				// client renders rasters while a plain terminal keeps the alt marker.
				graphics,
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

/// System: ctrl+c closes only the originating session, never the process. It
/// restores the client terminal (the leave sequences) *before* sending
/// [`SshEvent::Close`], so the client is not left in raw mouse-tracking mode; the
/// resulting [`SshRecv`] close despawns its surface.
fn close_session_on_ctrl_c(
	mut keys: MessageReader<KeyboardInput>,
	connections: Query<(), With<SshPeerInfo>>,
	mut surfaces: Query<&mut Terminal>,
	mut channels: Query<&mut ChannelTerminal>,
	mut commands: Commands,
) -> Result {
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
			// restore the client terminal, then close: the russh send loop forwards
			// the leave sequences (a `Data` event) before it processes the `Close`,
			// so the client receives them before the channel shuts.
			restore_session(
				window,
				&mut surfaces,
				&mut channels,
				&mut commands,
			)?;
			commands
				.entity(window)
				.trigger_target(SshSend(SshEvent::Close(None)));
		}
	}
	Ok(())
}

/// System: on a graceful [`AppExit`] (server shutdown), restore and close every
/// SSH session, so a clean exit does not leave clients stuck in the alternate
/// screen / raw mouse-tracking mode. Best-effort: a hard kill (`SIGKILL`, or a
/// `SIGINT` that bypasses [`AppExit`]) gives the process no chance to send the
/// leave sequences, so the client must reset itself.
fn restore_sessions_on_exit(
	connections: Query<Entity, With<SshPeerInfo>>,
	mut surfaces: Query<&mut Terminal>,
	mut channels: Query<&mut ChannelTerminal>,
	mut commands: Commands,
) -> Result {
	for connection in connections.iter() {
		restore_session(connection, &mut surfaces, &mut channels, &mut commands)?;
		commands
			.entity(connection)
			.trigger_target(SshSend(SshEvent::Close(None)));
	}
	Ok(())
}

/// Emit a connection's terminal-restore sequences to its SSH client: exit the
/// alternate screen, disable mouse tracking, and show the cursor (the inverse of
/// the setup `ChannelTerminal::new` wrote). Raw mode is the client's to restore (its
/// `TerminalConfig` has `raw_mode: false`), so this writes only the in-band escapes.
///
/// Writes the leave sequences into the surface, drains them, and sends them to the
/// client as a [`SshSend`] data event; the caller closes the channel afterwards.
/// A connection without a built surface (eg before its pty) is a no-op.
fn restore_session(
	connection: Entity,
	surfaces: &mut Query<&mut Terminal>,
	channels: &mut Query<&mut ChannelTerminal>,
	commands: &mut Commands,
) -> Result {
	let Ok(mut surface) = surfaces.get_mut(connection) else {
		return Ok(());
	};
	surface.restore_config()?;
	surface.flush()?;
	if let Ok(mut channel) = channels.get_mut(connection) {
		let output = channel.drain_write();
		if !output.is_empty() {
			commands
				.entity(connection)
				.trigger_target(SshSend(SshEvent::bytes(output)));
		}
	}
	Ok(())
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
	/// and request a pty of `size` on a plain `xterm`, as the russh accept loop would.
	fn open_connection(app: &mut App, router: Entity, size: UVec2) -> Entity {
		open_connection_with(app, router, size, "xterm")
	}

	/// [`open_connection`] with a chosen pty terminal name, so a test can drive the
	/// per-session client-capability detection (eg graphics support).
	fn open_connection_with(
		app: &mut App,
		router: Entity,
		size: UVec2,
		terminal: &str,
	) -> Entity {
		let connection = app
			.world_mut()
			.spawn((SshPeerInfo::default(), ChildOf(router)))
			.id();
		app.world_mut()
			.entity_mut(connection)
			.trigger_target(SshRecv(SshEvent::RequestPty(RequestPty {
				terminal: terminal.into(),
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

	/// Each session's kitty-graphics support comes from its pty's forwarded
	/// terminal name (the client's capability), not the server's own `TERM`: a
	/// kitty client enables rasters while a plain xterm keeps the `[image]: alt`
	/// marker. The image-loading regression — over SSH the server env never names
	/// kitty, so the global-resource detection disabled graphics for every session.
	#[beet_core::test]
	async fn pty_terminal_sets_per_session_graphics_support() {
		let mut app = ssh_tui_app();
		let router = spawn_router(&mut app);
		let size = UVec2::new(40, 8);
		let kitty = open_connection_with(&mut app, router, size, "xterm-kitty");
		let plain =
			open_connection_with(&mut app, router, size, "xterm-256color");
		let enabled = |app: &App, connection: Entity| {
			app.world()
				.entity(connection)
				.get::<KittyGraphicsSupport>()
				.map(|support| support.enabled)
		};
		enabled(&app, kitty).xpect_eq(Some(true));
		enabled(&app, plain).xpect_eq(Some(false));
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

	/// Regression: the real beet-site path — a [`BsxLayout`]-wrapped [`BlobScene`]
	/// route (`RoutesDir` `.bsx` page + `BsxLayout{template:"Layout"}`) — must render
	/// once per SSH session. The route's content lives on the shared route entity,
	/// transcluded into each session's layout; a second session must not double it.
	#[cfg(feature = "bsx")]
	#[beet_core::test]
	async fn bsx_layout_blob_scene_renders_once_per_session() {
		let mut app = ssh_tui_app();
		let store = BlobStore::temp();
		store
			.insert(
				&"index.html".into(),
				"<div><h3>Mind your step</h3></div>".to_string(),
			)
			.await
			.unwrap();
		let mut registry = BsxTemplateRegistry::default();
		registry
			.insert_source(
				"Layout",
				"<html><body><main><Slot/></main></body></html>",
			)
			.unwrap();
		app.world_mut().insert_resource(registry);
		let router = app
			.world_mut()
			.spawn((
				store,
				Router,
				BsxLayout::default(),
				SshTuiServer,
				OpeningRoute(Url::parse("")),
				children![route("", BlobScene::new("index.html"))],
			))
			.flush();
		let first = open_connection(&mut app, router, UVec2::new(40, 8));
		drive_until(&mut app, first, "Mind your step");
		let second = open_connection(&mut app, router, UVec2::new(40, 8));
		drive_until(&mut app, second, "Mind your step");

		frame(&mut app, second)
			.matches("Mind your step")
			.count()
			.xpect_eq(1);
		frame(&mut app, first)
			.matches("Mind your step")
			.count()
			.xpect_eq(1);
	}

	/// Regression: on ctrl+c the session restores the client terminal (disable mouse
	/// tracking, show the cursor, leave the alternate screen) *before* it closes, so
	/// the client is not left spewing escape sequences on mouse movement. The local
	/// `TuiServer` restores via `restore_terminals` on `AppExit`; the SSH path emits
	/// the same leave sequences in-band over the channel ahead of the close.
	#[beet_core::test]
	async fn ctrl_c_restores_client_terminal_before_close() {
		let mut app = ssh_tui_app();
		let router = spawn_router(&mut app);
		let connection = open_connection(&mut app, router, UVec2::new(40, 8));
		drive_until(&mut app, connection, "Alpha page");

		// record, in order, every event the session would send to its client.
		let sent =
			std::sync::Arc::new(std::sync::Mutex::new(Vec::<SshEvent>::new()));
		let recorder = sent.clone();
		app.world_mut().entity_mut(connection).observe_any(
			move |ev: On<SshSend>| {
				recorder.lock().unwrap().push(ev.event().inner().clone());
			},
		);

		// ctrl+c from this connection's window (ctrl and c pressed together).
		for key_code in [KeyCode::ControlLeft, KeyCode::KeyC] {
			app.world_mut().write_message(KeyboardInput {
				key_code,
				logical_key: bevy::input::keyboard::Key::Character("c".into()),
				state: ButtonState::Pressed,
				text: None,
				repeat: false,
				window: connection,
			});
		}
		app.update();

		let events = sent.lock().unwrap().clone();
		// a data event carrying the leave sequences (disable any-motion mouse + show
		// cursor) was sent ...
		let restore = events
			.iter()
			.position(|ev| match ev {
				SshEvent::Data(bytes) => {
					let text = String::from_utf8_lossy(bytes);
					text.contains("\x1b[?1003l") && text.contains("\x1b[?25h")
				}
				_ => false,
			})
			.expect("ctrl+c did not emit the terminal restore sequences");
		// ... and it preceded the channel close.
		let close = events
			.iter()
			.position(|ev| matches!(ev, SshEvent::Close(_)))
			.expect("ctrl+c did not close the session");
		(restore < close).xpect_true();
	}

	// ================================================================
	// Multi-tenant INPUT regression coverage.
	//
	// The plain `ssh_tui_app` above omits `CharcellTuiPlugin`, so it has no
	// input pipeline (no `pointer_input`, disclosure observers, or
	// `sync_sidebar_breakpoint`) and its multi-session tests drive navigation
	// directly. The harness below adds the full live-TUI stack the real
	// `beet serve site --server=ssh` runs and drives real SGR mouse bytes across
	// two concurrent sessions, so cross-session state leaks are actually
	// exercised.
	// ================================================================

	use bevy::math::IVec2;

	/// An SGR mouse sequence: button `b` at 1-indexed cell `(col+1, row+1)`,
	/// pressed (`M`) or released (`m`). Mirrors the private helper the `beet_ui`
	/// hit-test tests use.
	fn sgr(b: u32, col: u32, row: u32, pressed: bool) -> Vec<u8> {
		let m = if pressed { 'M' } else { 'm' };
		format!("\x1b[<{b};{};{}{m}", col + 1, row + 1).into_bytes()
	}

	/// The live-TUI stack the real `beet serve site --server=ssh` runs, including
	/// the input pipeline the plain [`ssh_tui_app`] omits: [`CharcellTuiPlugin`]
	/// brings `pointer_input`/`scroll_input`, the disclosure observers, and
	/// `sync_sidebar_breakpoint`, so concurrent SGR input is exercised.
	fn ssh_tui_live_app() -> App {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			RouterPlugin,
			NavigatorPlugin,
			LivePagePlugin,
			SshTuiPlugin,
			CharcellTuiPlugin,
		));
		app
	}

	/// A per-session document layout with the responsive-drawer chrome: a menu
	/// button wired to `#sidebar` via `aria-controls`, the `<nav id="sidebar">`
	/// rail, and the route content transcluded into `<main>` — the shape every
	/// live page has once wrapped by [`BaseLayout`].
	#[template]
	fn DrawerLayout() -> impl Bundle {
		rsx! {
			<body>
				<button aria-controls="sidebar">"M"</button>
				<nav id="sidebar">"NAV"</nav>
				<main><Slot/></main>
			</body>
		}
	}

	/// A router carrying the drawer layout + the SSH-TUI server, serving one home
	/// route and opening on it.
	fn spawn_drawer_router(app: &mut App) -> Entity {
		app.world_mut()
			.spawn((
				Router,
				BaseLayout::<DrawerLayout>::default(),
				SshTuiServer,
				OpeningRoute(Url::parse("home")),
				children![render_action::fixed_func_route("home", || {
					rsx! { <p>"Home page"</p> }
				})],
			))
			.flush()
	}

	/// The string value of attribute `key` on element `entity`, if present.
	fn attr(app: &mut App, entity: Entity, key: &str) -> Option<String> {
		app.world_mut()
			.with_state::<(Query<&Attributes>, Query<&Attribute>, Query<&Value>), _>(
				move |(attributes, attr_keys, values)| {
					attributes
						.get(entity)
						.ok()
						.and_then(|attrs| {
							attrs.iter().find(|&attr| {
								attr_keys
									.get(attr)
									.is_ok_and(|attr_key| attr_key.as_str() == key)
							})
						})
						.and_then(|attr| values.get(attr).ok())
						.and_then(|value| value.as_str().ok().map(String::from))
				},
			)
	}

	/// The sole element with `tag` whose owning render surface is `surface`,
	/// resolved through [`SurfaceQuery`] so it works for both per-session chrome
	/// (a `ChildOf` path to the page's [`RenderSurface`]) and Portal-transcluded
	/// route content (which crosses the holder).
	fn element_on(app: &mut App, surface: Entity, tag: &str) -> Entity {
		let world = app.world_mut();
		let matches: Vec<Entity> = world
			.query::<(Entity, &Element)>()
			.iter(world)
			.filter(|(_, element)| element.tag() == tag)
			.map(|(entity, _)| entity)
			.collect();
		world.with_state::<SurfaceQuery, _>(move |surfaces| {
			matches
				.into_iter()
				.find(|&entity| surfaces.surface_of(entity) == Some(surface))
				.unwrap_or_else(|| panic!("no <{tag}> for surface {surface:?}"))
		})
	}

	/// Whether `entity` is `target` or a `ChildOf`-descendant of it.
	fn is_within(world: &World, mut entity: Entity, target: Entity) -> bool {
		loop {
			if entity == target {
				return true;
			}
			match world.get_entity(entity).ok().and_then(|e| e.get::<ChildOf>()) {
				Some(child_of) => entity = child_of.parent(),
				None => return false,
			}
		}
	}

	/// The first painted cell in `surface`'s front buffer owned by `target`'s
	/// subtree, for aiming an SGR click at a rendered element.
	fn cell_of(app: &mut App, surface: Entity, target: Entity) -> Option<IVec2> {
		let painted: Vec<(IVec2, Entity)> = {
			let buffer = app.world().get::<DoubleBuffer>(surface)?;
			let front = buffer.front_buffer();
			let size = front.size();
			(0..size.y)
				.flat_map(|y| (0..size.x).map(move |x| UVec2::new(x, y)))
				.filter_map(|pos| {
					front.get(pos).and_then(|cell| cell.entity).map(|entity| {
						(IVec2::new(pos.x as i32, pos.y as i32), entity)
					})
				})
				.collect()
		};
		painted
			.into_iter()
			.find(|(_, entity)| is_within(app.world(), *entity, target))
			.map(|(pos, _)| pos)
	}

	/// Drive an SGR left-click (hover, press, release) at `cell` on `surface`'s
	/// channel, stepping the app after each so the input pipeline consumes it.
	fn click_at(app: &mut App, surface: Entity, cell: IVec2) {
		let (col, row) = (cell.x as u32, cell.y as u32);
		for seq in [
			sgr(35, col, row, true),
			sgr(0, col, row, true),
			sgr(0, col, row, false),
		] {
			app.world_mut()
				.get_mut::<ChannelTerminal>(surface)
				.unwrap()
				.send_input(&seq)
				.unwrap();
			app.update();
		}
	}

	/// Regression (multi-tenant disclosure): clicking session A's menu button
	/// toggles only A's own sidebar; the idle session B's rail is untouched. The
	/// deploy verifier saw the opposite — A's own drawer stayed closed while the
	/// idle B's drawer opened — i.e. one session's input crossing into another's
	/// state. Driven through the full input pipeline (SGR bytes → hit-test →
	/// disclosure observer), which no prior in-process test exercised.
	#[beet_core::test]
	async fn menu_button_toggles_only_its_own_session_sidebar() {
		let mut app = ssh_tui_live_app();
		let router = spawn_drawer_router(&mut app);
		let session_a = open_connection(&mut app, router, UVec2::new(40, 8));
		let session_b = open_connection(&mut app, router, UVec2::new(40, 8));
		drive_until(&mut app, session_a, "Home page");
		drive_until(&mut app, session_b, "Home page");

		// each session's own rail + menu button (per-session layout entities)
		let nav_a = element_on(&mut app, session_a, "nav");
		let nav_b = element_on(&mut app, session_b, "nav");
		let button_a = element_on(&mut app, session_a, "button");

		let before_a = attr(&mut app, nav_a, "aria-hidden");
		let before_b = attr(&mut app, nav_b, "aria-hidden");

		// click A's menu button via real SGR bytes on A's channel only
		let cell = cell_of(&mut app, session_a, button_a)
			.expect("session A's menu button painted a cell");
		click_at(&mut app, session_a, cell);
		app.update();

		let after_a = attr(&mut app, nav_a, "aria-hidden");
		let after_b = attr(&mut app, nav_b, "aria-hidden");

		// A's own rail toggled ...
		(after_a != before_a).xpect_true();
		// ... and B's rail is exactly as it was (no cross-session leak).
		after_b.xpect_eq(before_b);
	}

	/// Regression (multi-tenant reactive state): incrementing session A's counter
	/// advances only A's own count; the idle session B's count is unchanged — the
	/// deploy's multi-tenant check asserts independent per-session counts. Driven
	/// through the full input pipeline (SGR bytes → hit-test → `bx:click` verb)
	/// over the real shared-content Portal structure: the `bx:scope`/`@doc:count`
	/// reactive document is parsed fresh per session, so each binds its own.
	#[cfg(feature = "bsx")]
	#[beet_core::test]
	async fn counter_increments_only_its_own_session() {
		let mut app = ssh_tui_live_app();
		let store = BlobStore::temp();
		store
			.insert(
				&"counter.bsx".into(),
				r#"<article bx:scope="counter"><p>You have clicked {@doc:count=0} times.</p><button bx:click=increment{ field: @doc:count }>More</button></article>"#
					.to_string(),
			)
			.await
			.unwrap();
		let mut registry = BsxTemplateRegistry::default();
		registry
			.insert_source(
				"Layout",
				"<html><body><main><Slot/></main></body></html>",
			)
			.unwrap();
		app.world_mut().insert_resource(registry);
		let router = app
			.world_mut()
			.spawn((
				store,
				Router,
				BsxLayout::default(),
				SshTuiServer,
				OpeningRoute(Url::parse("counter")),
				children![route("counter", BlobScene::new("counter.bsx"))],
			))
			.flush();
		let session_a = open_connection(&mut app, router, UVec2::new(40, 8));
		let session_b = open_connection(&mut app, router, UVec2::new(40, 8));
		drive_until(&mut app, session_a, "clicked 0 times");
		drive_until(&mut app, session_b, "clicked 0 times");

		// click A's "More" button via real SGR bytes on A's channel only
		let button_a = element_on(&mut app, session_a, "button");
		let cell = cell_of(&mut app, session_a, button_a)
			.expect("session A's counter button painted a cell");
		click_at(&mut app, session_a, cell);

		// A's own count advanced to 1 ...
		drive_until(&mut app, session_a, "clicked 1 times");
		// ... and B's count is untouched (its reactive document is its own).
		frame(&mut app, session_b)
			.xpect_contains("clicked 0 times")
			.xnot()
			.xpect_contains("clicked 1 times");
	}
}
