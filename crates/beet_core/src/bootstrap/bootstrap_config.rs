//! The process-config type every beet binary boots from.

use crate::prelude::*;
use bevy::platform::sync::LazyLock;
use core::fmt::Display;
use core::net::IpAddr;
use core::str::FromStr;

/// Every pre-scene knob a beet process boots from, in one type.
///
/// ## What it is, and is not
///
/// A `BootstrapConfig` describes **one process launch**. There are exactly two
/// things to do with one: read the launch *this* process booted from
/// ([`get`](Self::get) / [`from_env`](Self::from_env)), or describe the launch of
/// a process you are about to start ([`launch`](Self::launch), rendered through
/// [`to_argv`](Self::to_argv) / [`to_env`](Self::to_env) /
/// [`to_cmd_json`](Self::to_cmd_json) by `ChildProcess::with_bootstrap` and the
/// deploy blocks).
///
/// It is **not** a params parser. A route reading one flag off its request reads
/// that flag (a `ParamsPartial` params type, so `--help` documents it), because a
/// request is not a process launch: pulling a whole 19-knob config out of one to
/// reach a single field silently drags the `BEET_*` environment in behind it and
/// hides the flag from the route's own help. The single request-shaped
/// constructor, [`take_launch`](Self::take_launch), exists only for beet spawning
/// beet, and consumes the knobs it reads so they cannot also be forwarded.
///
/// Fields are private and set through the generated `with_*` builders, so a
/// construction site reads as the launch description it is.
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
/// ## Reading it
///
/// **The process config is a process-global singleton, not a resource.** Launch
/// arguments are a fact about the process, not state an app owns: there is one
/// argv, it never changes, and it must be readable from places that have no
/// world at all (a [`Default`] impl, `Stack::new`, a plugin's `build`, the
/// binary's pre-world entry resolution). Modelling it as a resource forced world
/// access into all of those *and* left two sources of truth, since a `Default`
/// impl could only reach the environment. So it follows `CanonicalPort`: one
/// static, read through [`get`](Self::get) from anywhere.
///
/// - [`get`](Self::get): the process config, parsed from argv + env on first
///   read and memoized. Never fails; a malformed field warns and falls back to
///   its static default, since a `Default` impl has no error channel.
/// - [`from_env`](Self::from_env): the same parse, *strict*. `BootstrapPlugin`
///   runs it so a malformed `--port=nope` fails the app loudly instead of only
///   warning.
///
/// ## Rendering it
///
/// [`to_argv`](Self::to_argv), [`to_env`](Self::to_env) and
/// [`to_cmd_json`](Self::to_cmd_json) are the deploy side of the same names, so
/// the deploy and the runtime cannot disagree about what a knob is called. Only
/// a field differing from its default renders, so a default config renders to
/// nothing and the renderers round-trip exactly.
///
/// Secrets are deliberately absent: the type has no secret field, so no renderer
/// can put one on an argv line, a `CMD` array or a systemd `ExecStart`. Secrets
/// stay env on their existing channels.
#[derive(Debug, Clone, PartialEq, Eq, Get, SetWith)]
#[set_with(unwrap_option)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct BootstrapConfig {
	/// The entry document: a path for a dir-rooted store, a name within a
	/// self-rooted one. `--main` / `BEET_MAIN`.
	main: Option<SmolStr>,
	/// The store the entry loads through. `--store` / `BEET_STORE`.
	store: Option<StoreUri>,
	/// Watch the entry's sources and live-reload. `--watch` / `BEET_WATCH`.
	watch: bool,
	/// Cargo features this binary is asserted to have been built with.
	/// `--features` / `BEET_FEATURES`.
	features: Vec<SmolStr>,
	/// Which declared servers boot. `--server` / `BEET_SERVER`.
	server: Option<ServerFilter>,
	/// The opening route a freshly-opened tui/ssh surface navigates to.
	/// `--path` / `BEET_PATH`.
	path: Option<SmolStr>,
	/// The address servers bind to, read through
	/// [`host_octets`](Self::host_octets). `--host` / `BEET_HOST`.
	#[get(skip)]
	host: Option<IpAddr>,
	/// The http listener port. `--port` / `BEET_HTTP_PORT`.
	#[get(copy)]
	http_port: Option<u16>,
	/// The ssh listener port. `--ssh-port` / `BEET_SSH_PORT`.
	#[get(copy)]
	ssh_port: Option<u16>,
	/// The infrastructure stage this process runs in, defaulting to
	/// [`DEFAULT_STAGE`](Self::DEFAULT_STAGE). `--stage` / `BEET_STAGE`.
	stage: SmolStr,
	/// Whether services resolve locally or against the cloud, defaulting to
	/// [`ServiceAccess::Local`]. `--service-access` / `BEET_SERVICE_ACCESS`.
	#[get(copy)]
	service_access: ServiceAccess,
	/// The deploy-provided analytics table name. `--analytics-table` /
	/// `BEET_ANALYTICS_TABLE`.
	analytics_table: Option<SmolStr>,
	/// This deployment's unique id. `--deploy-id` / `BEET_DEPLOY_ID`.
	deploy_id: Option<SmolStr>,
	/// This deployment's timestamp. `--deploy-timestamp` /
	/// `BEET_DEPLOY_TIMESTAMP`.
	deploy_timestamp: Option<SmolStr>,
	/// Force TLS on or off, overriding the managed-platform detection.
	/// `--tls` / `BEET_TLS`.
	#[get(copy)]
	tls: Option<bool>,
	/// Where the cached dev certificate lives. `--tls-dir` / `BEET_TLS_DIR`.
	tls_dir: Option<SmolStr>,
	/// Run terminal surfaces buffered, never touching the real tty.
	/// `--headless` / `BEET_HEADLESS`.
	headless: bool,
	/// Capture the first window to this PNG and exit. `--screenshot` /
	/// `BEET_SCREENSHOT`.
	screenshot: Option<SmolStr>,
	/// The frame the screenshot harness captures on. `--screenshot-frame` /
	/// `BEET_SCREENSHOT_FRAME`.
	#[get(copy)]
	screenshot_frame: Option<u32>,
}

/// Every field's static default, ie what an unset knob resolves to. Also the
/// baseline the renderers diff against, so a field left at its default never
/// reaches an argv line or an env pair. Spelled [`launch`](Self::launch) at a
/// construction site, which names what constructing one is for.
impl Default for BootstrapConfig {
	fn default() -> Self {
		Self {
			main: None,
			store: None,
			watch: false,
			features: default(),
			server: None,
			path: None,
			host: None,
			http_port: None,
			ssh_port: None,
			stage: Self::DEFAULT_STAGE.into(),
			service_access: default(),
			analytics_table: None,
			deploy_id: None,
			deploy_timestamp: None,
			tls: None,
			tls_dir: None,
			headless: false,
			screenshot: None,
			screenshot_frame: None,
		}
	}
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
	/// The stage an unqualified launch runs in. A deploy names its own stage on
	/// either transport, so only a local run relies on this.
	pub const DEFAULT_STAGE: &'static str = "dev";
	/// The stage that turns on production behaviour, ie dropping draft routes
	/// from a static export.
	pub const PROD_STAGE: &'static str = "prod";

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
		arg: ServerFilter::PARAM,
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

	/// A launch to describe: every knob at its static default, named explicitly
	/// from here with the `with_*` builders.
	///
	/// The one constructor, and only for a process *this* one is about to start:
	/// a beet child (`ChildProcess::with_bootstrap`) or a deployed binary (an
	/// infra block's rendered `CMD` / `ExecStart` / task env). Nothing is
	/// inherited by accident, because nothing is inherited at all. To read the
	/// launch this process itself booted from, use [`get`](Self::get).
	pub fn launch() -> Self { Self::default() }

	/// Parse this process's argv, with the `BEET_*` environment as the fallback for
	/// every field argv did not set. Strict: a malformed value errors.
	///
	/// Most callers want [`get`](Self::get), which memoizes and cannot fail. This
	/// is what [`BootstrapPlugin`] runs at [`PreStartup`] so a malformed value is
	/// still surfaced through the app's error handler.
	///
	/// In a browser [`env_ext::args`] yields the location query, so `?server=http`
	/// still selects; in a Cloudflare Worker both sources are empty and every field
	/// resolves to `None`, which is correct (the Worker resolves its store from
	/// bindings).
	pub fn from_env() -> Result<Self> {
		Self::parse(&CliArgs::parse_env().params, &Self::env_var)
	}

	/// The process config, parsed from argv + env on first read and memoized
	/// forever.
	///
	/// The single read path, callable from anywhere: a `Default` impl, a plugin's
	/// `build`, a world-free helper. Never fails, because most callers have no
	/// error channel; a malformed field warns and falls back to its static
	/// default, and [`BootstrapPlugin`] re-runs the strict
	/// [`from_env`](Self::from_env) at [`PreStartup`] to turn that into a real
	/// error.
	pub fn get() -> &'static Self { &BOOTSTRAP }

	/// The launch config for a beet child process, *taken out of* `params` (with
	/// this process's `BEET_*` environment as the fallback for every knob the
	/// params did not set).
	///
	/// The one request-shaped constructor, and only for beet spawning beet: the
	/// wasm runner is handed the module's argv and must split it, delivering the
	/// module's own knobs through [`to_argv`](Self::to_argv) and forwarding the
	/// rest (the test-runner flags) untouched. It removes what it reads precisely
	/// so a knob cannot be forwarded twice and the two copies disagree, which is
	/// also what stops it being repurposed to peek at a field: a route that wants
	/// one flag reads that flag from its own params type.
	///
	/// Env participates because such a caller is itself configured like a process
	/// (`BEET_STORE=.. cargo test ..` reaches the runner only through the
	/// environment), and the child's environment is scrubbed of `BEET_*` on the
	/// way out, so the argv is the whole delivery.
	pub fn take_launch(
		params: &mut MultiMap<SmolStr, SmolStr>,
	) -> Result<Self> {
		let config = Self::parse(params, &Self::env_var)?;
		for knob in Self::KNOBS {
			params.remove(knob.arg);
		}
		Ok(config)
	}

	/// A `BEET_*` name resolved from the real process environment, the fallback
	/// channel of [the transport rule](Self#the-transport-rule).
	fn env_var(key: &str) -> Option<SmolStr> {
		env_ext::var(key).ok().map(SmolStr::from)
	}

	/// Every knob, in field order. The one enumeration of the table, so a new
	/// field is either listed here or is not a knob at all.
	const KNOBS: [Knob; 19] = [
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
			stage: reader
				.value(Self::STAGE)
				.unwrap_or_else(|| Self::DEFAULT_STAGE.into()),
			service_access: reader
				.parsed(Self::SERVICE_ACCESS)?
				.unwrap_or_default(),
			analytics_table: reader.value(Self::ANALYTICS_TABLE),
			deploy_id: reader.value(Self::DEPLOY_ID),
			deploy_timestamp: reader.value(Self::DEPLOY_TIMESTAMP),
			tls: reader.bool_value(Self::TLS)?,
			tls_dir: reader.value(Self::TLS_DIR),
			headless: reader.flag(Self::HEADLESS),
			screenshot: reader.value(Self::SCREENSHOT),
			screenshot_frame: reader.parsed(Self::SCREENSHOT_FRAME)?,
		}
		.xok()
	}

	/// Whether this process runs in the [production stage](Self::PROD_STAGE), the
	/// one stage that changes behaviour rather than just naming resources (a
	/// static export drops draft routes there).
	pub fn is_prod(&self) -> bool { self.stage == Self::PROD_STAGE }

	/// The stage as a renderable value, `None` when it is
	/// [`DEFAULT_STAGE`](Self::DEFAULT_STAGE): a field left at its default parses
	/// back to that default anyway, so rendering it would only add noise.
	fn rendered_stage(&self) -> Option<String> {
		(self.stage != Self::DEFAULT_STAGE).then(|| self.stage.to_string())
	}

	/// The service access as a renderable value, `None` when it is the default.
	/// See [`rendered_stage`](Self::rendered_stage).
	fn rendered_service_access(&self) -> Option<String> {
		(self.service_access != ServiceAccess::default())
			.then(|| self.service_access.to_string())
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
		push(Self::STAGE, self.rendered_stage());
		push(Self::SERVICE_ACCESS, self.rendered_service_access());
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
		push(Self::STAGE, self.rendered_stage());
		push(Self::SERVICE_ACCESS, self.rendered_service_access());
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

	/// This launch's bind address as IPv4 octets, see
	/// [`ipv4_octets`](Self::ipv4_octets).
	pub fn host_octets(&self) -> Option<[u8; 4]> {
		self.host.and_then(Self::ipv4_octets)
	}

	/// A `--host` address as IPv4 octets, the form the server components hold.
	///
	/// `bevy_reflect` has no [`IpAddr`] impl, so a markup-declarable server field
	/// stays `[u8; 4]`; the typed parse still lives here, which is what fixes the
	/// old literal `"0.0.0.0"` check (every other value, including a real
	/// address, silently became localhost). An IPv6 address warns and yields
	/// `None`: the listeners bind a v4 socket.
	///
	/// The one place that rule lives, shared with the boot-request overlay a
	/// server applies over its declared host.
	pub fn ipv4_octets(host: IpAddr) -> Option<[u8; 4]> {
		match host {
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

/// The process config, parsed on first read and never mutated.
///
/// A [`LazyLock`] rather than a settable cell: argv and env do not change, so the
/// value is the same whenever it is computed and *when* it freezes cannot matter.
/// That removes the whole class of ordering hazards a mutable global carries (a
/// lost write, a read that freezes the wrong value) rather than mitigating them.
/// no_std-capable, like [`CanonicalPort`](crate::prelude::CanonicalPort).
///
/// The initializer is deliberately the *lenient* parse: a panicking [`LazyLock`]
/// initializer poisons the cell, and this is reached from `Default` impls that
/// cannot fail.
static BOOTSTRAP: LazyLock<BootstrapConfig> = LazyLock::new(|| {
	BootstrapConfig::parse_lenient(
		&CliArgs::parse_env().params,
		&BootstrapConfig::env_var,
	)
});

/// Validates the process [`BootstrapConfig`] at [`PreStartup`], and under `std`
/// ensures a [`PackageConfig`] exists for the readers that expect one.
///
/// It does not *assign* the config: [`BootstrapConfig::get`] owns that, lazily and
/// immutably. What this adds is the strict [`BootstrapConfig::from_env`] parse, so
/// a malformed `--port=nope` fails the app through its error handler instead of
/// only warning. That matters more than it looks: the lazy parse can happen before
/// `LogPlugin` initializes, where its warnings go nowhere, while a `PreStartup`
/// system runs with logging up.
#[derive(Default)]
pub struct BootstrapPlugin;

impl Plugin for BootstrapPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(PreStartup, validate_process_config);
		// `PackageConfig` is std-only (it kebab-cases cloud resource names)
		#[cfg(feature = "std")]
		app.add_systems(
			PreStartup,
			seed_package_config.after(validate_process_config),
		);
	}
}

/// The [`PreStartup`] validation, see [`BootstrapPlugin`]. The parsed value is
/// discarded: [`BootstrapConfig::get`] already holds the equivalent lenient parse,
/// so this exists purely to raise on a malformed field.
fn validate_process_config() -> Result {
	BootstrapConfig::from_env()?;
	Ok(())
}

/// The [`PreStartup`] [`PackageConfig`] seed, see [`BootstrapPlugin`]. Inserts
/// the defaults unless a host already supplied one (a
/// [`pkg_config!`](crate::pkg_config) with the real crate metadata), so a
/// `Res<PackageConfig>` reader never faces a missing resource. A markup
/// `<PackageConfig/>` patches it afterwards, keeping whatever it does not name.
#[cfg(feature = "std")]
fn seed_package_config(world: &mut World) {
	world.init_resource::<PackageConfig>();
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

	/// A [`ServerFilter`], through the filter's own param grammar so the boot-time
	/// read and this one cannot diverge, falling back to the env transport.
	/// `None` when the knob is absent from both, which is what lets a server fall
	/// back to its own `default_boot`.
	fn filter(&self, knob: Knob) -> Option<ServerFilter> {
		ServerFilter::from_params(self.params).or_else(|| {
			(self.env)(knob.env)
				.map(|value| ServerFilter::new(value.as_str()))
		})
	}
}

#[cfg(test)]
mod test {
	use super::BootstrapConfig;
	use crate::prelude::*;

	/// Parse an argv line alone, with no environment. The real parse is reached
	/// through [`BootstrapConfig::from_env`], which is not test-drivable (it reads
	/// the harness's own argv), so the cases exercise the shared inner parse.
	fn parse_argv(args: &str) -> Result<BootstrapConfig> {
		BootstrapConfig::parse(&CliArgs::parse(args).params, &|_| None)
	}

	/// A config with every field set, so a round trip exercises each one.
	fn full() -> BootstrapConfig {
		BootstrapConfig::launch()
			.with_main("main.bsx")
			.with_store(StoreUri::parse("s3://site?region=us-west-2").unwrap())
			.with_watch(true)
			.with_features(vec!["thread".into(), "sockets".into()])
			.with_server(ServerFilter::new("http,ssh"))
			.with_path("/docs")
			.with_host("0.0.0.0".parse().unwrap())
			.with_http_port(8337)
			.with_ssh_port(2222)
			.with_stage("prod")
			.with_service_access(ServiceAccess::Remote)
			.with_analytics_table("beet--prod--analytics")
			.with_deploy_id("019823ff-0000-7000-8000-000000000000")
			.with_deploy_timestamp("2026-08-09T00:00:00Z")
			.with_tls(true)
			.with_tls_dir("/tmp/tls")
			.with_headless(true)
			.with_screenshot("/tmp/shot.png")
			.with_screenshot_frame(30)
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
		BootstrapConfig::parse(&params, &|_| None)
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

	/// A bare launch renders to nothing on either channel.
	#[crate::test]
	fn default_renders_empty() {
		BootstrapConfig::launch().to_argv().unwrap().len().xpect_eq(0);
		BootstrapConfig::launch().to_env().len().xpect_eq(0);
	}

	/// A field with a static default renders only when it differs from it: the
	/// deploy never writes `BEET_STAGE=dev` because the runtime resolves `dev`
	/// anyway, while a named stage always reaches both channels.
	#[crate::test]
	fn defaulted_fields_render_only_when_named() {
		BootstrapConfig::launch()
			.with_stage("staging")
			.with_service_access(ServiceAccess::Remote)
			.to_argv()
			.unwrap()
			.join(" ")
			.xpect_eq("--stage=staging --service-access=remote");
		// the defaults are what an unset knob parses back to, so dropping them is
		// lossless
		parse_argv("").unwrap().xpect_eq(BootstrapConfig::launch());
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
		config.http_port().xpect_eq(Some(9000));
		// a field argv did not set still falls back to env
		config.stage().as_str().xpect_eq("prod");
	}

	/// A token the renderers cannot encode is a loud error, not a corrupted
	/// exec line: the caveat the hand-written encoders carried becomes a type
	/// error.
	#[crate::test]
	fn rejects_unencodable_tokens() {
		BootstrapConfig::launch()
			.with_main("my entry.bsx")
			.to_argv()
			.unwrap_err()
			.to_string()
			.xpect_contains("cannot be rendered");
		BootstrapConfig::launch()
			.with_main("say\"hi\"")
			.to_cmd_json("/app")
			.unwrap_err()
			.to_string()
			.xpect_contains("cannot be rendered");
	}

	#[crate::test]
	fn renders_cmd_json() {
		BootstrapConfig::launch()
			.with_store(StoreUri::parse("s3://site").unwrap())
			.with_server(ServerFilter::new("http,ssh"))
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
		parse_argv("--port=nope")
			.unwrap_err()
			.to_string()
			.xpect_contains("invalid --port / BEET_HTTP_PORT");
		parse_argv("--host=nope")
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
		config.http_port().xpect_none();
		config.stage().as_str().xpect_eq("prod");
	}

	/// `--server` present but empty is a selection with no constraint, distinct
	/// from an absent `--server` (which leaves each server's `default_boot` to
	/// decide).
	#[crate::test]
	fn empty_server_selection_is_present() {
		parse_argv("--server")
			.unwrap()
			.server()
			.clone()
			.unwrap()
			.passes("http")
			.xpect_true();
		parse_argv("").unwrap().server().xpect_none();
	}

	/// Beet spawning beet: the launch knobs leave the params (delivered on the
	/// child's argv instead) and everything else is untouched, so a flag can
	/// never be forwarded twice.
	#[crate::test]
	fn take_launch_consumes_only_the_knobs() {
		let mut params = CliArgs::parse("--port=9090 --nocapture").params;
		BootstrapConfig::take_launch(&mut params)
			.unwrap()
			.http_port()
			.xpect_eq(Some(9090));
		params.contains_key("port").xpect_false();
		params.contains_key("nocapture").xpect_true();
	}
}
