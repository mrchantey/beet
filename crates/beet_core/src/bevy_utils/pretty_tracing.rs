use crate::prelude::*;
use bevy::log::tracing;
use bevy::log::tracing_subscriber;
use std::str::FromStr;
use tracing::level_filters::LevelFilter;

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
		} = PrettyTracing::default();
		Self {
			level: default_level,
			filter,
		}
	}
}

impl Plugin for LogPlugin {
	fn build(&self, _app: &mut App) {
		PrettyTracing {
			default_level: self.level,
			filter: self.filter.clone(),
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
	pub fn init(&self) {
		let log_level = env_ext::var("AWS_LAMBDA_LOG_LEVEL")
			.or_else(|_| env_ext::var("RUST_LOG"))
			.map(|val| LevelFilter::from_str(&val).ok())
			.ok()
			.flatten()
			.unwrap_or(self.default_level.into());

		// caller-supplied directives win over the defaults below, so append them last.
		let env_filter = self
			.filter
			.split(',')
			.map(str::trim)
			.filter(|directive| !directive.is_empty())
			.fold(
				tracing_subscriber::EnvFilter::from_default_env()
					.add_directive("wgpu=error".parse().unwrap())
					.add_directive("naga=warn".parse().unwrap())
					.add_directive("bevy_app=warn".parse().unwrap())
					.add_directive("walrus=warn".parse().unwrap())
					.add_directive("aws=warn".parse().unwrap())
					.add_directive("hyper-util=warn".parse().unwrap())
					.add_directive(log_level.into()),
				|filter, directive| {
					filter.add_directive(directive.parse().unwrap())
				},
			);

		let builder = tracing_subscriber::fmt()
			.compact()
			.with_level(true)
			.with_target(false)
			.with_thread_ids(false)
			.with_thread_names(false)
			.with_file(true)
			.without_time()
			.with_line_number(true)
			.with_env_filter(env_filter);

		// wasm has no useful stdout (a Cloudflare Worker discards it), so route
		// formatted events to the JS console instead: `error!`/`info!` then surface
		// in browser devtools and `wrangler tail`, wiring diagnostics for every
		// upstream system rather than just the Worker entry. native keeps the
		// pretty stdout format.
		#[cfg(target_arch = "wasm32")]
		builder
			.with_ansi(false)
			.with_writer(console_writer::MakeConsoleWriter)
			.try_init()
			.ok();
		#[cfg(not(target_arch = "wasm32"))]
		builder.with_writer(std::io::stdout).pretty().try_init().ok();
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
		fn new(level: Level) -> Self { Self { level, buf: Vec::new() } }
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
			let value =
				JsValue::from_str(String::from_utf8_lossy(&self.buf).trim_end());
			// errors to `console.error`, everything else to `console.log` (the level
			// is also in the message text via `with_level(true)`).
			match self.level {
				Level::ERROR => console::error_1(&value),
				_ => console::log_1(&value),
			}
		}
	}
}
