//! The process-config type every beet binary boots from.

use crate::prelude::*;
use core::fmt::Display;
use core::net::IpAddr;
use core::str::FromStr;

/// Every pre-scene knob a beet process boots from, in one type.
///
/// ## The transport rule
///
/// One rule, no per-field exceptions: **every field parses from both transports,
/// `--kebab-name` argv and `BEET_SCREAMING_NAME` env, and argv wins.** So
/// `--store` / `BEET_STORE`, `--port` / `BEET_HTTP_PORT`, `--stage` /
/// `BEET_STAGE`, and so on. Both channels exist because both are natural
/// affordances: a human types argv, a platform writes env into a unit file or a
/// task definition.
///
/// ## Constructing it
///
/// - [`from_env`](Self::from_env) once per process, before the registry or the
///   scene exist. World-free consumers (a `Default` impl, `Stack::new`, the
///   binary's pre-world entry resolution) call it directly, usually through
///   [`from_env_or_warn`](Self::from_env_or_warn).
/// - [`from_params`](Self::from_params) over a routed request's params, for the
///   `serve` / `check` / `export-static` / `export-pdf` commands.
/// - [`parse_params`](Self::parse_params) for an overlay that must *not* consult
///   env, ie a server's boot request applied over its markup-declared fields.
/// - As a resource for world consumers, which `ServerPlugin` inserts from
///   [`from_env_or_warn`](Self::from_env_or_warn). Deliberately inserted rather
///   than `init_resource`d: [`Default`] is the *empty* config (so a parent can
///   build a child's config field by field with `..default()`), and a world with
///   no plugin reading no ambient config is the right fallback.
///
/// ## Rendering it
///
/// [`to_argv`](Self::to_argv), [`to_env`](Self::to_env) and
/// [`to_cmd_json`](Self::to_cmd_json) are the deploy side of the same names, so
/// the deploy and the runtime cannot disagree about what a knob is called. Only
/// `Some` fields render, so a default config renders to nothing.
///
/// Secrets are deliberately absent: the type has no secret field, so no renderer
/// can put one on an argv line, a `CMD` array or a systemd `ExecStart`. Secrets
/// stay env on their existing channels.
#[derive(Debug, Default, Clone, PartialEq, Eq, Resource)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct BootstrapConfig {
	/// The entry document: a path for a dir-rooted store, a name within a
	/// self-rooted one. `--main` / `BEET_MAIN`.
	pub main: Option<SmolStr>,
	/// The store the entry loads through. `--store` / `BEET_STORE`.
	pub store: Option<StoreUri>,
	/// Watch the entry's sources and live-reload. `--watch` / `BEET_WATCH`.
	pub watch: bool,
	/// Cargo features this binary is asserted to have been built with.
	/// `--features` / `BEET_FEATURES`.
	pub features: Vec<SmolStr>,
	/// Which declared servers boot. `--server` / `BEET_SERVER`.
	pub server: Option<ServerFilter>,
	/// The opening route a freshly-opened tui/ssh surface navigates to.
	/// `--path` / `BEET_PATH`.
	pub path: Option<SmolStr>,
	/// The address servers bind to. `--host` / `BEET_HOST`.
	pub host: Option<IpAddr>,
	/// The http listener port. `--port` / `BEET_HTTP_PORT`.
	pub http_port: Option<u16>,
	/// The ssh listener port. `--ssh-port` / `BEET_SSH_PORT`.
	pub ssh_port: Option<u16>,
	/// The infrastructure stage this process runs in. `--stage` / `BEET_STAGE`.
	pub stage: Option<SmolStr>,
	/// Whether services resolve locally or against the cloud.
	/// `--service-access` / `BEET_SERVICE_ACCESS`.
	pub service_access: Option<ServiceAccess>,
	/// The deploy-provided analytics table name. `--analytics-table` /
	/// `BEET_ANALYTICS_TABLE`.
	pub analytics_table: Option<SmolStr>,
	/// This deployment's unique id. `--deploy-id` / `BEET_DEPLOY_ID`.
	pub deploy_id: Option<SmolStr>,
	/// This deployment's timestamp. `--deploy-timestamp` /
	/// `BEET_DEPLOY_TIMESTAMP`.
	pub deploy_timestamp: Option<SmolStr>,
	/// The device a scene-push command targets. `--url` / `BEET_REMOTE_URL`.
	pub remote_url: Option<SmolStr>,
	/// Force TLS on or off, overriding the managed-platform detection.
	/// `--tls` / `BEET_TLS`.
	pub tls: Option<bool>,
	/// Where the cached dev certificate lives. `--tls-dir` / `BEET_TLS_DIR`.
	pub tls_dir: Option<SmolStr>,
	/// Run terminal surfaces buffered, never touching the real tty.
	/// `--headless` / `BEET_HEADLESS`.
	pub headless: bool,
	/// Capture the first window to this PNG and exit. `--screenshot` /
	/// `BEET_SCREENSHOT`.
	pub screenshot: Option<SmolStr>,
	/// The frame the screenshot harness captures on. `--screenshot-frame` /
	/// `BEET_SCREENSHOT_FRAME`.
	pub screenshot_frame: Option<u32>,
}

/// One knob's two transport names: the `--kebab-name` argv key and the
/// `BEET_SCREAMING_NAME` env key. Declared once per field so the parse and both
/// renderers cannot drift, which is exactly the class of bug this type retires
/// (the deploy wrote `BEET_PORT`, the runtime read `BEET_HTTP_PORT`).
#[derive(Debug, Copy, Clone)]
struct Knob {
	arg: &'static str,
	env: &'static str,
}

impl BootstrapConfig {
	const MAIN: Knob = Knob {
		arg: "main",
		env: "BEET_MAIN",
	};
	const STORE: Knob = Knob {
		arg: "store",
		env: "BEET_STORE",
	};
	const WATCH: Knob = Knob {
		arg: "watch",
		env: "BEET_WATCH",
	};
	const FEATURES: Knob = Knob {
		arg: "features",
		env: "BEET_FEATURES",
	};
	const SERVER: Knob = Knob {
		arg: "server",
		env: "BEET_SERVER",
	};
	const PATH: Knob = Knob {
		arg: "path",
		env: "BEET_PATH",
	};
	const HOST: Knob = Knob {
		arg: "host",
		env: "BEET_HOST",
	};
	const HTTP_PORT: Knob = Knob {
		arg: "port",
		env: "BEET_HTTP_PORT",
	};
	const SSH_PORT: Knob = Knob {
		arg: "ssh-port",
		env: "BEET_SSH_PORT",
	};
	const STAGE: Knob = Knob {
		arg: "stage",
		env: "BEET_STAGE",
	};
	const SERVICE_ACCESS: Knob = Knob {
		arg: "service-access",
		env: "BEET_SERVICE_ACCESS",
	};
	const ANALYTICS_TABLE: Knob = Knob {
		arg: "analytics-table",
		env: "BEET_ANALYTICS_TABLE",
	};
	const DEPLOY_ID: Knob = Knob {
		arg: "deploy-id",
		env: "BEET_DEPLOY_ID",
	};
	const DEPLOY_TIMESTAMP: Knob = Knob {
		arg: "deploy-timestamp",
		env: "BEET_DEPLOY_TIMESTAMP",
	};
	const REMOTE_URL: Knob = Knob {
		arg: "url",
		env: "BEET_REMOTE_URL",
	};
	const TLS: Knob = Knob {
		arg: "tls",
		env: "BEET_TLS",
	};
	const TLS_DIR: Knob = Knob {
		arg: "tls-dir",
		env: "BEET_TLS_DIR",
	};
	const HEADLESS: Knob = Knob {
		arg: "headless",
		env: "BEET_HEADLESS",
	};
	const SCREENSHOT: Knob = Knob {
		arg: "screenshot",
		env: "BEET_SCREENSHOT",
	};
	const SCREENSHOT_FRAME: Knob = Knob {
		arg: "screenshot-frame",
		env: "BEET_SCREENSHOT_FRAME",
	};

	/// The process config: this process's argv, with the `BEET_*` environment as
	/// the fallback for every field argv did not set.
	///
	/// Pure and cheap, callable before a world exists. In a browser
	/// [`env_ext::args`] yields the location query, so `?server=http` still
	/// selects; in a Cloudflare Worker both sources are empty and every field
	/// resolves to `None`, which is correct (the Worker resolves its store from
	/// bindings).
	pub fn from_env() -> Result<Self> {
		Self::from_params(&CliArgs::parse_env().params)
	}

	/// [`from_env`](Self::from_env) for a context that cannot fail: a `Default`
	/// impl, which has no error channel to raise on. A malformed field warns and
	/// resolves to `None` (so that field falls back to its static default);
	/// everything else still parses.
	pub fn from_env_or_warn() -> Self {
		Self::parse_lenient(&CliArgs::parse_env().params, &|key| {
			env_ext::var(key).ok().map(SmolStr::from)
		})
	}

	/// The config a routed request carries, with the `BEET_*` environment as the
	/// fallback for every field the params did not set. What the `serve` /
	/// `check` / `export-static` / `export-pdf` commands build from their
	/// request.
	pub fn from_params(params: &MultiMap<SmolStr, SmolStr>) -> Result<Self> {
		Self::parse(params, &|key| env_ext::var(key).ok().map(SmolStr::from))
	}

	/// The config these params alone carry, with no environment fallback. The
	/// overlay form: a server applies its boot request over its markup-declared
	/// fields, and env must not out-rank markup there.
	pub fn parse_params(params: &MultiMap<SmolStr, SmolStr>) -> Result<Self> {
		Self::parse(params, &|_| None)
	}

	/// [`parse_params`](Self::parse_params), removing every knob it consumed from
	/// `params`.
	///
	/// For a caller that forwards the *rest* of an argv on (the wasm runner hands
	/// the module its test-runner flags): the config's own knobs leave through
	/// [`to_argv`](Self::to_argv) instead, so a knob can never be forwarded twice
	/// and the two copies disagree.
	pub fn take_params(
		params: &mut MultiMap<SmolStr, SmolStr>,
	) -> Result<Self> {
		let config = Self::parse_params(params)?;
		for knob in Self::KNOBS {
			params.remove(knob.arg);
		}
		Ok(config)
	}

	/// Every knob, in field order. The one enumeration of the table, so a new
	/// field is either listed here or is not a knob at all.
	const KNOBS: [Knob; 20] = [
		Self::MAIN,
		Self::STORE,
		Self::WATCH,
		Self::FEATURES,
		Self::SERVER,
		Self::PATH,
		Self::HOST,
		Self::HTTP_PORT,
		Self::SSH_PORT,
		Self::STAGE,
		Self::SERVICE_ACCESS,
		Self::ANALYTICS_TABLE,
		Self::DEPLOY_ID,
		Self::DEPLOY_TIMESTAMP,
		Self::REMOTE_URL,
		Self::TLS,
		Self::TLS_DIR,
		Self::HEADLESS,
		Self::SCREENSHOT,
		Self::SCREENSHOT_FRAME,
	];

	/// The one parse. `env` resolves a `BEET_*` name, and is consulted only for a
	/// field the params did not set.
	fn parse(
		params: &MultiMap<SmolStr, SmolStr>,
		env: &dyn Fn(&str) -> Option<SmolStr>,
	) -> Result<Self> {
		Self::read(ConfigReader {
			params,
			env,
			lenient: false,
		})
	}

	/// [`parse`](Self::parse) with each malformed field warned about and dropped
	/// instead of failing the whole parse.
	fn parse_lenient(
		params: &MultiMap<SmolStr, SmolStr>,
		env: &dyn Fn(&str) -> Option<SmolStr>,
	) -> Self {
		Self::read(ConfigReader {
			params,
			env,
			lenient: true,
		})
		// a lenient reader never errors
		.unwrap_or_default()
	}

	/// Read every field through `reader`.
	fn read(reader: ConfigReader) -> Result<Self> {
		Self {
			main: reader.value(Self::MAIN),
			store: reader.parsed(Self::STORE)?,
			watch: reader.flag(Self::WATCH),
			features: reader.list(Self::FEATURES),
			server: reader.filter(Self::SERVER),
			path: reader.value(Self::PATH),
			host: reader.parsed(Self::HOST)?,
			http_port: reader.parsed(Self::HTTP_PORT)?,
			ssh_port: reader.parsed(Self::SSH_PORT)?,
			stage: reader.value(Self::STAGE),
			service_access: reader.parsed(Self::SERVICE_ACCESS)?,
			analytics_table: reader.value(Self::ANALYTICS_TABLE),
			deploy_id: reader.value(Self::DEPLOY_ID),
			deploy_timestamp: reader.value(Self::DEPLOY_TIMESTAMP),
			remote_url: reader.value(Self::REMOTE_URL),
			tls: reader.bool_value(Self::TLS)?,
			tls_dir: reader.value(Self::TLS_DIR),
			headless: reader.flag(Self::HEADLESS),
			screenshot: reader.value(Self::SCREENSHOT),
			screenshot_frame: reader.parsed(Self::SCREENSHOT_FRAME)?,
		}
		.xok()
	}

	/// Every set field as `--key=value` argv tokens, shell-safe by construction.
	///
	/// Each token is validated against a conservative charset (see
	/// [`validate_token`](Self::validate_token)), so a systemd `ExecStart` or a
	/// `sh -c` exec line can space-join them: a violation is a loud error here
	/// rather than a silently corrupted unit file.
	pub fn to_argv(&self) -> Result<Vec<SmolStr>> {
		let mut argv = Vec::new();
		let mut push = |knob: Knob, value: Option<String>| {
			if let Some(value) = value {
				argv.push(SmolStr::from(format!("--{}={value}", knob.arg)));
			}
		};
		push(Self::MAIN, self.main.as_ref().map(ToString::to_string));
		push(Self::STORE, self.store.as_ref().map(ToString::to_string));
		push(Self::SERVER, self.server.as_ref().map(ToString::to_string));
		push(Self::PATH, self.path.as_ref().map(ToString::to_string));
		push(Self::HOST, self.host.as_ref().map(ToString::to_string));
		push(Self::HTTP_PORT, self.http_port.map(|port| port.to_string()));
		push(Self::SSH_PORT, self.ssh_port.map(|port| port.to_string()));
		push(Self::STAGE, self.stage.as_ref().map(ToString::to_string));
		push(
			Self::SERVICE_ACCESS,
			self.service_access.as_ref().map(ToString::to_string),
		);
		push(
			Self::ANALYTICS_TABLE,
			self.analytics_table.as_ref().map(ToString::to_string),
		);
		push(
			Self::DEPLOY_ID,
			self.deploy_id.as_ref().map(ToString::to_string),
		);
		push(
			Self::DEPLOY_TIMESTAMP,
			self.deploy_timestamp.as_ref().map(ToString::to_string),
		);
		push(
			Self::REMOTE_URL,
			self.remote_url.as_ref().map(ToString::to_string),
		);
		push(Self::TLS, self.tls.map(|tls| tls.to_string()));
		push(Self::TLS_DIR, self.tls_dir.as_ref().map(ToString::to_string));
		push(
			Self::SCREENSHOT,
			self.screenshot.as_ref().map(ToString::to_string),
		);
		push(
			Self::SCREENSHOT_FRAME,
			self.screenshot_frame.map(|frame| frame.to_string()),
		);
		push(
			Self::FEATURES,
			(!self.features.is_empty()).then(|| self.features.join(",")),
		);
		// bare flags: `--watch` parses back as a flag, so no value is rendered.
		if self.watch {
			argv.push(SmolStr::from(format!("--{}", Self::WATCH.arg)));
		}
		if self.headless {
			argv.push(SmolStr::from(format!("--{}", Self::HEADLESS.arg)));
		}
		for token in &argv {
			Self::validate_token(token)?;
		}
		argv.xok()
	}

	/// Every set field as `BEET_*` environment pairs, the exact names the parse
	/// reads back. Consumed by a task definition, a systemd unit's
	/// `Environment=` lines, a lambda function env and a Worker's env object.
	pub fn to_env(&self) -> Vec<(SmolStr, SmolStr)> {
		let mut pairs: Vec<(SmolStr, SmolStr)> = Vec::new();
		let mut push = |knob: Knob, value: Option<String>| {
			if let Some(value) = value {
				pairs.push((knob.env.into(), value.into()));
			}
		};
		push(Self::MAIN, self.main.as_ref().map(ToString::to_string));
		push(Self::STORE, self.store.as_ref().map(ToString::to_string));
		push(Self::SERVER, self.server.as_ref().map(ToString::to_string));
		push(Self::PATH, self.path.as_ref().map(ToString::to_string));
		push(Self::HOST, self.host.as_ref().map(ToString::to_string));
		push(Self::HTTP_PORT, self.http_port.map(|port| port.to_string()));
		push(Self::SSH_PORT, self.ssh_port.map(|port| port.to_string()));
		push(Self::STAGE, self.stage.as_ref().map(ToString::to_string));
		push(
			Self::SERVICE_ACCESS,
			self.service_access.as_ref().map(ToString::to_string),
		);
		push(
			Self::ANALYTICS_TABLE,
			self.analytics_table.as_ref().map(ToString::to_string),
		);
		push(
			Self::DEPLOY_ID,
			self.deploy_id.as_ref().map(ToString::to_string),
		);
		push(
			Self::DEPLOY_TIMESTAMP,
			self.deploy_timestamp.as_ref().map(ToString::to_string),
		);
		push(
			Self::REMOTE_URL,
			self.remote_url.as_ref().map(ToString::to_string),
		);
		push(Self::TLS, self.tls.map(|tls| tls.to_string()));
		push(Self::TLS_DIR, self.tls_dir.as_ref().map(ToString::to_string));
		push(
			Self::SCREENSHOT,
			self.screenshot.as_ref().map(ToString::to_string),
		);
		push(
			Self::SCREENSHOT_FRAME,
			self.screenshot_frame.map(|frame| frame.to_string()),
		);
		push(
			Self::FEATURES,
			(!self.features.is_empty()).then(|| self.features.join(",")),
		);
		// presence is the signal for a flag, so any value parses back as `true`.
		push(Self::WATCH, self.watch.then(|| "1".to_string()));
		push(Self::HEADLESS, self.headless.then(|| "1".to_string()));
		pairs
	}

	/// `[binary, ..to_argv()]` as a JSON array, the container `CMD` form.
	///
	/// Correct by construction: every element is validated by
	/// [`to_argv`](Self::to_argv) (and `binary` by the same charset), so no
	/// element can carry a quote, backslash or control character needing an
	/// escape.
	pub fn to_cmd_json(&self, binary: &str) -> Result<String> {
		Self::validate_token(binary)?;
		let elements = core::iter::once(SmolStr::from(binary))
			.chain(self.to_argv()?)
			.map(|token| format!("\"{token}\""))
			.collect::<Vec<_>>()
			.join(", ");
		format!("[{elements}]").xok()
	}

	/// Split into the `(argv, env)` pair a platform encoder emits, each field on
	/// its documented default channel: boot selection (the store, the server
	/// selection, the opening path) is visible on argv, ambient service config
	/// (the bind address, the ports, the stage, the deploy identity) rides env.
	///
	/// The dev-harness fields (`main`, `watch`, `features`, `remote_url`,
	/// `tls_dir`, `headless`, `screenshot*`) belong to neither deploy channel and
	/// are dropped: a deploy has no use for them, and a deployed process probes
	/// its store for the entry rather than being told.
	pub fn split_channels(self) -> (Self, Self) {
		let argv = Self {
			store: self.store,
			server: self.server,
			path: self.path,
			..default()
		};
		let env = Self {
			host: self.host,
			http_port: self.http_port,
			ssh_port: self.ssh_port,
			stage: self.stage,
			service_access: self.service_access,
			analytics_table: self.analytics_table,
			deploy_id: self.deploy_id,
			deploy_timestamp: self.deploy_timestamp,
			tls: self.tls,
			..default()
		};
		(argv, env)
	}

	/// The bind address as IPv4 octets, the form the server components hold.
	///
	/// `bevy_reflect` has no [`IpAddr`] impl, so a markup-declarable server field
	/// stays `[u8; 4]`; the typed parse still lives here, which is what fixes the
	/// old literal `"0.0.0.0"` check (every other value, including a real
	/// address, silently became localhost). An IPv6 address warns and yields
	/// `None`: the listeners bind a v4 socket.
	pub fn host_octets(&self) -> Option<[u8; 4]> {
		match self.host? {
			IpAddr::V4(addr) => Some(addr.octets()),
			IpAddr::V6(addr) => {
				warn!("ignoring --host / BEET_HOST `{addr}`: beet servers bind IPv4");
				None
			}
		}
	}

	/// Whether a token is safe to embed verbatim in a shell exec line, a JSON
	/// `CMD` array or a systemd `ExecStart`: no whitespace, quotes, backslashes
	/// or control characters.
	///
	/// The [`StoreUri`] and [`ServerFilter`] grammars cannot produce one, so a
	/// violation means malformed input rather than a gap in the encoding.
	fn validate_token(token: &str) -> Result {
		if let Some(bad) = token.chars().find(|char| {
			char.is_whitespace()
				|| char.is_control()
				|| matches!(char, '"' | '\'' | '\\')
		}) {
			bevybail!(
				"bootstrap arg `{token}` contains {bad:?}, which cannot be \
				rendered into a shell exec line or a container CMD"
			);
		}
		Ok(())
	}
}

/// Resolves one field across both transports: the `--kebab` param when present,
/// else the `BEET_*` env var. The single place the argv-over-env rule lives, so
/// no field can opt out of it.
struct ConfigReader<'a> {
	params: &'a MultiMap<SmolStr, SmolStr>,
	env: &'a dyn Fn(&str) -> Option<SmolStr>,
	/// Whether a malformed field warns and resolves to `None` instead of failing
	/// the parse, for a `Default` impl with no error channel to raise on.
	lenient: bool,
}

impl ConfigReader<'_> {
	/// Every value for a knob: the params' values (repeated flags, in order) when
	/// the key is present with a value, else the single env value.
	fn values(&self, knob: Knob) -> Vec<SmolStr> {
		match self.params.get_vec(knob.arg) {
			Some(values) if !values.is_empty() => values.clone(),
			_ => (self.env)(knob.env).into_iter().collect(),
		}
	}

	/// The first value for a knob.
	fn value(&self, knob: Knob) -> Option<SmolStr> {
		self.values(knob).into_iter().next()
	}

	/// The first value parsed into `T`, naming both transports on failure.
	fn parsed<T>(&self, knob: Knob) -> Result<Option<T>>
	where
		T: FromStr,
		T::Err: Display,
	{
		match self.value(knob) {
			Some(value) => match value.parse::<T>() {
				Ok(parsed) => Ok(Some(parsed)),
				Err(err) => self.malformed(knob, err),
			},
			None => Ok(None),
		}
	}

	/// A malformed field: an error naming both transports, or in lenient mode a
	/// warning and `None`.
	fn malformed<T>(
		&self,
		knob: Knob,
		err: impl Display,
	) -> Result<Option<T>> {
		let message =
			format!("invalid --{} / {}: {err}", knob.arg, knob.env);
		match self.lenient {
			true => {
				warn!("{message}");
				Ok(None)
			}
			false => Err(bevyhow!("{message}")),
		}
	}

	/// A bare-presence flag: `--watch`, or any `BEET_WATCH` value.
	fn flag(&self, knob: Knob) -> bool {
		self.params.contains_key(knob.arg) || (self.env)(knob.env).is_some()
	}

	/// An explicit on/off value, where a bare `--tls` (no value) means on.
	fn bool_value(&self, knob: Knob) -> Result<Option<bool>> {
		match self.value(knob) {
			Some(value) => match value.trim() {
				"1" | "true" | "on" | "yes" => Ok(Some(true)),
				"0" | "false" | "off" | "no" => Ok(Some(false)),
				other => {
					self.malformed(knob, format!("`{other}`, expected on or off"))
				}
			},
			None if self.params.contains_key(knob.arg) => Ok(Some(true)),
			None => Ok(None),
		}
	}

	/// A comma-separated list, accumulated across repeated flags.
	fn list(&self, knob: Knob) -> Vec<SmolStr> {
		self.values(knob)
			.iter()
			.flat_map(|value| value.split(','))
			.map(str::trim)
			.filter(|entry| !entry.is_empty())
			.map(SmolStr::from)
			.collect()
	}

	/// A [`ServerFilter`], accumulated across repeated flags. `None` when the
	/// knob is absent from both transports, which is what lets a server fall back
	/// to its own `default_boot`.
	fn filter(&self, knob: Knob) -> Option<ServerFilter> {
		let values = self.values(knob);
		if values.is_empty() && !self.params.contains_key(knob.arg) {
			return None;
		}
		let mut filter = ServerFilter::default();
		for value in &values {
			filter.extend(value);
		}
		Some(filter)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// A config with every field set, so a round trip exercises each one.
	fn full() -> BootstrapConfig {
		BootstrapConfig {
			main: Some("main.bsx".into()),
			store: Some(StoreUri::parse("s3://site?region=us-west-2").unwrap()),
			watch: true,
			features: vec!["thread".into(), "sockets".into()],
			server: Some(ServerFilter::new("http,ssh")),
			path: Some("/docs".into()),
			host: Some("0.0.0.0".parse().unwrap()),
			http_port: Some(8337),
			ssh_port: Some(2222),
			stage: Some("prod".into()),
			service_access: Some(ServiceAccess::Remote),
			analytics_table: Some("beet--prod--analytics".into()),
			deploy_id: Some("019823ff-0000-7000-8000-000000000000".into()),
			deploy_timestamp: Some("2026-08-09T00:00:00Z".into()),
			remote_url: Some("http://192.168.0.4:8337".into()),
			tls: Some(true),
			tls_dir: Some("/tmp/tls".into()),
			headless: true,
			screenshot: Some("/tmp/shot.png".into()),
			screenshot_frame: Some(30),
		}
	}

	/// The renderer invariant: argv round-trips through the argv parse, and env
	/// through the env parse. This is the whole point of the type, the deploy
	/// side and the runtime side cannot disagree about a name.
	#[crate::test]
	fn renderers_round_trip() {
		let config = full();
		// argv: render, re-tokenize as a shell would, parse back
		let argv = config.to_argv().unwrap();
		let params = CliArgs::parse_tokens(
			argv.iter().map(ToString::to_string).collect(),
		)
		.params;
		BootstrapConfig::parse_params(&params)
			.unwrap()
			.xpect_eq(config.clone());
		// env: render, look the pairs back up by name
		let pairs = config.to_env();
		let lookup = |key: &str| {
			pairs
				.iter()
				.find(|(name, _)| name == key)
				.map(|(_, value)| value.clone())
		};
		BootstrapConfig::parse(&default(), &lookup)
			.unwrap()
			.xpect_eq(config);
	}

	/// A default config renders to nothing on either channel.
	#[crate::test]
	fn default_renders_empty() {
		BootstrapConfig::default().to_argv().unwrap().len().xpect_eq(0);
		BootstrapConfig::default().to_env().len().xpect_eq(0);
	}

	/// Argv beats env, per field, with no exceptions.
	#[crate::test]
	fn argv_beats_env() {
		let params = CliArgs::parse("--port=9000").params;
		let env = |key: &str| match key {
			"BEET_HTTP_PORT" => Some(SmolStr::from("8337")),
			"BEET_STAGE" => Some(SmolStr::from("prod")),
			_ => None,
		};
		let config = BootstrapConfig::parse(&params, &env).unwrap();
		config.http_port.xpect_eq(Some(9000));
		// a field argv did not set still falls back to env
		config.stage.xpect_eq(Some(SmolStr::from("prod")));
	}

	/// A token the renderers cannot encode is a loud error, not a corrupted
	/// exec line: the caveat the hand-written encoders carried becomes a type
	/// error.
	#[crate::test]
	fn rejects_unencodable_tokens() {
		let config = BootstrapConfig {
			main: Some("my entry.bsx".into()),
			..default()
		};
		config
			.to_argv()
			.unwrap_err()
			.to_string()
			.xpect_contains("cannot be rendered");
		BootstrapConfig {
			main: Some("say\"hi\"".into()),
			..default()
		}
		.to_cmd_json("/app")
		.unwrap_err()
		.to_string()
		.xpect_contains("cannot be rendered");
	}

	#[crate::test]
	fn renders_cmd_json() {
		BootstrapConfig {
			store: Some(StoreUri::parse("s3://site").unwrap()),
			server: Some(ServerFilter::new("http,ssh")),
			..default()
		}
		.to_cmd_json("/app")
		.unwrap()
		.xpect_eq(r#"["/app", "--store=s3://site", "--server=http,ssh"]"#);
	}

	/// Boot selection rides argv, ambient service config rides env, and the
	/// dev-harness fields ride neither.
	#[crate::test]
	fn splits_channels() {
		let (argv, env) = full().split_channels();
		argv.to_argv()
			.unwrap()
			.join(" ")
			.xpect_eq("--store=s3://site?region=us-west-2 --server=http,ssh --path=/docs");
		env.to_env()
			.iter()
			.map(|(key, _)| key.as_str())
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				"BEET_HOST",
				"BEET_HTTP_PORT",
				"BEET_SSH_PORT",
				"BEET_STAGE",
				"BEET_SERVICE_ACCESS",
				"BEET_ANALYTICS_TABLE",
				"BEET_DEPLOY_ID",
				"BEET_DEPLOY_TIMESTAMP",
				"BEET_TLS",
			]);
	}

	/// A malformed value errors at parse rather than silently taking a default.
	#[crate::test]
	fn malformed_errors() {
		BootstrapConfig::parse_params(&CliArgs::parse("--port=nope").params)
			.unwrap_err()
			.to_string()
			.xpect_contains("invalid --port / BEET_HTTP_PORT");
		BootstrapConfig::parse_params(&CliArgs::parse("--host=nope").params)
			.unwrap_err()
			.to_string()
			.xpect_contains("invalid --host / BEET_HOST");
	}

	/// A `Default` impl has no error channel, so one malformed field warns and
	/// resolves to `None` while every other field still parses.
	#[crate::test]
	fn lenient_drops_only_the_malformed_field() {
		let config = BootstrapConfig::parse_lenient(
			&CliArgs::parse("--port=nope --stage=prod").params,
			&|_| None,
		);
		config.http_port.xpect_none();
		config.stage.xpect_eq(Some(SmolStr::from("prod")));
	}

	/// `--server` present but empty is a selection with no constraint, distinct
	/// from an absent `--server` (which leaves each server's `default_boot` to
	/// decide).
	#[crate::test]
	fn empty_server_selection_is_present() {
		BootstrapConfig::parse_params(&CliArgs::parse("--server").params)
			.unwrap()
			.server
			.unwrap()
			.passes("http")
			.xpect_true();
		BootstrapConfig::parse_params(&default())
			.unwrap()
			.server
			.xpect_none();
	}
}
