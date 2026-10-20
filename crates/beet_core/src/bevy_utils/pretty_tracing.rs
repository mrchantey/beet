use crate::prelude::*;
use bevy::log::tracing;
use bevy::log::tracing_subscriber;
use std::str::FromStr;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::FormatTime;
use tracing_subscriber::util::SubscriberInitExt;

/// Drop-in replacement for bevy's [`bevy::log::LogPlugin`] that installs beet's
/// [`PrettyTracing`] subscriber. Prefer this over bevy's `LogPlugin` so every
/// beet app shares the same compact, cross-platform log format.
#[derive(Clone)]
pub struct LogPlugin {
	/// See [`PrettyTracing::default_level`].
	pub level: tracing::Level,
	/// See [`PrettyTracing::own_targets`].
	pub own_targets: Vec<String>,
	/// See [`PrettyTracing::filter`].
	pub filter: String,
	/// See [`PrettyTracing::interactive`].
	pub interactive: bool,
}

impl LogPlugin {
	/// Creates a new `LogPlugin` with the provided default level
	pub fn new(level: tracing::Level) -> Self { Self { level, ..default() } }
}

impl Default for LogPlugin {
	fn default() -> Self {
		let PrettyTracing {
			default_level,
			own_targets,
			filter,
			interactive,
		} = PrettyTracing::default();
		Self {
			level: default_level,
			own_targets,
			filter,
			interactive,
		}
	}
}

impl Plugin for LogPlugin {
	fn build(&self, _app: &mut App) {
		PrettyTracing {
			default_level: self.level,
			own_targets: self.own_targets.clone(),
			filter: self.filter.clone(),
			interactive: self.interactive,
		}
		.init();
	}
}

/// Opinionated high level tracing initialization.
///
/// ## Levels
///
/// Beet's own crates log at [`default_level`](Self::default_level), every
/// other crate (rustls, hyper, the AWS SDK, bevy itself) warnings and errors
/// only: a dependency's `debug` is its authors' diagnostics, not the app's.
/// Which targets are beet's own is a prefix rule,
/// [`own_targets`](Self::own_targets), so a downstream crate opts in by name.
///
/// `RUST_LOG` (or a lambda's `AWS_LAMBDA_LOG_LEVEL`), then
/// [`filter`](Self::filter), override the defaults, each a comma separated
/// directive list: a bare level sets the own targets' level
/// (`RUST_LOG=info`), a `target=level` directive any target's, a dependency's
/// included (`RUST_LOG=info,aws_smithy_runtime=debug`).
#[derive(Clone)]
pub struct PrettyTracing {
	/// The level the [`own_targets`](Self::own_targets) log at, unless a bare
	/// level in the environment or [`filter`](Self::filter) says otherwise.
	pub default_level: tracing::Level,
	/// Target prefixes logging at the resolved level, everything else at
	/// `warn`. Defaults to `beet`, the prefix of every beet crate's module
	/// path (`beet_core`, `beet_cli`, ..), and the running binary's own crate
	/// (an example, a downstream app).
	pub own_targets: Vec<String>,
	/// Extra comma separated directives, folded in last so they win: a bare
	/// level is the own targets' level, `target=level` any target's.
	pub filter: String,
	/// Whether output is going to a human at a terminal, defaulting to
	/// [`PrettyTracing::is_interactive`]. Interactive output is colored,
	/// multi-line and timestamp-free; everything else is one plain
	/// [`Iso8601Timer`]-stamped line per event. See [`PrettyTracing::init`].
	pub interactive: bool,
}

impl Default for PrettyTracing {
	fn default() -> Self {
		#[cfg(test)]
		let default_level = tracing::Level::WARN;
		#[cfg(not(test))]
		let default_level = tracing::Level::DEBUG;
		Self {
			default_level,
			own_targets: Self::default_own_targets(),
			filter: String::new(),
			interactive: Self::is_interactive(),
		}
	}
}

impl PrettyTracing {
	/// Opinionated tracing defaults for bevy, using the provided level
	/// if none in environment variables.
	/// if already initialized, this will do nothing
	///
	/// ## AWS Lambda
	/// This also considers the AWS Lambda tracing environment variables,
	/// as defined in [`lambda_http::tracing::init_default_subscriber_with_writer`]
	///
	/// ## Format
	/// See [`PrettyTracing::interactive`]: a terminal gets the colored
	/// multi-line format, a log file or log shipper gets one plain
	/// timestamped line per event.
	pub fn init(&self) {
		// wasm has no useful stdout (a Cloudflare Worker discards it), so route
		// formatted events to the JS console instead: `error!`/`info!` then surface
		// in browser devtools and `wrangler tail`, wiring diagnostics for every
		// upstream system rather than just the Worker entry.
		#[cfg(target_arch = "wasm32")]
		let dispatch = self.dispatch(console_writer::MakeConsoleWriter);
		// natively, STDERR. Logs are the diagnostics beside a program, not its
		// answer: stdout carries the answer alone, which is what makes
		// `cross_log!` mean something and a one-shot's output pipeable
		// (`query --format=jsonl > rows.jsonl` gets rows, not rows and a log).
		// It is also what every unix tool does, so `2>/dev/null` behaves the way
		// an operator expects.
		#[cfg(not(target_arch = "wasm32"))]
		let dispatch = self.dispatch(std::io::stderr);
		// also installs the `log` -> `tracing` bridge; a no-op if a subscriber
		// has already been installed
		dispatch.try_init().ok();
	}

	/// `true` when the LOG stream is an interactive terminal, ie a developer
	/// watching a tty rather than a log file, a pipe or a deployed service.
	///
	/// Stderr, because that is where the logs go: a one-shot whose stdout is
	/// piped still has a human reading its diagnostics, and they should get the
	/// colored format while the pipe gets clean output.
	///
	/// Always `false` on wasm, which has no tty: the JS console host stamps
	/// entries at *ingest*, so beet emits its own *emit* timestamp there too.
	pub fn is_interactive() -> bool {
		cfg_if! {
			if #[cfg(target_arch = "wasm32")] {
				false
			} else {
				std::io::IsTerminal::is_terminal(&std::io::stderr())
			}
		}
	}

	/// Builds the subscriber, writing formatted events to `writer` through
	/// the environment's [`env_filter`](Self::env_filter).
	fn dispatch<W>(&self, writer: W) -> tracing::Dispatch
	where
		W: 'static + Send + Sync + for<'a> MakeWriter<'a>,
	{
		self.dispatch_filtered(writer, self.env_filter())
	}

	/// Builds the subscriber, writing the events `filter` passes to `writer`.
	fn dispatch_filtered<W>(
		&self,
		writer: W,
		filter: EnvFilter,
	) -> tracing::Dispatch
	where
		W: 'static + Send + Sync + for<'a> MakeWriter<'a>,
	{
		let builder = tracing_subscriber::fmt()
			.compact()
			.with_level(true)
			.with_target(false)
			.with_thread_ids(false)
			.with_thread_names(false)
			.with_file(true)
			.with_line_number(true)
			.with_writer(writer)
			// a stream that will not take an event (`2>&1 | head`) drops it,
			// best effort like `cross_log!`: the subscriber's own report of
			// the failure is an `eprintln!`, which panics on that same stream
			.log_internal_errors(false)
			.with_env_filter(filter);
		match self.interactive {
			// a human at a terminal: colored and roomy, and the wall clock is a
			// glance away so the timestamp is just noise.
			true => builder.pretty().without_time().finish().into(),
			// a log file, a pipe or a log shipper: one grep-able line per event,
			// stamped so it can be correlated with other systems, and free of ansi
			// escapes that a log viewer would render as garbage.
			false => builder
				.with_ansi(false)
				.with_timer(Iso8601Timer)
				.finish()
				.into(),
		}
	}

	/// `beet`, and the running binary's crate when it is named otherwise.
	fn default_own_targets() -> Vec<String> {
		let binary = env_ext::program()
			.and_then(|program| Self::crate_of(&program))
			.filter(|crate_name| !crate_name.starts_with("beet"));
		core::iter::once("beet".to_string()).chain(binary).collect()
	}

	/// The crate an executable at `program` was built from: its file name
	/// less a test binary's `-<hash>` suffix, dashes as underscores the way
	/// rustc names a crate.
	fn crate_of(program: &str) -> Option<String> {
		let stem = std::path::Path::new(program).file_stem()?.to_string_lossy();
		let name = match stem.rsplit_once('-') {
			Some((name, hash))
				if !hash.is_empty()
					&& hash.chars().all(|char| char.is_ascii_hexdigit()) =>
			{
				name
			}
			_ => &stem,
		};
		name.replace('-', "_").xsome()
	}

	/// The resolved [`EnvFilter`], see [`PrettyTracing`]'s levels.
	fn env_filter(&self) -> EnvFilter {
		let env = env_ext::var("AWS_LAMBDA_LOG_LEVEL")
			.or_else(|_| env_ext::var("RUST_LOG"))
			.ok();
		self.filter_with(env.as_deref())
	}

	/// The filter for the environment's directive list `env`, the pure half of
	/// [`env_filter`](Self::env_filter).
	fn filter_with(&self, env: Option<&str>) -> EnvFilter {
		let directives = env
			.into_iter()
			.chain(core::iter::once(self.filter.as_str()))
			.flat_map(|list| list.split(','))
			.map(str::trim)
			.filter(|directive| !directive.is_empty())
			.collect::<Vec<_>>();
		// a bare level is the own targets' level, the last one winning
		let level = directives
			.iter()
			.filter_map(|directive| LevelFilter::from_str(directive).ok())
			.last()
			.unwrap_or_else(|| self.default_level.into());
		let defaults = EnvFilter::default()
			.add_directive(LevelFilter::WARN.into())
			.add_directive("wgpu=error".parse().unwrap());
		let own = self
			.own_targets
			.iter()
			.filter_map(|target| format!("{target}={level}").parse().ok())
			.fold(defaults, |filter, directive| {
				filter.add_directive(directive)
			});
		// every target directive after the defaults, so it wins; an
		// unparseable one is dropped, as `EnvFilter` itself would
		directives
			.iter()
			.filter(|directive| LevelFilter::from_str(directive).is_err())
			.filter_map(|directive| directive.parse().ok())
			.fold(own, |filter, directive| filter.add_directive(directive))
	}
}

/// The [`FormatTime`] used by non-interactive [`PrettyTracing`] output,
/// emitting an ISO 8601 UTC timestamp with millisecond precision, eg
/// `2024-09-09T19:46:02.102Z`.
///
/// Backed by the cross-platform [`time_ext::try_now`] clock, so it also works
/// on wasm and on a bare target once a clock is installed; until then the fmt
/// layer substitutes `<unknown time>` rather than dropping the event.
#[derive(Debug, Default, Clone, Copy)]
pub struct Iso8601Timer;

impl FormatTime for Iso8601Timer {
	fn format_time(&self, writer: &mut Writer<'_>) -> core::fmt::Result {
		let Ok(now) = Timestamp::try_now() else {
			return Err(core::fmt::Error);
		};
		writer.write_str(&now.format_iso8601())
	}
}

/// A [`MakeWriter`](tracing_subscriber::fmt::MakeWriter) that routes formatted
/// tracing events to the JS console, the wasm logging backend [`PrettyTracing`]
/// installs. Each event becomes one `console.log`/`console.error` call, so the
/// `tracing` macros work in the browser and in a Cloudflare Worker (`wrangler
/// tail`) the same as they do on native stderr.
#[cfg(target_arch = "wasm32")]
mod console_writer {
	use bevy::log::tracing::Level;
	use bevy::log::tracing::Metadata;
	use bevy::log::tracing_subscriber::fmt::MakeWriter;
	use std::io;
	use wasm_bindgen::JsValue;

	/// Builds a fresh [`ConsoleWriter`] per event, tagged with the event's level.
	pub struct MakeConsoleWriter;

	impl<'a> MakeWriter<'a> for MakeConsoleWriter {
		type Writer = ConsoleWriter;
		fn make_writer(&'a self) -> Self::Writer {
			ConsoleWriter::new(Level::INFO)
		}
		fn make_writer_for(&'a self, meta: &Metadata<'_>) -> Self::Writer {
			ConsoleWriter::new(*meta.level())
		}
	}

	/// Buffers one formatted event, emitting it to the console on drop (the point
	/// at which the fmt layer has written the whole line).
	pub struct ConsoleWriter {
		level: Level,
		buf: Vec<u8>,
	}

	impl ConsoleWriter {
		fn new(level: Level) -> Self {
			Self {
				level,
				buf: Vec::new(),
			}
		}
	}

	impl io::Write for ConsoleWriter {
		fn write(&mut self, data: &[u8]) -> io::Result<usize> {
			self.buf.extend_from_slice(data);
			Ok(data.len())
		}
		fn flush(&mut self) -> io::Result<()> { Ok(()) }
	}

	impl Drop for ConsoleWriter {
		fn drop(&mut self) {
			use crate::exports::web_sys::console;
			// the fmt layer appends a trailing newline; the console adds its own.
			let value = JsValue::from_str(
				String::from_utf8_lossy(&self.buf).trim_end(),
			);
			// errors to `console.error`, everything else to `console.log` (the level
			// is also in the message text via `with_level(true)`).
			match self.level {
				Level::ERROR => console::error_1(&value),
				_ => console::log_1(&value),
			}
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::log::tracing;
	use bevy::log::tracing_subscriber::fmt::MakeWriter;
	use std::io;
	use std::sync::Arc;
	use std::sync::Mutex;

	/// A [`MakeWriter`] collecting formatted events into a shared buffer.
	#[derive(Default, Clone)]
	struct CaptureWriter(Arc<Mutex<Vec<u8>>>);

	impl CaptureWriter {
		fn into_string(self) -> String {
			String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
		}
	}

	impl io::Write for CaptureWriter {
		fn write(&mut self, data: &[u8]) -> io::Result<usize> {
			self.0.lock().unwrap().extend_from_slice(data);
			Ok(data.len())
		}
		fn flush(&mut self) -> io::Result<()> { Ok(()) }
	}

	impl<'a> MakeWriter<'a> for CaptureWriter {
		type Writer = Self;
		fn make_writer(&'a self) -> Self::Writer { self.clone() }
	}

	/// Formats a single event with the subscriber built for the given mode.
	fn render(interactive: bool) -> String {
		let writer = CaptureWriter::default();
		let dispatch = PrettyTracing {
			interactive,
			// pin this module's level so the ambient `RUST_LOG` cannot mute it
			filter: format!("{}=error", module_path!()),
			..default()
		}
		.dispatch(writer.clone());
		tracing::dispatcher::with_default(&dispatch, || {
			tracing::error!("hello");
		});
		writer.into_string()
	}

	/// Today's date, the prefix of every timestamp [`Iso8601Timer`] emits.
	fn today() -> String { Date::today().to_string() }

	#[crate::test]
	fn non_interactive_stamps_lines() {
		let output = render(false);
		// a full `YYYY-MM-DDTHH:MM:SS.mmmZ` leads the line
		let timestamp = output.split_whitespace().next().unwrap();
		timestamp.len().xpect_eq(24);
		timestamp.xpect_starts_with(today()).xpect_ends_with("Z");
		// and the line is plain single-line text a log shipper can parse
		output.trim_end().lines().count().xpect_eq(1);
		output
			.xpect_contains("hello")
			.xnot()
			.xpect_contains("\u{1b}[");
	}

	/// What `config` writes of the events `emit` logs, under the filter it
	/// builds for the environment directives `env`.
	fn captured(
		config: PrettyTracing,
		env: Option<&str>,
		emit: fn(),
	) -> String {
		let writer = CaptureWriter::default();
		let dispatch = PrettyTracing {
			interactive: false,
			..config.clone()
		}
		.dispatch_filtered(writer.clone(), config.filter_with(env));
		tracing::dispatcher::with_default(&dispatch, emit);
		writer.into_string()
	}

	/// One event per level of interest, for beet's crates, a downstream app's,
	/// and two dependencies.
	fn emit_levels() {
		tracing::trace!(target: "beet_net::server", "beet trace");
		tracing::debug!(target: "beet_infra::deploy", "beet debug");
		tracing::info!(target: "beet_cli", "cli info");
		tracing::debug!(target: "my_app::main", "app debug");
		tracing::debug!(target: "rustls::client::hs", "rustls debug");
		tracing::info!(target: "aws_smithy_runtime::client", "sdk info");
		tracing::debug!(target: "aws_smithy_runtime::client", "sdk debug");
		tracing::warn!(target: "hyper_util::client", "hyper warn");
	}

	/// The tracing of an app named `my_app` logging at `debug`.
	fn my_app() -> PrettyTracing {
		PrettyTracing {
			default_level: tracing::Level::DEBUG,
			own_targets: vec!["beet".into(), "my_app".into()],
			..default()
		}
	}

	/// Every beet crate and the app log at the configured level, a dependency
	/// warnings and errors only (rustls alone logged a dozen debug lines per
	/// handshake into every deploy).
	#[crate::test]
	fn dependencies_only_warn() {
		captured(my_app(), None, emit_levels)
			.xpect_contains("beet debug")
			.xpect_contains("cli info")
			.xpect_contains("app debug")
			.xpect_contains("hyper warn")
			.xnot()
			.xpect_contains("beet trace")
			.xnot()
			.xpect_contains("rustls debug")
			.xnot()
			.xpect_contains("sdk info");
	}

	/// `RUST_LOG` overrides: a bare level is beet's own, a target directive
	/// raises a dependency, and [`PrettyTracing::filter`] wins over both.
	#[crate::test]
	fn the_environment_overrides() {
		captured(my_app(), Some("info,aws_smithy_runtime=debug"), emit_levels)
			.xpect_contains("cli info")
			.xpect_contains("sdk debug")
			.xnot()
			.xpect_contains("beet debug")
			.xnot()
			.xpect_contains("rustls debug");
		let traced = PrettyTracing {
			filter: "trace".into(),
			..my_app()
		};
		captured(traced, Some("info"), emit_levels)
			.xpect_contains("beet trace")
			.xnot()
			.xpect_contains("rustls debug");
	}

	/// The running binary's crate is the app's own: an example, a renamed
	/// release binary, and not a test binary's hash suffix.
	#[crate::test]
	fn names_the_running_crate() {
		PrettyTracing::crate_of("target/debug/examples/http_server")
			.xpect_eq(Some("http_server".into()));
		PrettyTracing::crate_of("target/debug/deps/beet_core-a05ed5b61105e20a")
			.xpect_eq(Some("beet_core".into()));
		PrettyTracing::crate_of("/usr/local/bin/main-lightsail")
			.xpect_eq(Some("main_lightsail".into()));
	}

	#[crate::test]
	fn interactive_omits_timestamps() {
		render(true)
			.xpect_contains("hello")
			.xnot()
			.xpect_contains(today());
	}

	/// Marks the child run of [`tolerates_a_closed_stream`].
	#[cfg(unix)]
	const CLOSED_STREAM_CHILD: &str = "--pretty-tracing-closed-stream-child";

	/// Logs, from a child run of this test binary, into a stderr whose reader
	/// is gone, as `2>&1 | head` leaves it: the event is dropped and the child
	/// carries on, where the subscriber's own report of the failed write, an
	/// `eprintln!` to that same stream, panicked.
	#[cfg(unix)]
	#[crate::test]
	fn tolerates_a_closed_stream() {
		if env_ext::args().iter().any(|arg| arg == CLOSED_STREAM_CHILD) {
			let mut fds = [0; 2];
			// SAFETY: `pipe` fills the two-fd array, `close` and `dup2` take
			// the fds it returned
			unsafe {
				libc::pipe(fds.as_mut_ptr());
				libc::close(fds[0]);
				libc::dup2(fds[1], 2);
			}
			let dispatch = PrettyTracing {
				interactive: false,
				filter: format!("{}=error", module_path!()),
				..default()
			}
			.dispatch(std::io::stderr);
			tracing::dispatcher::with_default(&dispatch, || {
				tracing::error!("nobody is listening");
			});
			process_ext::exit(0);
		}
		std::process::Command::new(std::env::current_exe().unwrap())
			.args(["tolerates_a_closed_stream", "--quiet", CLOSED_STREAM_CHILD])
			.output()
			.unwrap()
			.status
			.code()
			.xpect_eq(Some(0));
	}
}
