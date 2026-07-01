//! Load generator for the multi-tenant SSH-TUI server. Two modes:
//!
//! - **connect-storm** (default): open N sessions at once, count how many
//!   connected and painted a first frame, total bytes, and how long it took,
//!   then exit. Measures connect + first-frame throughput.
//! - **steady-state** (`--hold`): open N sessions, keep them resident, and on a
//!   cadence drive each one with navigation/scroll/hover input, measuring
//!   input-to-frame latency under sustained interaction. This is the regime that
//!   sets per-task capacity: an idle-but-resident session still costs the server
//!   a per-frame layout/paint pass, and a driven one costs a frame round-trip.
//!
//! ```sh
//! # start a server (in another terminal), capped at 30fps:
//! cargo run --example ssh_tui_site --features ssh_tui,http_server,markdown
//! # connect-storm: how fast can N sessions connect + paint?
//! cargo run --example ssh_stress --features ssh_client -- --count=50
//! # steady-state: hold N sessions for 20s, driving 2 inputs/sec each:
//! cargo run --example ssh_stress --features ssh_client -- --hold --count=50 --hold-secs=20 --input-hz=2
//! cargo run --example ssh_stress --features ssh_client -- --hold --count=200 --addr=1.2.3.4:8339
//! ```
//!
//! Each session connects anonymously and requests an 80x24 pty + shell.

use beet::prelude::*;
use bevy::math::UVec2;

/// What to open, where, and how to drive it.
#[derive(Resource)]
struct StressConfig {
	count: usize,
	addr: String,
	/// Steady-state mode: hold sessions open and drive them, rather than
	/// connect-then-exit.
	hold: bool,
	/// In hold mode, how long to sustain the load before reporting and exiting.
	hold_for: Duration,
	/// In hold mode, inputs sent per session per second.
	input_hz: f32,
}

/// Live tallies across all sessions.
#[derive(Resource, Default)]
struct StressStats {
	connected: usize,
	first_frame: usize,
	bytes: usize,
	closed: usize,
	/// Total driving inputs sent (hold mode).
	inputs_sent: usize,
	/// Input-to-frame latency samples in milliseconds (hold mode): the gap from
	/// sending an input to the next frame that streamed back for that session.
	latencies_ms: Vec<f32>,
}

/// Marks a session that has received its first frame, so each is counted once.
#[derive(Component)]
struct Painted;

/// Per-session driver state (hold mode): the next input to send (round-robin)
/// and the timestamp of the last input awaiting a frame, for latency.
#[derive(Component, Default)]
struct Driver {
	next: usize,
	pending_since: Option<Instant>,
}

fn main() -> Result {
	let args = CliArgs::parse_env();
	let count = parse_param(&args, "count").unwrap_or(20);
	let addr = args
		.params
		.get("addr")
		.map(|addr| addr.to_string())
		.unwrap_or_else(|| format!("127.0.0.1:{}", DEFAULT_SSH_PORT));
	let hold = args.params.contains_key("hold");
	let hold_for = Duration::from_secs_f32(
		parse_param(&args, "hold-secs").unwrap_or(20.0),
	);
	let input_hz = parse_param(&args, "input-hz").unwrap_or(2.0);

	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			AsyncPlugin::default(),
		))
		.insert_resource(StressConfig {
			count,
			addr,
			hold,
			hold_for,
			input_hz,
		})
		.init_resource::<StressStats>()
		.add_systems(Startup, spawn_sessions)
		.add_observer(on_recv)
		.add_systems(Update, (drive_sessions, report_and_exit).chain())
		.run();
	Ok(())
}

/// Parse a `--key=value` param into any `FromStr` type.
fn parse_param<T: core::str::FromStr>(args: &CliArgs, key: &str) -> Option<T> {
	args.params.get(key).and_then(|value| value.parse().ok())
}

/// Spawn `count` anonymous SSH sessions at once.
fn spawn_sessions(mut commands: Commands, config: Res<StressConfig>) {
	info!(
		"opening {} ssh sessions to {} ({})",
		config.count,
		config.addr,
		if config.hold {
			"steady-state"
		} else {
			"connect-storm"
		}
	);
	for _ in 0..config.count {
		commands.spawn(SshSession::insert_anon(&config.addr));
	}
}

/// Per-session: on connect request a pty + shell (start the TUI); on data tally
/// the frame bytes, count a first frame, and (hold mode) resolve a pending input
/// latency; on close tally the disconnect.
fn on_recv(
	ev: On<SshRecv>,
	config: Res<StressConfig>,
	mut drivers: Query<&mut Driver>,
	is_painted: Query<(), With<Painted>>,
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
			// count the first painted frame per session exactly once, and in hold
			// mode arm the session as a driver target.
			if !is_painted.contains(session) {
				stats.first_frame += 1;
				commands.entity(session).insert(Painted);
				if config.hold {
					commands.entity(session).insert(Driver::default());
				}
			}
			// a frame arriving after a driving input closes the latency sample.
			if let Ok(mut driver) = drivers.get_mut(session) {
				if let Some(sent) = driver.pending_since.take() {
					stats
						.latencies_ms
						.push(sent.elapsed().as_secs_f32() * 1000.0);
				}
			}
		}
		SshEvent::Close(_) => stats.closed += 1,
		_ => {}
	}
}

/// Steady-state driver (hold mode): on a fixed cadence, send each resident
/// session one navigation/scroll/hover input, round-robin, and arm a latency
/// timer. A no-op outside hold mode.
fn drive_sessions(
	config: Res<StressConfig>,
	mut stats: ResMut<StressStats>,
	mut drivers: Query<(Entity, &mut Driver)>,
	mut commands: Commands,
	mut last_tick: Local<Option<Instant>>,
) {
	if !config.hold {
		return;
	}
	let interval = Duration::from_secs_f32(1.0 / config.input_hz.max(0.1));
	let tick_now = last_tick
		.map(|last| last.elapsed() >= interval)
		.unwrap_or(true);
	if !tick_now {
		return;
	}
	*last_tick = Some(Instant::now());

	for (session, mut driver) in drivers.iter_mut() {
		let input = &DRIVE_INPUTS[driver.next % DRIVE_INPUTS.len()];
		driver.next += 1;
		// latency is measured input-to-frame for inputs that actually repaint (a
		// navigation). Re-arm the clock on every frame-producing input so a no-op
		// input (scroll/hover on content that fits) can't inflate the next sample,
		// and an un-answered arm is overwritten rather than measuring a stale gap.
		if input.repaints {
			driver.pending_since = Some(Instant::now());
		}
		stats.inputs_sent += 1;
		commands
			.entity(session)
			.trigger_target(SshSend(SshEvent::bytes(input.bytes.to_vec())));
	}
}

/// One driving input: the bytes to send and whether it is expected to repaint
/// (so latency is only clocked against inputs that actually stream a frame).
struct DriveInput {
	bytes: &'static [u8],
	repaints: bool,
}

/// The round-robin interaction cycle each driven session replays: scroll, hover,
/// then Tab + Enter to navigate between the two routes. Scroll/hover exercise the
/// input + hit-test path but repaint nothing on content that fits the screen;
/// the Tab+Enter navigation swaps the page, forcing a fresh layout/paint and a
/// streamed frame diff (the dominant per-interaction cost). Only the navigating
/// Enter is flagged `repaints`, so the latency samples reflect real frames.
const DRIVE_INPUTS: &[DriveInput] = &[
	DriveInput {
		bytes: b"\x1b[<65;10;5M",
		repaints: false,
	}, // wheel down
	DriveInput {
		bytes: b"\x1b[<35;20;3M",
		repaints: false,
	}, // hover move
	DriveInput {
		bytes: b"\t",
		repaints: false,
	}, // focus the link
	DriveInput {
		bytes: b"\r",
		repaints: true,
	}, // Enter: navigate
];

/// Report progress each second. In connect-storm mode, exit once every session
/// has painted (or a 30s ceiling). In hold mode, exit after `hold` elapses,
/// printing the steady-state latency summary.
fn report_and_exit(
	config: Res<StressConfig>,
	stats: Res<StressStats>,
	mut exit: MessageWriter<AppExit>,
	mut start: Local<Option<Instant>>,
	mut seated_at: Local<Option<Instant>>,
	mut last_log: Local<Option<Instant>>,
) {
	let start = start.get_or_insert_with(Instant::now);
	let elapsed = start.elapsed();
	// mark when every session first became resident, so the hold window is
	// measured from steady state, not from app start (connecting takes time).
	if seated_at.is_none() && stats.first_frame >= config.count {
		*seated_at = Some(Instant::now());
	}
	// once-a-second progress line.
	let log_now = last_log
		.map(|last| last.elapsed().as_secs() >= 1)
		.unwrap_or(true);
	if log_now {
		*last_log = Some(Instant::now());
		info!(
			"connected {}/{}, painted {}, {} bytes, {} inputs, {:.1}s",
			stats.connected,
			config.count,
			stats.first_frame,
			stats.bytes,
			stats.inputs_sent,
			elapsed.as_secs_f32(),
		);
	}

	let done = if config.hold {
		// hold for the configured window measured from steady state (all seated),
		// or a generous ceiling so a server that can't seat N still reports.
		let held_long_enough = seated_at
			.map(|at| at.elapsed() >= config.hold_for)
			.unwrap_or(false);
		held_long_enough || elapsed.as_secs() >= 60
	} else {
		stats.first_frame >= config.count || elapsed.as_secs() >= 30
	};
	if done {
		report_summary(&config, &stats, elapsed);
		exit.write(AppExit::Success);
	}
}

/// The final result line(s): always the connect/paint tally; in hold mode also
/// the input count and input-to-frame latency percentiles.
fn report_summary(
	config: &StressConfig,
	stats: &StressStats,
	elapsed: Duration,
) {
	cross_log!(
		"stress done: {}/{} connected, {}/{} painted, {} bytes in {:.1}s",
		stats.connected,
		config.count,
		stats.first_frame,
		config.count,
		stats.bytes,
		elapsed.as_secs_f32(),
	);
	if !config.hold {
		return;
	}
	let mut samples = stats.latencies_ms.clone();
	samples.sort_by(|a, b| a.total_cmp(b));
	let pct = |p: f32| -> f32 {
		if samples.is_empty() {
			return 0.0;
		}
		let idx = ((samples.len() as f32 - 1.0) * p).round() as usize;
		samples[idx]
	};
	let input_rate = stats.inputs_sent as f32 / elapsed.as_secs_f32();
	cross_log!(
		"steady-state: {} sessions held, {} inputs ({:.1}/s), {} frames measured, latency p50={:.0}ms p99={:.0}ms",
		config.count,
		stats.inputs_sent,
		input_rate,
		samples.len(),
		pct(0.50),
		pct(0.99),
	);
}
