//! The process-config type every beet binary boots from.

use crate::prelude::*;
use bevy::platform::sync::LazyLock;
use bevy::reflect::TypeInfo;
use bevy::reflect::Typed;
use core::net::IpAddr;
use heck::ToKebabCase;
use heck::ToShoutySnakeCase;

/// Every pre-scene knob a beet process boots from, in one type.
///
/// ## What it is, and is not
///
/// A `BootstrapConfig` describes **one process launch**. There are exactly two
/// things to do with one: read the launch *this* process booted from
/// ([`get`](Self::get) / [`from_env`](Self::from_env)), or construct one to
/// describe the launch of a process you are about to start, rendered through
/// [`to_argv`](Self::to_argv) / [`to_env`](Self::to_env) /
/// [`to_cmd_json`](Self::to_cmd_json) by `ChildProcess::with_bootstrap` and the
/// deploy blocks.
///
/// It is **not** a params parser. A route reading one flag off its request
/// takes that flag (an `#[action]`'s `#[param]`, so `--help` documents it),
/// because a request is not a process launch: pulling a whole config out of one
/// to reach a single field silently drags the `BEET_*` environment in behind it
/// and hides the flag from the route's own help.
///
/// ## The transport rule
///
/// Each field's doc comment is its `--help` description, listed under "Launch"
/// by the root help, so it is written as plain text.
///
/// One rule, derived from the reflected fields with no per-field exceptions:
/// **a knob's flag is its kebab-case field name and its environment variable is
/// `BEET_` plus its screaming snake field name, and argv wins**, so `--repo` /
/// `BEET_REPO`, `--store-fork` / `BEET_STORE_FORK`. An unqualified `port` is
/// the http port, everywhere: the field, the flag and the variable. A list knob
/// is comma separated on either transport. Both channels exist because both are
/// natural affordances: a human types argv, a platform writes env into a unit
/// file or a task definition.
///
/// ## The process request
///
/// The same argv is the process request a `CliServer` dispatches, so the
/// launch only knobs are [stripped](Self::strip) from it first: the request
/// keeps only the [`REQUEST_RANK`](Self::REQUEST_RANK) knobs, the ones a server
/// overlays at request rank. The root `--help` lists the rest under "Launch".
///
/// ## Reading it
///
/// **The process config is a process-global singleton, not a resource.** Launch
/// arguments are a fact about the process, not state an app owns: there is one
/// argv, it never changes, and it must be readable from places that have no
/// world at all (a [`Default`] impl, a deploy [`Stack`]'s stage resolution, a
/// plugin's `build`, the
/// binary's pre-world entry resolution). Modelling it as a resource forced world
/// access into all of those *and* left two sources of truth, since a `Default`
/// impl could only reach the environment. So it follows `CanonicalPort`: one
/// static, read through [`get`](Self::get) from anywhere.
///
/// - [`get`](Self::get): the process config, parsed from argv + env on first
///   read and memoized. Never fails; a malformed field warns and falls back to
///   its static default, since a `Default` impl has no error channel.
/// - [`from_env`](Self::from_env): the same parse, *strict*. `BootstrapPlugin`
///   runs it so a malformed `--port=nope` exits with the usage code instead of
///   only warning.
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
#[derive(Debug, Clone, PartialEq, Eq, Reflect)]
#[reflect(Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct BootstrapConfig {
	/// The entry document: a path for a dir-rooted repo store, a name within a
	/// self-rooted one.
	pub entry: Option<SmolStr>,
	/// The repo store the entry loads through.
	pub repo: Option<StoreUri>,
	/// The local store the repo store is forked into: reads fall through to
	/// the repo, writes land here, so a process on a remote repo forks rather
	/// than writes back. Absent, a browser whose repo is remote forks into
	/// IndexedDB; every other launch reads its repo directly.
	// the browser default is `entry_build::default_store_fork`
	pub store_fork: Option<StoreUri>,
	/// Watch the entry's sources and live-reload.
	pub watch: bool,
	/// Cargo features this binary must have been built with, refusing the
	/// launch otherwise, as Cargo's own `required-features` gates a target.
	pub required_features: Vec<SmolStr>,
	/// Which declared servers boot.
	pub server: Option<ServerFilter>,
	/// The opening route a freshly-opened tui/ssh surface navigates to.
	pub path: Option<SmolStr>,
	/// The address servers bind to, usually read through
	/// [`host_octets`](Self::host_octets).
	pub host: Option<IpAddress>,
	/// The http listener port.
	pub port: Option<u16>,
	/// The ssh listener port.
	pub ssh_port: Option<u16>,
	/// The infrastructure stage this process runs in, defaulting to `dev`.
	pub stage: SmolStr,
	/// Whether services resolve locally or against the cloud, defaulting to
	/// `local`.
	pub service_access: ServiceAccess,
	/// This deployment's unique id.
	pub deploy_id: Option<SmolStr>,
	/// This deployment's timestamp.
	pub deploy_timestamp: Option<SmolStr>,
	/// Force TLS on or off, overriding the managed-platform detection.
	pub tls: Option<bool>,
	/// Where the cached dev certificate lives.
	pub tls_dir: Option<SmolStr>,
	/// Run terminal surfaces buffered, never touching the real tty.
	pub headless: bool,
	/// Capture the first window to this PNG and exit.
	pub screenshot: Option<SmolStr>,
	/// The frame the screenshot harness captures on.
	pub screenshot_frame: Option<u32>,
}

/// Every field's static default, ie what an unset knob resolves to. Also the
/// baseline the renderers diff against, so a field left at its default never
/// reaches an argv line or an env pair, and the base a launch description is
/// built over: `BootstrapConfig { repo, ..default() }`.
impl Default for BootstrapConfig {
	fn default() -> Self {
		Self {
			entry: None,
			repo: None,
			store_fork: None,
			watch: false,
			required_features: default(),
			server: None,
			path: None,
			host: None,
			port: None,
			ssh_port: None,
			stage: Self::DEFAULT_STAGE.into(),
			service_access: default(),
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

impl BootstrapConfig {
	/// The stage an unqualified launch runs in. A deploy names its own stage on
	/// either transport, so only a local run relies on this.
	pub const DEFAULT_STAGE: &'static str = "dev";
	/// The stage that turns on production behaviour, ie dropping draft routes
	/// from a static export.
	pub const PROD_STAGE: &'static str = "prod";
	/// The `type` of the `<script>` a served page carries its browser launch
	/// in, see [`to_script`](Self::to_script).
	pub const SCRIPT_TYPE: &'static str = "application/vnd.beet.bootstrap";
	/// The knobs that stay in the process request when the rest are
	/// [stripped](Self::strip), as flags: a server reads them at request rank,
	/// overlaying them above its markup-declared fields, so `serve
	/// --server=http --port=8080` reaches the server the `serve` route boots.
	pub const REQUEST_RANK: [&'static str; 5] =
		["server", "path", "host", "port", "ssh-port"];

	/// Parse this process's argv, with the `BEET_*` environment as the fallback for
	/// every field argv did not set. Strict: a malformed value errors.
	///
	/// Most callers want [`get`](Self::get), which memoizes and cannot fail. This
	/// is what [`BootstrapPlugin`] runs at [`PreStartup`] so a malformed value,
	/// a [`ParamError`], still stops the app.
	///
	/// In a browser [`env_ext::args`] yields the location query plus the page's
	/// bootstrap script ([`to_script`](Self::to_script)), so a served page names
	/// the launch and `?server=http` still selects; in a Cloudflare Worker argv
	/// is empty and the environment is the Worker's `vars`, seeded into
	/// [`env_ext`] by its entry before this first reads, so `BEET_REPO` selects
	/// the store exactly as `--repo` does.
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

	/// Remove every launch only knob from `params`, leaving the
	/// [`REQUEST_RANK`](Self::REQUEST_RANK) ones: the process request a
	/// `CliServer` dispatches carries what its routes and servers read, never
	/// `--entry` or `--stage`, which the launch already consumed.
	pub fn strip(params: &mut MultiMap<SmolStr, SmolStr>) {
		for field in Self::field_names() {
			let flag = field.to_kebab_case();
			if !Self::REQUEST_RANK.contains(&flag.as_str()) {
				params.remove(flag.as_str());
				params.remove(field);
			}
		}
	}

	/// A `BEET_*` name resolved from the real process environment, the fallback
	/// channel of [the transport rule](Self#the-transport-rule).
	fn env_var(key: &str) -> Option<SmolStr> {
		env_ext::var(key).ok().map(SmolStr::from)
	}

	/// Every field name, in declaration order: the one enumeration of the
	/// knobs, so a new field is a knob on both transports by being declared.
	fn field_names() -> impl Iterator<Item = &'static str> {
		Self::type_info()
			.as_struct()
			.into_iter()
			.flat_map(|info| info.iter().map(|field| field.name()))
	}

	/// The environment variable a field reads, ie `BEET_STORE_FORK`.
	fn env_name(field: &str) -> SmolStr {
		format!("BEET_{}", field.to_shouty_snake_case()).into()
	}

	/// Every argv key a beet launch reads, ie the `entry` of `--entry`, derived
	/// from the field names.
	pub fn arg_names() -> impl Iterator<Item = SmolStr> {
		Self::field_names().map(|field| field.to_kebab_case().into())
	}

	/// Every environment variable name a beet launch reads, derived from the
	/// field names so a new knob cannot be forgotten.
	///
	/// Every one is `BEET_*`, `BEET_WORKSPACE_ROOT` included, so
	/// [`ChildProcess::without_launch_env`]'s prefix scrub already reaches them
	/// all and this is belt and braces for a name that happens to be unset.
	pub(crate) fn env_names() -> impl Iterator<Item = SmolStr> {
		let names = Self::field_names().map(Self::env_name);
		// the workspace root is the std filesystem's own variable
		#[cfg(feature = "std")]
		let names = names.chain([SmolStr::from(fs_ext::WORKSPACE_ROOT)]);
		names
	}

	/// The one parse. `env` resolves a `BEET_*` name, and is consulted only for a
	/// field the params did not set.
	fn parse(
		params: &MultiMap<SmolStr, SmolStr>,
		env: &dyn Fn(&str) -> Option<SmolStr>,
	) -> Result<Self> {
		let mut config = Self::default();
		for field in Self::field_names() {
			config.apply_field(field, params, env)?;
		}
		config.xok()
	}

	/// [`parse`](Self::parse) with each malformed field warned about and left
	/// at its default instead of failing the whole parse.
	fn parse_lenient(
		params: &MultiMap<SmolStr, SmolStr>,
		env: &dyn Fn(&str) -> Option<SmolStr>,
	) -> Self {
		let mut config = Self::default();
		for field in Self::field_names() {
			if let Err(err) = config.apply_field(field, params, env) {
				warn!("{err}");
			}
		}
		config
	}

	/// Apply one field over its current value: its params values when the
	/// request names it, else its `BEET_*` variable, through the same reflected
	/// read every request param takes. A malformed value is a [`ParamError`]
	/// naming both transports.
	fn apply_field(
		&mut self,
		field: &'static str,
		params: &MultiMap<SmolStr, SmolStr>,
		env: &dyn Fn(&str) -> Option<SmolStr>,
	) -> Result {
		let flag = field.to_kebab_case();
		let env_name = Self::env_name(field);
		let info = Self::type_info()
			.as_struct()?
			.field(field)
			.and_then(|field| field.type_info());
		let Some(values) = params
			.get_vec(flag.as_str())
			.or_else(|| params.get_vec(field))
			.cloned()
			.and_then(|values| Self::present(values, info))
			.or_else(|| env(&env_name).map(|value| vec![value]))
		else {
			return Ok(());
		};
		let values = match Self::is_list(info) {
			true => {
				let names = values
					.iter()
					.flat_map(|value| str_ext::csv(value))
					.map(SmolStr::from)
					.collect::<Vec<_>>();
				match names.is_empty()
					&& !matches!(Self::leaf(info), Some(TypeInfo::List(_)))
				{
					// a newtype list (`--server`) reads present and empty
					true => vec![SmolStr::default()],
					false => names,
				}
			}
			false => values,
		};
		let mut map = MultiMap::<SmolStr, SmolStr>::default();
		map.insert_vec(field.into(), values);
		map.apply_reflect(self).map_err(|err| {
			let reason = match err.downcast_ref::<ParamError>() {
				Some(param) => match param.problem() {
					ParamProblem::Invalid { reason } => reason.clone(),
					_ => param.to_string(),
				},
				None => err.to_string(),
			};
			ParamError::invalid(
				&flag,
				format!("{reason} (`--{flag}` / `{env_name}`)"),
			)
			.into()
		})
	}

	/// The values a present params key carries, `None` to fall back to env: a
	/// bare key is a flag's `true` and a list's empty, but names no value for
	/// any other knob.
	fn present(
		values: Vec<SmolStr>,
		info: Option<&TypeInfo>,
	) -> Option<Vec<SmolStr>> {
		match values.is_empty() {
			false => Some(values),
			true if Self::is_list(info)
				|| Self::leaf(info).is_some_and(|leaf| {
					leaf.type_id() == core::any::TypeId::of::<bool>()
				}) =>
			{
				Some(values)
			}
			true => None,
		}
	}

	/// A field type with any `Option` wrapper seen through.
	fn leaf(info: Option<&TypeInfo>) -> Option<&TypeInfo> {
		info.map(|info| reflect_ext::option_some_inner(info).unwrap_or(info))
	}

	/// Whether a field is a list knob, comma separated on either transport: a
	/// list, or a newtype over one ([`ServerFilter`]).
	fn is_list(info: Option<&TypeInfo>) -> bool {
		match Self::leaf(info) {
			Some(TypeInfo::List(_)) => true,
			Some(TypeInfo::TupleStruct(newtype)) => newtype
				.field_at(0)
				.and_then(|field| field.type_info())
				.is_some_and(|inner| matches!(inner, TypeInfo::List(_))),
			_ => false,
		}
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
		let mut push = |field: &str, value: Option<String>| {
			if let Some(value) = value {
				argv.push(SmolStr::from(format!(
					"--{}={value}",
					field.to_kebab_case()
				)));
			}
		};
		self.render(&mut push);
		// bare flags: `--watch` parses back as a flag, so no value is rendered.
		for (field, set) in [("watch", self.watch), ("headless", self.headless)]
		{
			if set {
				argv.push(SmolStr::from(format!("--{field}")));
			}
		}
		for token in &argv {
			Self::validate_token(token)?;
		}
		argv.xok()
	}

	/// This launch as the text of a served page's bootstrap script, the
	/// [`to_argv`](Self::to_argv) tokens space-joined (each is whitespace-free
	/// by construction), read back by [`script_argv`](Self::script_argv). A
	/// page renders it in a `<script type="application/vnd.beet.bootstrap">`
	/// ([`SCRIPT_TYPE`](Self::SCRIPT_TYPE)) beside its module loader, and the
	/// browser process appends it to the location's own argv.
	pub fn to_script(&self) -> Result<String> {
		self.to_argv()?.join(" ").xok()
	}

	/// The argv a bootstrap script's text carries, the inverse of
	/// [`to_script`](Self::to_script).
	pub fn script_argv(text: &str) -> Vec<SmolStr> {
		text.split_whitespace().map(SmolStr::from).collect()
	}

	/// Every valued knob in render order, handed to `push` as
	/// `(field, value)`, the one enumeration [`to_argv`](Self::to_argv) and
	/// [`to_env`](Self::to_env) share so the two channels cannot drift. The
	/// round trip test pins it to the parse.
	fn render(&self, push: &mut impl FnMut(&str, Option<String>)) {
		fn shown(value: Option<&impl ToString>) -> Option<String> {
			value.map(ToString::to_string)
		}
		push("entry", shown(self.entry.as_ref()));
		push("repo", shown(self.repo.as_ref()));
		push("store_fork", shown(self.store_fork.as_ref()));
		push("server", shown(self.server.as_ref()));
		push("path", shown(self.path.as_ref()));
		push("host", shown(self.host.as_ref()));
		push("port", shown(self.port.as_ref()));
		push("ssh_port", shown(self.ssh_port.as_ref()));
		push("stage", self.rendered_stage());
		push("service_access", self.rendered_service_access());
		push("deploy_id", shown(self.deploy_id.as_ref()));
		push("deploy_timestamp", shown(self.deploy_timestamp.as_ref()));
		push("tls", shown(self.tls.as_ref()));
		push("tls_dir", shown(self.tls_dir.as_ref()));
		push("screenshot", shown(self.screenshot.as_ref()));
		push("screenshot_frame", shown(self.screenshot_frame.as_ref()));
		push(
			"required_features",
			(!self.required_features.is_empty())
				.then(|| self.required_features.join(",")),
		);
	}

	/// Every set field as `BEET_*` environment pairs, the exact names the parse
	/// reads back. Consumed by a task definition, a systemd unit's
	/// `Environment=` lines, a lambda function env and a Worker's env object.
	pub fn to_env(&self) -> Vec<(SmolStr, SmolStr)> {
		let mut pairs: Vec<(SmolStr, SmolStr)> = Vec::new();
		let mut push = |field: &str, value: Option<String>| {
			if let Some(value) = value {
				pairs.push((Self::env_name(field), value.into()));
			}
		};
		self.render(&mut push);
		// a flag reads back `true` from any truthy value
		push("watch", self.watch.then(|| "1".to_string()));
		push("headless", self.headless.then(|| "1".to_string()));
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
	/// its documented default channel: boot selection (the repo store, the server
	/// selection, the opening path) is visible on argv, ambient service config
	/// (the bind address, the ports, the stage, the deploy identity) rides env.
	///
	/// The dev-harness fields (`entry`, `watch`, `required_features`, `tls_dir`,
	/// `headless`, `screenshot*`) belong to neither deploy channel and
	/// are dropped: a deploy has no use for them, and a deployed process probes
	/// its repo store for the entry rather than being told.
	pub fn split_channels(self) -> (Self, Self) {
		let argv = Self {
			repo: self.repo,
			store_fork: self.store_fork,
			server: self.server,
			path: self.path,
			..default()
		};
		let env = Self {
			host: self.host,
			port: self.port,
			ssh_port: self.ssh_port,
			stage: self.stage,
			service_access: self.service_access,
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
		self.host.and_then(|host| Self::ipv4_octets(*host))
	}

	/// A `--host` address as IPv4 octets, the form the server components hold.
	///
	/// A markup-declarable server field stays `[u8; 4]`; the typed parse still
	/// lives here, which is what fixes the old literal `"0.0.0.0"` check (every
	/// other value, including a real address, silently became localhost). An
	/// IPv6 address warns and yields `None`: the listeners bind a v4 socket.
	///
	/// The one place that rule lives, shared with the boot-request overlay a
	/// server applies over its declared host.
	pub fn ipv4_octets(host: IpAddr) -> Option<[u8; 4]> {
		match host {
			IpAddr::V4(addr) => Some(addr.octets()),
			IpAddr::V6(addr) => {
				warn!(
					"ignoring --host / BEET_HOST `{addr}`: beet servers bind IPv4"
				);
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

/// Loads a developer's `.env` into the process environment, validates the
/// process [`BootstrapConfig`] at [`PreStartup`], and under `std` registers
/// [`PackageConfig`] and inserts one for the readers that expect it.
///
/// The `.env` load ([`env_ext::load_dotenv`]) runs at build, so it lands
/// before the first [`BootstrapConfig::get`] freezes the environment and no
/// binary's `main` has to remember it; a plugin group puts this plugin first
/// for the same reason. `.env` is a developer's own file of flags, plaintext
/// and never committed: nothing beet needs lives there, since an entry's
/// secrets ride its `<Secrets>` document, loaded by the launch before the
/// entry builds.
///
/// It does not *assign* the config: [`BootstrapConfig::get`] owns that, lazily and
/// immutably. What this adds is the strict [`BootstrapConfig::from_env`] parse, so
/// a malformed `--port=nope` exits with the usage code before anything runs,
/// instead of only warning. That matters more than it looks: the lazy parse can happen before
/// `LogPlugin` initializes, where its warnings go nowhere, while a `PreStartup`
/// system runs with logging up.
#[derive(Default)]
pub struct BootstrapPlugin;

impl Plugin for BootstrapPlugin {
	fn build(&self, app: &mut App) {
		// a missing `.env` is the common case; a host with no environment to
		// load into (no_std, a browser tab) is the other non-failure
		env_ext::load_dotenv().ok();
		app.add_systems(PreStartup, validate_process_config);
		// `PackageConfig` is std-only (it kebab-cases cloud resource names).
		// Registered here rather than by a router, so a routerless app can
		// author `<PackageConfig app_name=".."/>` and a deploy can resolve it.
		//
		// Inserted at BUILD time, not from a schedule: a template build (and the
		// stack resolution under it) can run before the first `PreStartup`, so a
		// seed system would leave `Res<PackageConfig>` genuinely absent for the
		// readers that need it most. `init_resource` yields to a host that
		// already supplied a `pkg_config!()`, and a markup `<PackageConfig/>`
		// patches it afterwards, keeping whatever it does not name.
		#[cfg(feature = "std")]
		app.register_type::<PackageConfig>()
			.init_resource::<PackageConfig>();
	}
}

/// The [`PreStartup`] validation, see [`BootstrapPlugin`]. The parsed value is
/// discarded: [`BootstrapConfig::get`] already holds the equivalent lenient parse,
/// so this exists purely to raise on a malformed field.
///
/// A malformed knob is the caller's mistake, so it exits with the usage code
/// before anything runs; without `std` it raises through the error handler.
fn validate_process_config() -> Result {
	let Err(err) = BootstrapConfig::from_env() else {
		return Ok(());
	};
	#[cfg(feature = "std")]
	if let Some(param) = err.downcast_ref::<ParamError>() {
		error!("{param}\n`--help` lists the params it takes");
		process_ext::exit(ParamError::EXIT_CODE);
	}
	Err(err)
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
		BootstrapConfig {
			entry: Some("main.bsx".into()),
			repo: Some(StoreUri::parse("s3://site?region=us-west-2").unwrap()),
			store_fork: Some(StoreUri::parse("fs:/tmp/fork").unwrap()),
			watch: true,
			required_features: vec!["thread".into(), "sockets".into()],
			server: Some(ServerFilter::new("http,ssh")),
			path: Some("/docs".into()),
			host: Some("0.0.0.0".parse().unwrap()),
			port: Some(8337),
			ssh_port: Some(2222),
			stage: "prod".into(),
			service_access: ServiceAccess::Remote,
			deploy_id: Some("019823ff-0000-7000-8000-000000000000".into()),
			deploy_timestamp: Some("2026-08-09T00:00:00Z".into()),
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
		let params = CliArgs::parse_tokens(argv).params;
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

	/// A default config renders to nothing on either channel.
	#[crate::test]
	fn default_renders_empty() {
		BootstrapConfig::default()
			.to_argv()
			.unwrap()
			.len()
			.xpect_eq(0);
		BootstrapConfig::default().to_env().len().xpect_eq(0);
	}

	/// A field with a static default renders only when it differs from it: the
	/// deploy never writes `BEET_STAGE=dev` because the runtime resolves `dev`
	/// anyway, while a named stage always reaches both channels.
	#[crate::test]
	fn defaulted_fields_render_only_when_named() {
		BootstrapConfig {
			stage: "staging".into(),
			service_access: ServiceAccess::Remote,
			..default()
		}
		.to_argv()
		.unwrap()
		.join(" ")
		.xpect_eq("--stage=staging --service-access=remote");
		// the defaults are what an unset knob parses back to, so dropping them is
		// lossless
		parse_argv("").unwrap().xpect_eq(BootstrapConfig::default());
	}

	/// Argv beats env, per field, with no exceptions.
	#[crate::test]
	fn argv_beats_env() {
		let params = CliArgs::parse("--port=9000").params;
		let env = |key: &str| match key {
			"BEET_PORT" => Some(SmolStr::from("8337")),
			"BEET_STAGE" => Some(SmolStr::from("prod")),
			_ => None,
		};
		let config = BootstrapConfig::parse(&params, &env).unwrap();
		config.port.xpect_eq(Some(9000));
		// a field argv did not set still falls back to env
		config.stage.as_str().xpect_eq("prod");
	}

	/// A token the renderers cannot encode is a loud error, not a corrupted
	/// exec line: the caveat the hand-written encoders carried becomes a type
	/// error.
	#[crate::test]
	fn rejects_unencodable_tokens() {
		BootstrapConfig {
			entry: Some("my entry.bsx".into()),
			..default()
		}
		.to_argv()
		.unwrap_err()
		.to_string()
		.xpect_contains("cannot be rendered");
		BootstrapConfig {
			entry: Some("say\"hi\"".into()),
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
			repo: Some(StoreUri::parse("s3://site").unwrap()),
			server: Some(ServerFilter::new("http,ssh")),
			..default()
		}
		.to_cmd_json("/app")
		.unwrap()
		.xpect_eq(r#"["/app", "--repo=s3://site", "--server=http,ssh"]"#);
	}

	/// Boot selection rides argv, ambient service config rides env, and the
	/// dev-harness fields ride neither.
	#[crate::test]
	fn splits_channels() {
		let (argv, env) = full().split_channels();
		argv.to_argv().unwrap().join(" ").xpect_eq(
			"--repo=s3://site?region=us-west-2 --store-fork=fs:/tmp/fork \
			 --server=http,ssh --path=/docs",
		);
		env.to_env()
			.iter()
			.map(|(key, _)| key.as_str())
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				"BEET_HOST",
				"BEET_PORT",
				"BEET_SSH_PORT",
				"BEET_STAGE",
				"BEET_SERVICE_ACCESS",
				"BEET_DEPLOY_ID",
				"BEET_DEPLOY_TIMESTAMP",
				"BEET_TLS",
			]);
	}

	/// A malformed value errors at parse rather than silently taking a default.
	#[crate::test]
	fn malformed_errors() {
		let port = parse_argv("--port=nope").unwrap_err();
		port.to_string()
			.xpect_contains("invalid param `--port`: invalid number \"nope\"")
			.xpect_contains("(`--port` / `BEET_PORT`)");
		// the caller's mistake, which a launch exits on with the usage code
		port.downcast_ref::<ParamError>()
			.unwrap()
			.key()
			.xpect_eq("port");
		parse_argv("--host=nope")
			.unwrap_err()
			.to_string()
			.xpect_contains("invalid param `--host`")
			.xpect_contains("`BEET_HOST`");
	}

	/// A `Default` impl has no error channel, so one malformed field warns and
	/// resolves to `None` while every other field still parses.
	#[crate::test]
	fn lenient_drops_only_the_malformed_field() {
		let config = BootstrapConfig::parse_lenient(
			&CliArgs::parse("--port=nope --stage=prod").params,
			&|_| None,
		);
		config.port.xpect_none();
		config.stage.as_str().xpect_eq("prod");
	}

	/// `--server` present but empty is a selection with no constraint, distinct
	/// from an absent `--server` (which leaves each server's `default_boot` to
	/// decide).
	#[crate::test]
	fn empty_server_selection_is_present() {
		parse_argv("--server")
			.unwrap()
			.server
			.unwrap()
			.passes("http")
			.xpect_true();
		parse_argv("").unwrap().server.xpect_none();
	}

	/// The browser channel: a page's bootstrap script appended to the
	/// location's own argv parses back to the launch, with the location's
	/// path and query still positionals and flags of the same request, and the
	/// `--server` it carries selects the facet by name.
	#[crate::test]
	fn script_appends_to_the_location_argv() {
		let launch = BootstrapConfig {
			entry: Some("scene_editor.bsx".into()),
			repo: Some(StoreUri::parse("http:examples/ui").unwrap()),
			server: Some(ServerFilter::new("dom")),
			..default()
		};
		let script = launch.to_script().unwrap();
		script.xpect_eq(
			"--entry=scene_editor.bsx --repo=http:examples/ui --server=dom",
		);
		// the page at `/docs?color-scheme=dark` booting through the script
		let mut argv = vec![SmolStr::new("docs"), "--color-scheme=dark".into()];
		argv.extend(BootstrapConfig::script_argv(&script));
		let args = CliArgs::parse_tokens(argv);
		args.path.xpect_eq(vec![SmolStr::new("docs")]);
		args.params
			.get("color-scheme")
			.xpect_eq(Some(&"dark".into()));
		BootstrapConfig::parse(&args.params, &|_| None)
			.unwrap()
			.xpect_eq(launch);
		let filter = ServerFilter::from_params(&args.params).unwrap();
		filter.passes("dom").xpect_true();
		filter.passes("tui").xpect_false();
	}

	/// The process request keeps only the knobs a server reads at request
	/// rank, in either spelling, and every param that is no knob at all.
	#[crate::test]
	fn strip_keeps_the_request_rank() {
		let mut params = CliArgs::parse(
			"--entry=site --store_fork=fs:/tmp --port=9090 --server=http --fix",
		)
		.params;
		BootstrapConfig::strip(&mut params);
		let mut keys = params.keys().map(SmolStr::as_str).collect::<Vec<_>>();
		keys.sort();
		keys.xpect_eq(vec!["fix", "port", "server"]);
	}

	/// The knob names derive from the fields: the kebab flag and the
	/// `BEET_` screaming snake variable.
	#[crate::test]
	fn names_derive_from_the_fields() {
		BootstrapConfig::arg_names()
			.any(|name| name == "store-fork")
			.xpect_true();
		BootstrapConfig::env_names()
			.any(|name| name == "BEET_STORE_FORK")
			.xpect_true();
		BootstrapConfig::REQUEST_RANK
			.iter()
			.all(|knob| BootstrapConfig::arg_names().any(|name| name == *knob))
			.xpect_true();
	}
}
