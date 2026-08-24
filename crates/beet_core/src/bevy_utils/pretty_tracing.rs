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
	/// Default level used when neither `RUST_LOG` nor the AWS lambda log level
	/// is present in the environment.
	pub level: tracing::Level,
	/// Extra comma-separated `EnvFilter` directives appended to the defaults,
	/// eg `"ureq=off,ureq_proto=off"`.
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
			filter,
			interactive,
		} = PrettyTracing::default();
		Self {
			level: default_level,
			filter,
			interactive,
		}
	}
}

impl Plugin for LogPlugin {
	fn build(&self, _app: &mut App) {
		PrettyTracing {
			default_level: self.level,
			filter: self.filter.clone(),
			interactive: self.interactive,
		}
		.init();
	}
}

/// Opinionated high level tracing initialization
#[derive(Clone)]
pub struct PrettyTracing {
	/// Level used when no level is found in the environment.
	pub default_level: tracing::Level,
	/// Extra comma-separated `EnvFilter` directives appended to the defaults.
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
			filter: String::new(),
			interactive: Self::is_interactive(),
		}
	}
}

/// Non-beet log targets pinned to `warn` by [`PrettyTracing::init`]: the bevy
/// render / window / GPU / asset stack plus a few other chatty dependencies.
/// These are never the beet app's own output, so their `debug`/`info` spam (eg
/// `offset_allocator` freelist churn, `bevy_shader` setup) is suppressed while
/// beet's crates and the running app keep the resolved level. Add origins here
/// as new noise appears.
const QUIET_TARGETS: &[&str] = &[
	// gpu / allocator / shader churn
	"offset_allocator",
	"gpu_allocator",
	"naga",
	// bevy render + windowing stack
	"bevy_app",
	"bevy_render",
	"bevy_shader",
	"bevy_pbr",
	"bevy_core_pipeline",
	"bevy_sprite",
	"bevy_sprite_render",
	"bevy_text",
	"bevy_ui",
	"bevy_ui_render",
	"bevy_gltf",
	"bevy_scene",
	"bevy_image",
	"bevy_mesh",
	"bevy_camera",
	"bevy_light",
	"bevy_material",
	"bevy_anti_alias",
	"bevy_post_process",
	"bevy_gizmos",
	"bevy_gizmos_render",
	"bevy_picking",
	"bevy_animation",
	"bevy_audio",
	"bevy_winit",
	"bevy_window",
	"bevy_a11y",
	"bevy_gilrs",
	"bevy_asset",
	"bevy_diagnostic",
	"bevy_dev_tools",
	// windowing / input / text deps
	"winit",
	"gilrs",
	"gilrs_core",
	"cosmic_text",
	"fontdb",
	// networking / cloud / wasm tooling
	"walrus",
	"aws",
	"hyper_util",
	"ureq",
];

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
		// upstream system rather than just the Worker entry. native keeps the
		// pretty stdout format.
		#[cfg(target_arch = "wasm32")]
		let dispatch = self.dispatch(console_writer::MakeConsoleWriter);
		#[cfg(not(target_arch = "wasm32"))]
		let dispatch = self.dispatch(std::io::stdout);
		// also installs the `log` -> `tracing` bridge; a no-op if a subscriber
		// has already been installed
		dispatch.try_init().ok();
	}

	/// `true` when stdout is an interactive terminal, ie a developer watching a
	/// tty rather than a log file, a pipe or a deployed service.
	///
	/// Always `false` on wasm, which has no tty: the JS console host stamps
	/// entries at *ingest*, so beet emits its own *emit* timestamp there too.
	pub fn is_interactive() -> bool {
		cfg_if! {
			if #[cfg(target_arch = "wasm32")] {
				false
			} else {
				std::io::IsTerminal::is_terminal(&std::io::stdout())
			}
		}
	}

	/// Builds the subscriber, writing formatted events to `writer`.
	fn dispatch<W>(&self, writer: W) -> tracing::Dispatch
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
			.with_env_filter(self.env_filter());
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

	/// The resolved [`EnvFilter`]: the environment or [`Self::default_level`],
	/// with [`QUIET_TARGETS`] pinned to `warn` and [`Self::filter`] folded in
	/// last so caller directives win.
	fn env_filter(&self) -> EnvFilter {
		// The env may carry a plain level (`info`), a directive list
		// (`info,beet_net=warn`), or nothing. A plain level (or the built-in
		// default when the env is absent) becomes the catch-all directive; a
		// directive list already carries its own catch-all through
		// `EnvFilter::from_default_env`, so re-adding the default here would
		// override the user's level (a bare level directive added later wins).
		let env_level = env_ext::var("AWS_LAMBDA_LOG_LEVEL")
			.or_else(|_| env_ext::var("RUST_LOG"))
			.ok();
		let log_level = match &env_level {
			Some(val) => LevelFilter::from_str(val).ok(),
			None => Some(self.default_level.into()),
		};

		// every non-beet origin in `QUIET_TARGETS` is pinned to `warn` (`wgpu`
		// lower still); the resolved level applies to everything else, ie the
		// beet crates and the running app. caller-supplied directives win, so
		// they are folded in last.
		let mut base = tracing_subscriber::EnvFilter::from_default_env()
			.add_directive("wgpu=error".parse().unwrap());
		if let Some(log_level) = log_level {
			base = base.add_directive(log_level.into());
		}
		let base = QUIET_TARGETS.iter().fold(base, |filter, target| {
			filter.add_directive(format!("{target}=warn").parse().unwrap())
		});
		self.filter
			.split(',')
			.map(str::trim)
			.filter(|directive| !directive.is_empty())
			.fold(base, |filter, directive| {
				filter.add_directive(directive.parse().unwrap())
			})
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
		let Ok(now) = time_ext::try_now() else {
			return Err(core::fmt::Error);
		};
		writer.write_str(&time_ext::format_iso8601(now))
	}
}

/// A [`MakeWriter`](tracing_subscriber::fmt::MakeWriter) that routes formatted
/// tracing events to the JS console, the wasm logging backend [`PrettyTracing`]
/// installs. Each event becomes one `console.log`/`console.error` call, so the
/// `tracing` macros work in the browser and in a Cloudflare Worker (`wrangler
/// tail`) the same as they do on native stdout.
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
	fn today() -> String {
		time_ext::format_iso8601(time_ext::now())[..10].into()
	}

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

	#[crate::test]
	fn interactive_omits_timestamps() {
		render(true)
			.xpect_contains("hello")
			.xnot()
			.xpect_contains(today());
	}
}
