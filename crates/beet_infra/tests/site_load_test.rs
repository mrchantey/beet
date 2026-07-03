//! Drives concurrent SSH-TUI sessions against the live deployed beet site to
//! exercise Fargate CPU target-tracking autoscaling: scale-out under load, then
//! scale-in once the load drops. `#[ignore]`d: it hits the live `beet-site--dev`
//! deployment, costs money, and runs ~25 minutes.
//!
//! ## Background (`.agents/reports/load-test.md`, reconciled to the v3 topology)
//!
//! The service autoscales on the predefined target-tracking metric
//! `ECSServiceAverageCPUUtilization`, target 50%, min 1 / max 5. The v1 load test
//! found roughly **40 driven SSH-TUI sessions per 1-vCPU task** to cross the 50%
//! threshold (about 1 CPU point per active navigating session above idle), and
//! demonstrated scale-out 1 -> 3 (and to 5 under deploy churn) then scale-in back
//! to 1. It also surfaced an idle-CPU bug (each 0.25-vCPU task pinned ~100% CPU at
//! idle, so the service never scaled in); that was fixed by pacing the schedule
//! loop to 30Hz and bumping the task to 1 vCPU, so idle CPU now sits ~12% and
//! scale-in is meaningful.
//!
//! Two topology changes since that report: ssh is now on the unified NLB at
//! **port 22** via the DNS-only `app.dev.beet.org` hostname (no `-p`, no separate
//! ssh NLB on 8339), and the cluster/service are
//! `beet-site--dev--main-fargate-cluster` / `main-fargate--service`.
//!
//! Run it (after the site is deployed) with:
//!     cargo test -p beet_infra --features deploy,aws_sdk,fargate_block \
//!         --test site_load_test -- --ignored

beet_core::test_main!();

use beet_core::prelude::*;
use beet_net::prelude::*;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

mod runtime_utils;
use runtime_utils::*;

const REGION: &str = "us-west-2";
const CLUSTER: &str = "beet-site--dev--main-fargate-cluster";
const SERVICE: &str = "main-fargate--service";
// the app hostname is DNS-only, so the load test bypasses the edge cache
const SSH_ADDR: &str = "app.dev.beet.org:22";
const HTTP_HEALTH: &str = "http://app.dev.beet.org/health";
/// Driven sessions: ~1 per CPU point above idle, so ~40 crosses the 50% target.
const SESSIONS: usize = 40;

#[beet_core::test(timeout_ms = 1_500_000)]
#[ignore = "drives load against the live dev site and exercises real Fargate autoscaling"]
async fn site_scales_out_then_in() {
	init_logger();
	info!("==== STARTING SITE LOAD TEST (scale-out then scale-in) ====");

	// pre-flight: the site is reachable, and capture the idle baseline.
	verify_live(HTTP_HEALTH, "", 10, 6).await.unwrap();
	let (base_running, base_cpu) = poll().await.unwrap();
	info!("baseline: running={base_running} cpu={base_cpu:.1}%");

	// ramp the load on a background app, then watch the service scale out.
	let load = SshLoad::start(SSH_ADDR, SESSIONS);
	let scaled_out = wait_for(
		Duration::from_secs(600),
		Duration::from_secs(30),
		|running, _| running > 1,
	)
	.await;
	scaled_out.xpect_true();

	// drop the load and watch the service step back to the floor (the slow
	// direction: cloudwatch lags ~2 min and scale-in respects a cooldown).
	load.stop();
	info!("load dropped; awaiting scale-in to 1");
	let scaled_in = wait_for(
		Duration::from_secs(720),
		Duration::from_secs(30),
		|running, _| running <= 1,
	)
	.await;
	scaled_in.xpect_true();
}

/// Poll the service `(runningCount, averageCpuPercent)` every `interval` until
/// `done(running, cpu)` holds or `timeout` elapses, logging each sample.
async fn wait_for(
	timeout: Duration,
	interval: Duration,
	done: impl Fn(i64, f64) -> bool,
) -> bool {
	let start = Instant::now();
	while start.elapsed() < timeout {
		if let Ok((running, cpu)) = poll().await {
			info!("running={running} cpu={cpu:.1}%");
			if done(running, cpu) {
				return true;
			}
		}
		time_ext::sleep(interval).await;
	}
	false
}

/// One AWS poll: the service's running count and the trailing-window average CPU.
async fn poll() -> Result<(i64, f64)> {
	let running = aws_text(&[
		"ecs",
		"describe-services",
		"--cluster",
		CLUSTER,
		"--services",
		SERVICE,
		"--region",
		REGION,
		"--query",
		"services[0].runningCount",
		"--output",
		"text",
	])
	.await?
	.trim()
	.parse()
	.unwrap_or(0);

	// cloudwatch needs absolute ISO8601 UTC; `Instant` is not wall-clock, so shell
	// `date -u` for the window bounds (the v1 monitor did the same).
	let start = aws_cmd("date", &["-u", "-d", "-5 min", "+%Y-%m-%dT%H:%M:%SZ"])
		.await?
		.trim()
		.to_string();
	let end = aws_cmd("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"])
		.await?
		.trim()
		.to_string();
	let dims_cluster = format!("Name=ClusterName,Value={CLUSTER}");
	let dims_service = format!("Name=ServiceName,Value={SERVICE}");
	let cpu = aws_text(&[
		"cloudwatch",
		"get-metric-statistics",
		"--namespace",
		"AWS/ECS",
		"--metric-name",
		"CPUUtilization",
		"--dimensions",
		&dims_cluster,
		&dims_service,
		"--start-time",
		&start,
		"--end-time",
		&end,
		"--period",
		"60",
		"--statistics",
		"Average",
		"--region",
		REGION,
		"--query",
		"sort_by(Datapoints,&Timestamp)[-1].Average",
		"--output",
		"text",
	])
	.await?
	.trim()
	.parse()
	.unwrap_or(0.0);
	Ok((running, cpu))
}

/// Run the `aws` CLI and capture stdout.
async fn aws_text(args: &[&str]) -> Result<String> {
	aws_cmd("aws", args).await
}

/// Run `cmd args` and capture stdout (a failed/non-zero exit is an `Err`).
async fn aws_cmd(cmd: &str, args: &[&str]) -> Result<String> {
	ChildProcess::new(cmd)
		.with_args(args.iter().copied())
		.run_async_stdout()
		.await
}

/// A background app holding `count` anonymous SSH-TUI sessions, driving each with
/// navigation keystrokes so the server keeps repainting. [`stop`](Self::stop)
/// closes the sessions and drains the app.
struct SshLoad {
	shutdown: Arc<AtomicBool>,
	thread: Option<std::thread::JoinHandle<()>>,
}

impl SshLoad {
	fn start(addr: &str, count: usize) -> Self {
		let shutdown = Arc::new(AtomicBool::new(false));
		let config = LoadConfig {
			addr: addr.to_string(),
			count,
			shutdown: shutdown.clone(),
		};
		info!("opening {count} ssh sessions to {addr}");
		let thread = std::thread::spawn(move || {
			App::new()
				.add_plugins((
					MinimalPlugins,
					LogPlugin::default(),
					AsyncPlugin::default(),
				))
				.insert_resource(config)
				.add_systems(Startup, spawn_sessions)
				.add_observer(on_recv)
				.add_systems(Update, drive_sessions)
				.run();
		});
		Self {
			shutdown,
			thread: Some(thread),
		}
	}

	/// Signal the app to close every session and exit, then join it.
	fn stop(mut self) {
		self.shutdown.store(true, Ordering::Relaxed);
		if let Some(thread) = self.thread.take() {
			thread.join().ok();
		}
	}
}

#[derive(Resource)]
struct LoadConfig {
	addr: String,
	count: usize,
	shutdown: Arc<AtomicBool>,
}

/// Marks a connected session, so the driver can find them all.
#[derive(Component)]
struct Session;

fn spawn_sessions(mut commands: Commands, config: Res<LoadConfig>) {
	for _ in 0..config.count {
		commands.spawn(SshSession::insert_anon(&config.addr));
	}
}

/// On connect, request an 80x24 pty + shell to start the TUI and mark the session.
fn on_recv(ev: On<SshRecv>, mut commands: Commands) {
	let session = ev.target();
	if let SshEvent::Connect = ev.event().inner() {
		commands
			.entity(session)
			.trigger_target(SshSend(SshEvent::RequestPty(RequestPty {
				terminal: "xterm-256color".into(),
				window: SshWindowSize {
					cells: UVec2::new(80, 24),
					pixels: UVec2::ZERO,
				},
				terminal_modes: Vec::new(),
			})));
		commands
			.entity(session)
			.trigger_target(SshSend(SshEvent::RequestShell));
		commands.entity(session).insert(Session);
	}
}

/// On a ~2Hz cadence drive each session one navigation keystroke (round-robin Tab
/// /Enter to swap routes, forcing a repaint). On shutdown, close all sessions and
/// exit the app.
fn drive_sessions(
	config: Res<LoadConfig>,
	sessions: Query<Entity, With<Session>>,
	mut commands: Commands,
	mut last_tick: Local<Option<Instant>>,
	mut next: Local<usize>,
	mut exit: MessageWriter<AppExit>,
) {
	if config.shutdown.load(Ordering::Relaxed) {
		for session in sessions.iter() {
			commands
				.entity(session)
				.trigger_target(SshSend(SshEvent::Close(None)));
		}
		exit.write(AppExit::Success);
		return;
	}
	let tick = last_tick
		.map(|last| last.elapsed() >= Duration::from_millis(500))
		.unwrap_or(true);
	if !tick {
		return;
	}
	*last_tick = Some(Instant::now());
	// Tab focuses the in-page link, Enter navigates (a fresh layout + paint).
	const KEYS: &[&[u8]] = &[b"\t", b"\r", b"j", b"k"];
	for session in sessions.iter() {
		let keys = KEYS[*next % KEYS.len()];
		*next += 1;
		commands
			.entity(session)
			.trigger_target(SshSend(SshEvent::bytes(keys.to_vec())));
	}
}
