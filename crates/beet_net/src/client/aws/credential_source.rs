use crate::prelude::*;
use beet_core::prelude::*;
use std::sync::Arc;
use std::sync::LazyLock;

/// Where an [`AwsCredentials`] comes from: an erased
/// [`AwsCredentialProvider`] behind a cache. A cached credential is answered
/// until five minutes before its expiry, then loaded afresh; a failure is
/// never cached, so a process that gains credentials resolves on its next
/// call.
///
/// [`default_chain`](Self::default_chain) is the launch's own, shared by
/// every client in the process. The explicit identities (the `beet-agent`
/// pair, an admin session, a member account's role, an Identity Center
/// session) are sources of their own, never links of the chain.
#[derive(Clone)]
pub struct AwsCredentialSource(Arc<SourceInner>);

struct SourceInner {
	provider: Box<dyn AwsCredentialProvider>,
	cache: async_lock::Mutex<Option<AwsCredentials>>,
	clock: fn() -> Timestamp,
}

impl core::fmt::Debug for AwsCredentialSource {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_tuple("AwsCredentialSource")
			.field(&self.describe())
			.finish()
	}
}

/// One way of answering credentials, erased behind an
/// [`AwsCredentialSource`].
pub trait AwsCredentialProvider: 'static + Send + Sync {
	/// What this is, for an error naming where credentials were looked for,
	/// ie `the environment`.
	fn describe(&self) -> SmolStr;

	/// The credential, `Ok(None)` when this provider has none to give (no
	/// variables set, not on EC2) and an error when it should and failed.
	fn load(&self) -> SendBoxedFuture<Result<Option<AwsCredentials>>>;

	/// Whether loading again may answer a credential other than the one
	/// that lapsed, which is what a refusal's recovery asks. A fixed pair
	/// cannot.
	fn refreshable(&self) -> bool { true }
}

impl AwsCredentialSource {
	/// How long before its expiry a cached credential is loaded afresh.
	pub const REFRESH_WINDOW: Duration = Duration::from_secs(5 * 60);

	/// A source over `provider`.
	pub fn new(provider: impl AwsCredentialProvider) -> Self {
		Self(Arc::new(SourceInner {
			provider: Box::new(provider),
			cache: default(),
			clock: AwsClock::now,
		}))
	}

	/// The launch's own credential, resolved through
	/// [`AwsDefaultChain`] and cached once per process.
	pub fn default_chain() -> Self {
		static CHAIN: LazyLock<AwsCredentialSource> =
			LazyLock::new(|| AwsCredentialSource::new(AwsDefaultChain));
		CHAIN.clone()
	}

	/// The same provider read against `clock`, a test's stand-in for time.
	#[cfg(test)]
	pub(crate) fn with_clock(
		provider: impl AwsCredentialProvider,
		clock: fn() -> Timestamp,
	) -> Self {
		Self(Arc::new(SourceInner {
			provider: Box::new(provider),
			cache: default(),
			clock,
		}))
	}

	/// What the provider is, ie `the environment`.
	pub fn describe(&self) -> SmolStr { self.0.provider.describe() }

	/// The current credential, from the cache while it is fresh.
	pub async fn credentials(&self) -> Result<AwsCredentials> {
		let mut cache = self.0.cache.lock().await;
		let now = (self.0.clock)();
		if let Some(credentials) = cache.as_ref()
			&& credentials
				.expiry()
				.is_none_or(|expiry| now + Self::REFRESH_WINDOW < expiry)
		{
			return credentials.clone().xok();
		}
		match self.0.provider.load().await? {
			Some(credentials) => {
				*cache = Some(credentials.clone());
				credentials.xok()
			}
			None => {
				*cache = None;
				bevybail!("no AWS credentials in {}", self.describe())
			}
		}
	}

	/// Drop the cached credential after a refusal said it lapsed, answering
	/// whether the next [`credentials`](Self::credentials) may differ.
	pub async fn invalidate(&self) -> bool {
		*self.0.cache.lock().await = None;
		self.0.provider.refreshable()
	}
}

impl From<AwsCredentials> for AwsCredentialSource {
	fn from(credentials: AwsCredentials) -> Self {
		Self::new(FixedCredentials(credentials))
	}
}

/// A fixed credential, never refreshed.
struct FixedCredentials(AwsCredentials);

impl AwsCredentialProvider for FixedCredentials {
	fn describe(&self) -> SmolStr {
		format!("the credential `{}`", self.0.access_key_id()).into()
	}
	fn load(&self) -> SendBoxedFuture<Result<Option<AwsCredentials>>> {
		let credentials = self.0.clone();
		Box::pin(async move { Ok(Some(credentials)) })
	}
	fn refreshable(&self) -> bool { false }
}

/// The process's clock as AWS sees it: the wall clock plus the offset a
/// skew refusal corrected it by, so every later signature in the process
/// carries the server's time rather than repeating the refusal.
pub struct AwsClock;

impl AwsClock {
	/// The corrected time.
	pub fn now() -> Timestamp {
		Timestamp::from_millis(
			Timestamp::now().millis()
				+ Self::offset().load(core::sync::atomic::Ordering::Relaxed),
		)
	}

	/// Correct the clock to `server`, the time a response said it is.
	pub fn correct(server: Timestamp) {
		Self::offset().store(
			server.millis() - Timestamp::now().millis(),
			core::sync::atomic::Ordering::Relaxed,
		);
	}

	fn offset() -> &'static core::sync::atomic::AtomicI64 {
		static OFFSET: core::sync::atomic::AtomicI64 =
			core::sync::atomic::AtomicI64::new(0);
		&OFFSET
	}
}

/// The launch's own credential, the first link that has one:
///
/// 1. the environment (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`,
///    `AWS_SESSION_TOKEN`): a box's unit loads its pair through an
///    `EnvironmentFile`, a deployer's launch the `env_var` records of its
///    secrets document, a Worker its secret bindings;
/// 2. the shared credentials file (`AWS_SHARED_CREDENTIALS_FILE`, else
///    `~/.aws/credentials`, profile `AWS_PROFILE` else `default`): the
///    bootstrap path, for a machine with no secrets document yet;
/// 3. the ECS container endpoint (a Fargate task role);
/// 4. EC2 instance metadata (IMDSv2, an instance role).
///
/// A credential a client is given explicitly is not a link: it wins before
/// the chain is consulted. No `~/.aws/config` profile is read (no
/// `source_profile` chains, `sso_session` sections or `credential_process`):
/// beet machines carry no ambient AWS config by design. The shared file, ECS
/// and IMDS need a native target; a Worker's chain is the environment alone.
/// An error names every link tried and what each found.
pub struct AwsDefaultChain;

impl AwsDefaultChain {
	/// The links, in order.
	#[cfg_attr(target_arch = "wasm32", allow(unused_mut))]
	fn links() -> Vec<Box<dyn AwsCredentialProvider>> {
		let mut links: Vec<Box<dyn AwsCredentialProvider>> =
			vec![Box::new(EnvCredentials)];
		#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
		links.push(Box::new(SharedFileCredentials::from_env()));
		#[cfg(not(target_arch = "wasm32"))]
		links.push(Box::new(EcsCredentials::from_env()));
		#[cfg(not(target_arch = "wasm32"))]
		links.push(Box::new(ImdsCredentials::from_env()));
		links
	}
}

impl AwsDefaultChain {
	/// The first of `links` answering a credential, else an error naming
	/// each and what it found.
	async fn first_of(
		links: Vec<Box<dyn AwsCredentialProvider>>,
	) -> Result<Option<AwsCredentials>> {
		let mut tried = Vec::new();
		for link in links {
			match link.load().await {
				Ok(Some(credentials)) => return Ok(Some(credentials)),
				Ok(None) => tried.push(format!("{}: none", link.describe())),
				Err(err) => tried.push(format!(
					"{}: {}",
					link.describe(),
					err.to_string().lines().next().unwrap_or_default()
				)),
			}
		}
		bevybail!(
			"no AWS credentials, tried{}",
			tried
				.iter()
				.enumerate()
				.map(|(index, tried)| format!("\n  {}. {tried}", index + 1))
				.collect::<String>()
		)
	}
}

impl AwsCredentialProvider for AwsDefaultChain {
	fn describe(&self) -> SmolStr { "the default chain".into() }

	fn load(&self) -> SendBoxedFuture<Result<Option<AwsCredentials>>> {
		Box::pin(Self::first_of(Self::links()))
	}
}

/// The environment's pair, see [`AwsCredentials::from_env`].
pub struct EnvCredentials;

impl AwsCredentialProvider for EnvCredentials {
	fn describe(&self) -> SmolStr {
		format!(
			"the environment (`{}`, `{}`)",
			AwsCredentials::ACCESS_KEY_ID,
			AwsCredentials::SECRET_ACCESS_KEY
		)
		.into()
	}
	fn load(&self) -> SendBoxedFuture<Result<Option<AwsCredentials>>> {
		Box::pin(async { Ok(AwsCredentials::from_env()) })
	}
}

/// A profile of the shared credentials file, the bootstrap path before a
/// secrets document exists.
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
pub struct SharedFileCredentials {
	path: Option<std::path::PathBuf>,
	profile: SmolStr,
}

#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
impl SharedFileCredentials {
	/// `profile` of the file at `path`.
	pub fn new(
		path: impl Into<std::path::PathBuf>,
		profile: impl Into<SmolStr>,
	) -> Self {
		Self {
			path: Some(path.into()),
			profile: profile.into(),
		}
	}

	/// The launch's file and profile: `AWS_SHARED_CREDENTIALS_FILE` else
	/// `~/.aws/credentials`, `AWS_PROFILE` else `default`.
	pub fn from_env() -> Self {
		Self {
			path: match env_ext::var("AWS_SHARED_CREDENTIALS_FILE") {
				Ok(path) => Some(std::path::PathBuf::from(path.as_str())),
				Err(_) => fs_ext::home_dir()
					.ok()
					.map(|home| home.join(".aws").join("credentials")),
			},
			profile: env_ext::var("AWS_PROFILE")
				.map(SmolStr::from)
				.unwrap_or_else(|_| "default".into()),
		}
	}
}

#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
impl AwsCredentialProvider for SharedFileCredentials {
	fn describe(&self) -> SmolStr {
		match &self.path {
			Some(path) => format!(
				"the shared credentials file (`{}`, profile `{}`)",
				path.display(),
				self.profile
			),
			None => "the shared credentials file (no home directory)".into(),
		}
		.into()
	}
	fn load(&self) -> SendBoxedFuture<Result<Option<AwsCredentials>>> {
		let text = self
			.path
			.as_ref()
			.and_then(|path| fs_ext::read_to_string(path).ok());
		let profile = self.profile.clone();
		Box::pin(async move {
			text.and_then(|text| AwsCredentials::parse_profile(&text, &profile))
				.xok()
		})
	}
}

/// The ECS container credential endpoint, a Fargate task's role: the
/// relative uri at `169.254.170.2`, or a full uri with its authorization
/// token (inline or in a file).
#[cfg(not(target_arch = "wasm32"))]
pub struct EcsCredentials {
	endpoint: Option<EcsEndpoint>,
}

#[cfg(not(target_arch = "wasm32"))]
struct EcsEndpoint {
	url: String,
	/// The `Authorization` header value, or the file holding it.
	token: Option<EcsToken>,
}

#[cfg(not(target_arch = "wasm32"))]
enum EcsToken {
	Inline(SmolStr),
	File(std::path::PathBuf),
}

#[cfg(not(target_arch = "wasm32"))]
impl EcsCredentials {
	/// The host the relative uri is served from.
	pub const HOST: &'static str = "169.254.170.2";

	/// The endpoint the launch's variables name, none when it names none.
	pub fn from_env() -> Self {
		Self::from_vars(|name| env_ext::var(name).ok().map(String::from))
	}

	/// The endpoint `var` names, the variables read through it.
	fn from_vars(var: impl Fn(&str) -> Option<String>) -> Self {
		let endpoint = match (
			var("AWS_CONTAINER_CREDENTIALS_RELATIVE_URI"),
			var("AWS_CONTAINER_CREDENTIALS_FULL_URI"),
		) {
			(Some(relative), _) => Some(EcsEndpoint {
				url: format!("http://{}{relative}", Self::HOST),
				token: None,
			}),
			(None, Some(full)) => Some(EcsEndpoint {
				url: full,
				token: match (
					var("AWS_CONTAINER_AUTHORIZATION_TOKEN_FILE"),
					var("AWS_CONTAINER_AUTHORIZATION_TOKEN"),
				) {
					(Some(file), _) => Some(EcsToken::File(file.into())),
					(None, Some(token)) => Some(EcsToken::Inline(token.into())),
					(None, None) => None,
				},
			}),
			(None, None) => None,
		};
		Self { endpoint }
	}

	/// Whether a full uri may be trusted with a credential: https anywhere,
	/// http only to loopback or the container agent's own addresses.
	fn is_trusted(url: &str) -> bool {
		let Some(rest) = url.strip_prefix("http://") else {
			return url.starts_with("https://");
		};
		let host = rest.split(['/', '?']).next().unwrap_or_default();
		let host = match host.starts_with('[') {
			true => host
				.split(']')
				.next()
				.unwrap_or_default()
				.trim_start_matches('['),
			false => host.split(':').next().unwrap_or_default(),
		};
		match host.parse::<std::net::IpAddr>() {
			Ok(ip) => {
				ip.is_loopback()
					|| ["169.254.170.2", "169.254.170.23", "fd00:ec2::23"]
						.iter()
						.any(|agent| agent.parse() == Ok(ip))
			}
			Err(_) => host == "localhost",
		}
	}
}

#[cfg(not(target_arch = "wasm32"))]
impl AwsCredentialProvider for EcsCredentials {
	fn describe(&self) -> SmolStr {
		match &self.endpoint {
			Some(endpoint) => {
				format!("the ECS container endpoint (`{}`)", endpoint.url)
			}
			None => "the ECS container endpoint (no \
				`AWS_CONTAINER_CREDENTIALS_*_URI`)"
				.into(),
		}
		.into()
	}

	fn load(&self) -> SendBoxedFuture<Result<Option<AwsCredentials>>> {
		let Some(endpoint) = &self.endpoint else {
			return Box::pin(async { Ok(None) });
		};
		let url = endpoint.url.clone();
		let token = match &endpoint.token {
			Some(EcsToken::Inline(token)) => Some(Ok(token.to_string())),
			Some(EcsToken::File(path)) => Some(
				fs_ext::read_to_string(path)
					.map(|token| token.trim().to_string())
					.map_err(BevyError::from),
			),
			None => None,
		};
		Box::pin(async move {
			if !Self::is_trusted(&url) {
				bevybail!(
					"refusing to send a container credential request to \
					`{url}`: plain http only reaches loopback or the \
					container agent"
				);
			}
			let request = match token {
				Some(token) => Request::get(url).with_auth_raw(&token?),
				None => Request::get(url),
			};
			let response = request.send().await?;
			if !response.status().is_success() {
				bevybail!("answered {}", response.status());
			}
			AwsCredentials::from_metadata_json(&serde_json::from_str(
				&response.text().await?,
			)?)?
			.xsome()
			.xok()
		})
	}
}

/// EC2 instance metadata, IMDSv2: a session token from a `PUT`, then the
/// instance role's credential. Off EC2 the endpoint does not answer, so the
/// token request is given one second before the link reports none.
#[cfg(not(target_arch = "wasm32"))]
pub struct ImdsCredentials {
	/// The service root, `None` when `AWS_EC2_METADATA_DISABLED` is set.
	endpoint: Option<SmolStr>,
}

#[cfg(not(target_arch = "wasm32"))]
impl ImdsCredentials {
	/// The service root on every instance.
	pub const ENDPOINT: &'static str = "http://169.254.169.254";
	/// How long the token request may take before the link gives up.
	pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(1);

	/// The endpoint the launch names: `AWS_EC2_METADATA_SERVICE_ENDPOINT`
	/// else [`ENDPOINT`](Self::ENDPOINT), off when
	/// `AWS_EC2_METADATA_DISABLED` is `true`.
	pub fn from_env() -> Self {
		let disabled = env_ext::var("AWS_EC2_METADATA_DISABLED")
			.is_ok_and(|value| value.eq_ignore_ascii_case("true"));
		Self {
			endpoint: (!disabled).then(|| {
				env_ext::var("AWS_EC2_METADATA_SERVICE_ENDPOINT")
					.map(|endpoint| {
						SmolStr::new(endpoint.trim_end_matches('/'))
					})
					.unwrap_or_else(|_| Self::ENDPOINT.into())
			}),
		}
	}

	/// The service at `endpoint`, a test's stand-in.
	pub fn new(endpoint: impl AsRef<str>) -> Self {
		Self {
			endpoint: Some(endpoint.as_ref().trim_end_matches('/').into()),
		}
	}

	async fn load_from(endpoint: SmolStr) -> Result<Option<AwsCredentials>> {
		let token_request =
			Request::put(format!("{endpoint}/latest/api/token"))
				.with_header_raw(
					"x-aws-ec2-metadata-token-ttl-seconds",
					"21600",
				)
				.send();
		let Ok(token) =
			async_ext::timeout(Self::CONNECT_TIMEOUT, token_request).await
		else {
			// nothing answered: not an instance
			return Ok(None);
		};
		let token = token?;
		if !token.status().is_success() {
			bevybail!("the token request answered {}", token.status());
		}
		let token = token.text().await?;
		let get = |path: &str| {
			Request::get(format!(
				"{endpoint}/latest/meta-data/iam/security-credentials/{path}"
			))
			.with_header_raw("x-aws-ec2-metadata-token", &token)
			.send()
		};
		let roles = get("").await?;
		match roles.status().as_u16() {
			// an instance without a role
			404 => return Ok(None),
			status if status >= 300 => {
				bevybail!("the role listing answered {status}")
			}
			_ => {}
		}
		let roles = roles.text().await?;
		let Some(role) =
			roles.lines().map(str::trim).find(|line| !line.is_empty())
		else {
			return Ok(None);
		};
		let credentials = get(role).await?;
		if !credentials.status().is_success() {
			bevybail!("the role `{role}` answered {}", credentials.status());
		}
		AwsCredentials::from_metadata_json(&serde_json::from_str(
			&credentials.text().await?,
		)?)?
		.xsome()
		.xok()
	}
}

#[cfg(not(target_arch = "wasm32"))]
impl AwsCredentialProvider for ImdsCredentials {
	fn describe(&self) -> SmolStr {
		match &self.endpoint {
			Some(endpoint) => format!("EC2 instance metadata (`{endpoint}`)"),
			None => {
				"EC2 instance metadata (`AWS_EC2_METADATA_DISABLED`)".into()
			}
		}
		.into()
	}

	fn load(&self) -> SendBoxedFuture<Result<Option<AwsCredentials>>> {
		let endpoint = self.endpoint.clone();
		Box::pin(async move {
			match endpoint {
				Some(endpoint) => Self::load_from(endpoint).await,
				None => Ok(None),
			}
		})
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// A provider answering a fresh credential per load, expiring an hour
	/// after the test clock's zero.
	struct Counting(Arc<core::sync::atomic::AtomicU32>);

	impl AwsCredentialProvider for Counting {
		fn describe(&self) -> SmolStr { "a counter".into() }
		fn load(&self) -> SendBoxedFuture<Result<Option<AwsCredentials>>> {
			let count =
				self.0.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
			Box::pin(async move {
				AwsCredentials::new(format!("KEY{count}"), "secret")
					.with_expiry(Timestamp::from_secs(3600))
					.xsome()
					.xok()
			})
		}
	}

	static TEST_NOW: core::sync::atomic::AtomicI64 =
		core::sync::atomic::AtomicI64::new(0);
	fn test_clock() -> Timestamp {
		Timestamp::from_millis(
			TEST_NOW.load(core::sync::atomic::Ordering::SeqCst),
		)
	}

	/// A credential is cached until five minutes before it lapses, then
	/// loaded afresh; an invalidation drops it.
	#[beet_core::test]
	async fn refreshes_before_expiry() {
		let loads = Arc::new(core::sync::atomic::AtomicU32::new(0));
		let source = AwsCredentialSource::with_clock(
			Counting(loads.clone()),
			test_clock,
		);
		let key = async || {
			source
				.credentials()
				.await
				.unwrap()
				.access_key_id()
				.to_string()
		};
		key().await.xpect_eq("KEY0");
		// fifty four minutes in, still fresh
		TEST_NOW.store(54 * 60 * 1000, core::sync::atomic::Ordering::SeqCst);
		key().await.xpect_eq("KEY0");
		// inside the refresh window
		TEST_NOW.store(56 * 60 * 1000, core::sync::atomic::Ordering::SeqCst);
		key().await.xpect_eq("KEY1");
		source.invalidate().await.xpect_true();
		key().await.xpect_eq("KEY2");
	}

	/// A provider that has nothing is never cached as an answer.
	#[beet_core::test]
	async fn never_caches_a_failure() {
		struct Empty;
		impl AwsCredentialProvider for Empty {
			fn describe(&self) -> SmolStr { "nowhere".into() }
			fn load(&self) -> SendBoxedFuture<Result<Option<AwsCredentials>>> {
				Box::pin(async { Ok(None) })
			}
		}
		AwsCredentialSource::new(Empty)
			.credentials()
			.await
			.unwrap_err()
			.to_string()
			.xpect_starts_with("no AWS credentials in nowhere");
	}

	/// In a Worker the chain is the environment, which the entry seeds from
	/// the Worker's secret bindings exactly as this does (`seed_env`).
	#[cfg(target_arch = "wasm32")]
	#[beet_core::test]
	async fn a_worker_reads_its_bindings() {
		for (key, value) in [
			(AwsCredentials::ACCESS_KEY_ID, "AKIDWORKER"),
			(AwsCredentials::SECRET_ACCESS_KEY, "binding-secret"),
		] {
			unsafe { env_ext::set_var(key, value) }.unwrap();
		}
		AwsCredentialSource::new(AwsDefaultChain)
			.credentials()
			.await
			.unwrap()
			.access_key_id()
			.xpect_eq("AKIDWORKER");
	}

	/// A fixed credential cannot be refreshed, so a refusal is not retried.
	#[beet_core::test]
	async fn fixed_is_not_refreshable() {
		AwsCredentialSource::from(AwsCredentials::new("AKID", "secret"))
			.invalidate()
			.await
			.xpect_false();
	}

	/// The container endpoint the variables name: the relative uri at the
	/// agent's address, else the full uri with its token, the file winning.
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test]
	fn reads_the_container_variables() {
		let vars = |pairs: &'static [(&'static str, &'static str)]| {
			move |name: &str| {
				pairs
					.iter()
					.find(|(key, _)| *key == name)
					.map(|(_, value)| value.to_string())
			}
		};
		EcsCredentials::from_vars(vars(&[(
			"AWS_CONTAINER_CREDENTIALS_RELATIVE_URI",
			"/v2/credentials/abc",
		)]))
		.describe()
		.xpect_contains("http://169.254.170.2/v2/credentials/abc");
		let full = EcsCredentials::from_vars(vars(&[
			(
				"AWS_CONTAINER_CREDENTIALS_FULL_URI",
				"http://127.0.0.1:9/creds",
			),
			("AWS_CONTAINER_AUTHORIZATION_TOKEN", "inline"),
			("AWS_CONTAINER_AUTHORIZATION_TOKEN_FILE", "/run/token"),
		]));
		matches!(full.endpoint.unwrap().token, Some(EcsToken::File(_)))
			.xpect_true();
		EcsCredentials::from_vars(vars(&[]))
			.endpoint
			.is_none()
			.xpect_true();
	}

	/// Plain http reaches only loopback and the container agent.
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test]
	fn trusts_only_local_http() {
		EcsCredentials::is_trusted("http://127.0.0.1:8080/creds").xpect_true();
		EcsCredentials::is_trusted("http://169.254.170.23/v1").xpect_true();
		EcsCredentials::is_trusted("http://[fd00:ec2::23]/v1").xpect_true();
		EcsCredentials::is_trusted("https://creds.example.com/").xpect_true();
		EcsCredentials::is_trusted("http://evil.example.com/").xpect_false();
	}
}

/// The metadata links against a local server standing in for the endpoint.
#[cfg(all(
	test,
	feature = "server",
	feature = "ureq",
	feature = "fs",
	not(target_arch = "wasm32")
))]
mod endpoint_test {
	use super::*;

	/// A metadata answer for `key`, lapsing in an hour.
	fn metadata(key: &str) -> String {
		serde_json::json!({
			"AccessKeyId": key,
			"SecretAccessKey": "secret",
			"Token": "token",
			"Expiration": (Timestamp::now() + Duration::from_secs(3600)).format_iso8601_secs(),
		})
		.to_string()
	}

	/// The full uri path, its authorization token read from a file.
	#[beet_core::test]
	async fn reads_the_container_endpoint() {
		let url = HttpServer::spawn_test(exchange_ext::handler(|cx| {
			let request = cx.take();
			match (
				request.url().path_string().as_str(),
				request.headers.first_raw("authorization"),
			) {
				("/creds", Some("tok3n")) => {
					Response::ok_text(metadata("ECSKEY"))
				}
				_ => Response::from_status(StatusCode::FORBIDDEN),
			}
		}))
		.await;
		let dir = TempDir::new().unwrap();
		let token = dir.join("token");
		fs_ext::write(&token, "tok3n\n").unwrap();
		let credentials = EcsCredentials {
			endpoint: Some(EcsEndpoint {
				url: format!("{url}/creds"),
				token: Some(EcsToken::File(token.into())),
			}),
		}
		.load()
		.await
		.unwrap()
		.unwrap();
		credentials.access_key_id().xpect_eq("ECSKEY");
		credentials.session_token().xpect_eq(Some("token"));
		credentials.expiry().is_some().xpect_true();
	}

	/// IMDSv2: the token from the `PUT`, then the role's credential with it.
	#[beet_core::test]
	async fn reads_instance_metadata() {
		let url = HttpServer::spawn_test(exchange_ext::handler(|cx| {
			let request = cx.take();
			let path = request.url().path_string();
			let token = request.headers.first_raw("x-aws-ec2-metadata-token");
			match (request.method(), path.as_str(), token) {
				(HttpMethod::Put, "/latest/api/token", _) => {
					Response::ok_text("imds-token")
				}
				(
					HttpMethod::Get,
					"/latest/meta-data/iam/security-credentials/",
					Some("imds-token"),
				) => Response::ok_text("box-role\n"),
				(
					HttpMethod::Get,
					"/latest/meta-data/iam/security-credentials/box-role",
					Some("imds-token"),
				) => Response::ok_text(metadata("IMDSKEY")),
				_ => Response::from_status(StatusCode::UNAUTHORIZED),
			}
		}))
		.await;
		ImdsCredentials::new(url)
			.load()
			.await
			.unwrap()
			.unwrap()
			.access_key_id()
			.xpect_eq("IMDSKEY");
	}

	/// Off EC2 nothing answers, and the link reports none within its second.
	#[beet_core::test]
	async fn imds_off_ec2_is_none() {
		let start = Instant::now();
		// the documentation range never routes
		ImdsCredentials::new("http://192.0.2.1")
			.load()
			.await
			.unwrap()
			.xpect_none();
		start.elapsed().xpect_less_than(Duration::from_secs(3));
	}

	/// The chain's error names every link it tried and what each found.
	#[beet_core::test]
	async fn names_every_link() {
		AwsDefaultChain::first_of(vec![
			Box::new(SharedFileCredentials::new(
				"/nonexistent/credentials",
				"default",
			)),
			Box::new(EcsCredentials::from_vars(|_| None)),
			Box::new(EcsCredentials {
				endpoint: Some(EcsEndpoint {
					url: "http://evil.example.com/creds".into(),
					token: None,
				}),
			}),
			Box::new(ImdsCredentials { endpoint: None }),
		])
		.await
		.unwrap_err()
		.to_string()
		.lines()
		.take(5)
		.collect::<Vec<_>>()
		.join("\n")
		.xpect_snapshot();
	}
}
