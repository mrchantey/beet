//! Open N concurrent SSH TUI sessions against a server and report load stats:
//! how many connected, how many painted a first frame, total bytes received, and
//! how long it took. The load generator for the multi-tenant SSH-TUI server.
//!
//! ```sh
//! # start a server (in another terminal):
//! cargo run --example ssh_tui_site --features ssh_tui,http_server,markdown
//! # then hammer it:
//! cargo run --example ssh_stress --features ssh_client -- --count=50
//! cargo run --example ssh_stress --features ssh_client -- --count=200 --addr=1.2.3.4:8322
//! ```
//!
//! Each session connects anonymously, requests an 80x24 pty + shell, then counts
//! the rendered frame bytes that stream back, so a deployed server's behavior
//! under many concurrent terminals (and Fargate autoscaling) can be observed.

use beet::prelude::*;
use bevy::math::UVec2;

/// How many sessions to open and where.
#[derive(Resource)]
struct StressConfig {
	count: usize,
	addr: String,
}

/// Live tallies across all sessions.
#[derive(Resource, Default)]
struct StressStats {
	connected: usize,
	first_frame: usize,
	bytes: usize,
	closed: usize,
}

/// Marks a session that has received its first frame, so each is counted once.
#[derive(Component)]
struct Painted;

fn main() -> Result {
	let args = CliArgs::parse_env();
	let count = args
		.params
		.get("count")
		.and_then(|count| count.parse().ok())
		.unwrap_or(20);
	let addr = args
		.params
		.get("addr")
		.map(|addr| addr.to_string())
		.unwrap_or_else(|| format!("127.0.0.1:{}", DEFAULT_SSH_PORT));

	App::new()
		.add_plugins((MinimalPlugins, LogPlugin::default(), AsyncPlugin::default()))
		.insert_resource(StressConfig { count, addr })
		.init_resource::<StressStats>()
		.add_systems(Startup, spawn_sessions)
		.add_observer(on_recv)
		.add_systems(Update, report_and_exit)
		.run();
	Ok(())
}

/// Spawn `count` anonymous SSH sessions at once.
fn spawn_sessions(mut commands: Commands, config: Res<StressConfig>) {
	info!("opening {} ssh sessions to {}", config.count, config.addr);
	for _ in 0..config.count {
		commands.spawn(SshSession::insert_anon(&config.addr));
	}
}

/// Per-session: on connect request a pty + shell (start the TUI); on data tally
/// the frame bytes and count a first frame; on close tally the disconnect.
fn on_recv(
	ev: On<SshRecv>,
	painted: Query<(), With<Painted>>,
	mut stats: ResMut<StressStats>,
	mut commands: Commands,
) {
	let session = ev.target();
	match ev.event().inner() {
		SshEvent::Connect => {
			stats.connected += 1;
			commands.entity(session).trigger_target(SshSend(
				SshEvent::RequestPty(RequestPty {
					terminal: "xterm-256color".into(),
					window: SshWindowSize {
						cells: UVec2::new(80, 24),
						pixels: UVec2::ZERO,
					},
					terminal_modes: Vec::new(),
				}),
			));
			commands
				.entity(session)
				.trigger_target(SshSend(SshEvent::RequestShell));
		}
		SshEvent::Data(bytes) => {
			stats.bytes += bytes.len();
			// count the first painted frame per session exactly once.
			if !painted.contains(session) {
				stats.first_frame += 1;
				commands.entity(session).insert(Painted);
			}
		}
		SshEvent::Close(_) => stats.closed += 1,
		_ => {}
	}
}

/// Report progress each second and exit once every session has painted (or after
/// a 30s ceiling), printing the final tally.
fn report_and_exit(
	config: Res<StressConfig>,
	stats: Res<StressStats>,
	mut exit: MessageWriter<AppExit>,
	mut start: Local<Option<Instant>>,
	mut last_log: Local<Option<Instant>>,
) {
	let start = start.get_or_insert_with(Instant::now);
	let elapsed = start.elapsed();
	// once-a-second progress line.
	let log_now = last_log
		.map(|last| last.elapsed().as_secs() >= 1)
		.unwrap_or(true);
	if log_now {
		*last_log = Some(Instant::now());
		info!(
			"connected {}/{}, painted {}, {} bytes, {:.1}s",
			stats.connected,
			config.count,
			stats.first_frame,
			stats.bytes,
			elapsed.as_secs_f32(),
		);
	}
	let done = stats.first_frame >= config.count;
	if done || elapsed.as_secs() >= 30 {
		cross_log!(
			"stress done: {}/{} connected, {}/{} painted, {} bytes in {:.1}s",
			stats.connected,
			config.count,
			stats.first_frame,
			config.count,
			stats.bytes,
			elapsed.as_secs_f32(),
		);
		exit.write(AppExit::Success);
	}
}
