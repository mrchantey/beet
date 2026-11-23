//! An IAM Identity Center sign in: the human path into every member account.

use crate::prelude::*;
use beet_core::prelude::*;
use serde_json::Value;
use serde_json::json;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

/// IAM Identity Center's sign in for one instance: its region and start
/// url, the OAuth service (`oidc.{region}.amazonaws.com`) and the access
/// portal (`portal.sso.{region}.amazonaws.com`).
///
/// A login registers this machine as a public client once (the registration
/// cached on tmpfs until it lapses), then signs the person in through the
/// browser, authorization code with PKCE over a [`LoopbackRedirect`], or
/// with no browser through the device code grant, showing a url and a code
/// to approve from another machine. Either answers an
/// [`IdentityCenterSession`].
///
/// ## Session rules
///
/// Read by default, write behind a human act, as with `beet admin`:
///
/// - a live session reaches every account the person is assigned as
///   [`READ_SET`](IdentityCenterSession::READ_SET), refreshing its token as
///   needed, for as long as Identity Center keeps the session;
/// - [`ADMIN_SET`](IdentityCenterSession::ADMIN_SET) credentials are minted
///   only inside [`ADMIN_WINDOW`](IdentityCenterSession::ADMIN_WINDOW) after
///   an interactive sign in, read from the token set's sign in instant, which
///   a refresh never moves. An agent in the same terminal gets admin only
///   inside the hour a human just opened.
#[derive(Clone)]
pub struct IdentityCenterClient {
	region: SmolStr,
	start_url: SmolStr,
	oidc: AwsClient,
	portal: AwsClient,
	present: SsoPresenter,
	clock: fn() -> Timestamp,
}

/// How a login shows the person where to go: the browser's authorization
/// url, or the device grant's url and code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SsoPrompt {
	/// Open this url and sign in.
	Browser(String),
	/// Open this url on any machine and confirm this code.
	Device {
		/// The verification url, the code already in it.
		url: String,
		/// The code the page shows, to compare.
		code: String,
	},
}

type SsoPresenter =
	Arc<dyn 'static + Send + Sync + Fn(SsoPrompt) -> SendBoxedFuture<Result>>;

impl core::fmt::Debug for IdentityCenterClient {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("IdentityCenterClient")
			.field("region", &self.region)
			.field("start_url", &self.start_url)
			.finish_non_exhaustive()
	}
}

/// This machine's registration as a public client, cached until it lapses.
#[derive(Clone, Serialize, Deserialize)]
struct SsoClient {
	client_id: SmolStr,
	client_secret: SmolStr,
	expires_at: Timestamp,
	/// The start url it was registered for: another instance registers anew.
	start_url: SmolStr,
}

impl IdentityCenterClient {
	/// The scope every login asks for, the portal's account access.
	pub const SCOPE: &'static str = "sso:account:access";
	/// The redirect every registration names; a login listens on its own
	/// ephemeral port, which the loopback rules let vary.
	const REGISTERED_REDIRECT: &'static str = "http://127.0.0.1/oauth/callback";
	/// The device code grant type.
	const DEVICE_GRANT: &'static str =
		"urn:ietf:params:oauth:grant-type:device_code";

	/// The instance at `start_url` in `region`.
	pub fn new(
		region: impl Into<SmolStr>,
		start_url: impl Into<SmolStr>,
	) -> Self {
		let region = region.into();
		Self {
			oidc: AwsClient::new(AwsService::SSO_OIDC, region.clone())
				.anonymous(),
			portal: AwsClient::new(AwsService::SSO_PORTAL, region.clone())
				.anonymous(),
			region,
			start_url: start_url.into(),
			present: Arc::new(|prompt| Box::pin(Self::present(prompt))),
			clock: Timestamp::now,
		}
	}

	/// Reach the OAuth service and the portal at `oidc` and `portal`, a
	/// test's stand-ins.
	pub fn with_endpoints(mut self, oidc: &str, portal: &str) -> Self {
		self.oidc = self.oidc.with_endpoint(oidc);
		self.portal = self.portal.with_endpoint(portal);
		self
	}

	/// Show login prompts through `present` rather than the operator.
	pub fn with_presenter(
		mut self,
		present: impl 'static
		+ Send
		+ Sync
		+ Fn(SsoPrompt) -> SendBoxedFuture<Result>,
	) -> Self {
		self.present = Arc::new(present);
		self
	}

	/// Read time from `clock`, a test's stand-in.
	pub fn with_clock(mut self, clock: fn() -> Timestamp) -> Self {
		self.clock = clock;
		self
	}

	/// The instance's region.
	pub fn region(&self) -> &str { &self.region }

	/// The instance's start url.
	pub fn start_url(&self) -> &str { &self.start_url }

	/// The operator's view of a prompt: the url through [`OAuthBrowser`], or
	/// the device code as a notice.
	async fn present(prompt: SsoPrompt) -> Result {
		match prompt {
			SsoPrompt::Browser(url) => {
				OAuthBrowser::open("an Identity Center sign in", &url).await
			}
			SsoPrompt::Device { url, code } => OperatorPrompt::ask(
				OperatorAsk::notice(
					"an Identity Center sign in",
					format!(
						"open this url on any machine and confirm the code \
						{code}:\n\n  {url}\n"
					),
				)
				.with_instead("run the login in a terminal"),
			)
			.await
			.map(|_| ()),
		}
	}

	/// Sign in through the browser: authorization code with PKCE, the
	/// redirect received in `world`.
	#[cfg(feature = "server")]
	pub async fn login(
		&self,
		dir: &Path,
		world: &AsyncWorld,
	) -> Result<IdentityCenterSession> {
		let client = self.client(dir).await?;
		let pkce = Pkce::new()?;
		let state = OAuthState::new()?;
		let receiver = LoopbackRedirect::open(world).await?;
		let redirect_uri = receiver.redirect_uri();
		let url = self.authorize_url(&client, &redirect_uri, &pkce, &state);
		(self.present)(SsoPrompt::Browser(url)).await?;
		let code = receiver.wait(&state).await?;
		let answer = self
			.create_token(
				&client,
				json!({
					"grantType": "authorization_code",
					"code": code.as_str(),
					"redirectUri": redirect_uri,
					"codeVerifier": pkce.verifier(),
				}),
			)
			.await?;
		self.session(client, &answer, None)
	}

	/// Sign in with no browser here: the device code grant, its url and code
	/// shown to approve from any machine.
	pub async fn login_device(
		&self,
		dir: &Path,
	) -> Result<IdentityCenterSession> {
		self.login_device_polled(dir, DeviceGrant::new).await
	}

	/// [`login_device`](Self::login_device) polled as `grant` builds it.
	async fn login_device_polled(
		&self,
		dir: &Path,
		grant: impl FnOnce(Duration, Duration) -> DeviceGrant,
	) -> Result<IdentityCenterSession> {
		let client = self.client(dir).await?;
		let start = self
			.oidc
			.send_json(
				AwsRequest::post(
					"StartDeviceAuthorization",
					"/device_authorization",
				)
				.with_json(&json!({
					"clientId": client.client_id.as_str(),
					"clientSecret": client.client_secret.as_str(),
					"startUrl": self.start_url.as_str(),
				}))?
				.with_secret(client.client_secret.as_str()),
			)
			.await?;
		let field = |name: &str| {
			start[name]
				.as_str()
				.map(ToString::to_string)
				.ok_or_else(|| {
					bevyhow!("the device authorization answered no `{name}`")
				})
		};
		let device_code = field("deviceCode")?;
		(self.present)(SsoPrompt::Device {
			url: start["verificationUriComplete"]
				.as_str()
				.map(ToString::to_string)
				.map_or_else(|| field("verificationUri"), Ok)?,
			code: field("userCode")?,
		})
		.await?;
		let secs = |name: &str, default: u64| {
			Duration::from_secs(start[name].as_u64().unwrap_or(default))
		};
		let answer = grant(secs("interval", 5), secs("expiresIn", 600))
			.poll(|| {
				let device_code = device_code.clone();
				let client = client.clone();
				async move {
					match self
						.create_token(
							&client,
							json!({
								"grantType": Self::DEVICE_GRANT,
								"deviceCode": device_code,
							}),
						)
						.await
					{
						Ok(answer) => DevicePoll::Granted(answer).xok(),
						Err(err)
							if AwsError::is(&err, &[
								"AuthorizationPendingException",
							]) =>
						{
							DevicePoll::Pending.xok()
						}
						Err(err)
							if AwsError::is(&err, &["SlowDownException"]) =>
						{
							DevicePoll::SlowDown.xok()
						}
						Err(err) => Err(err),
					}
				}
			})
			.await?;
		self.session(client, &answer, None)
	}

	/// The authorization url a browser login opens.
	#[cfg(feature = "server")]
	fn authorize_url(
		&self,
		client: &SsoClient,
		redirect_uri: &str,
		pkce: &Pkce,
		state: &OAuthState,
	) -> String {
		let base = self.oidc.endpoint_url().trim_end_matches('/').to_string();
		let query = [
			("response_type", "code"),
			("client_id", client.client_id.as_str()),
			("redirect_uri", redirect_uri),
			("state", state.as_str()),
			("code_challenge_method", Pkce::METHOD),
			("code_challenge", pkce.challenge()),
			("scopes", Self::SCOPE),
		]
		.iter()
		.map(|(key, value)| format!("{key}={}", uri_encode(value, "")))
		.collect::<Vec<_>>()
		.join("&");
		format!("{base}/authorize?{query}")
	}

	/// `CreateToken` with `grant`, as `client`.
	async fn create_token(
		&self,
		client: &SsoClient,
		mut grant: Value,
	) -> Result<Value> {
		grant["clientId"] = json!(client.client_id.as_str());
		grant["clientSecret"] = json!(client.client_secret.as_str());
		self.oidc
			.send_json(
				AwsRequest::post("CreateToken", "/token")
					.with_json(&grant)?
					.with_secret(client.client_secret.as_str()),
			)
			.await
	}

	/// The session a token answer opens, `previous` the session it refreshes.
	fn session(
		&self,
		client: SsoClient,
		answer: &Value,
		previous: Option<&OAuthTokens>,
	) -> Result<IdentityCenterSession> {
		let Some(access_token) = answer["accessToken"].as_str() else {
			bevybail!("CreateToken answered no access token");
		};
		let refresh_token = answer["refreshToken"].as_str().map(SmolStr::new);
		let expires_in =
			Duration::from_secs(answer["expiresIn"].as_u64().unwrap_or(3600));
		let now = (self.clock)();
		IdentityCenterSession {
			region: self.region.clone(),
			start_url: self.start_url.clone(),
			tokens: match previous {
				Some(previous) => previous.refreshed(
					access_token,
					refresh_token,
					expires_in,
					now,
				),
				None => OAuthTokens::signed_in(
					access_token,
					refresh_token,
					expires_in,
					now,
				),
			},
			client,
		}
		.xok()
	}

	/// The registration cached in `dir`, else a new one written there.
	async fn client(&self, dir: &Path) -> Result<SsoClient> {
		let path = dir.join(IdentityCenterSession::CLIENT_FILE);
		if let Ok(text) = fs_ext::read_to_string(&path)
			&& let Ok(client) = serde_json::from_str::<SsoClient>(&text)
			&& client.start_url == self.start_url
			&& (self.clock)() + Duration::from_secs(3600) < client.expires_at
		{
			return client.xok();
		}
		let answer = self
			.oidc
			.send_json(
				AwsRequest::post("RegisterClient", "/client/register")
					.with_json(&json!({
						"clientName": "beet",
						"clientType": "public",
						"scopes": [Self::SCOPE],
						"redirectUris": [Self::REGISTERED_REDIRECT],
						"grantTypes": ["authorization_code", "refresh_token", Self::DEVICE_GRANT],
						"issuerUrl": self.start_url.as_str(),
					}))?,
			)
			.await?;
		let field = |name: &str| {
			answer[name]
				.as_str()
				.map(SmolStr::new)
				.ok_or_else(|| bevyhow!("RegisterClient answered no `{name}`"))
		};
		let client = SsoClient {
			client_id: field("clientId")?,
			client_secret: field("clientSecret")?,
			expires_at: Timestamp::from_secs(
				answer["clientSecretExpiresAt"].as_i64().unwrap_or_default(),
			),
			start_url: self.start_url.clone(),
		};
		fs_ext::create_dir_private(dir)?;
		fs_ext::write_private(&path, serde_json::to_string(&client)?)?;
		client.xok()
	}

	/// The session refreshed when its access token is not fresh, a lapsed
	/// refresh answering [`SsoState::Lapsed`]'s error.
	pub async fn fresh(
		&self,
		session: IdentityCenterSession,
	) -> Result<IdentityCenterSession> {
		if session.tokens.is_fresh((self.clock)()) {
			return session.xok();
		}
		let Some(refresh_token) = session.tokens.refresh_token() else {
			bevybail!(
				"the Identity Center session has lapsed: `beet sso login` opens another"
			);
		};
		let answer = self
			.create_token(
				&session.client,
				json!({
					"grantType": "refresh_token",
					"refreshToken": refresh_token,
				}),
			)
			.await
			.map_err(|err| {
				match AwsError::is(&err, &[
					"InvalidGrantException",
					"ExpiredTokenException",
				]) {
					true => bevyhow!(
						"the Identity Center session has lapsed: `beet sso login` opens another"
					),
					false => err,
				}
			})?;
		self.session(session.client.clone(), &answer, Some(&session.tokens))
	}

	/// A portal request as `session`.
	fn portal_request(
		&self,
		request: AwsRequest,
		session: &IdentityCenterSession,
	) -> AwsRequest {
		request
			.with_header(
				"x-amz-sso_bearer_token",
				session.tokens.access_token(),
			)
			.with_secret(session.tokens.access_token())
	}

	/// Every account the session is assigned, by name and id.
	pub async fn accounts(
		&self,
		session: &IdentityCenterSession,
	) -> Result<Vec<SsoAccount>> {
		let mut accounts = Vec::new();
		let mut next: Option<String> = None;
		loop {
			let mut request =
				AwsRequest::get("ListAccounts", "/assignment/accounts");
			if let Some(token) = next.take() {
				request = request.with_query("next_token", token);
			}
			let page = self
				.portal
				.send_json(self.portal_request(request, session))
				.await?;
			accounts.extend(
				page["accountList"]
					.as_array()
					.into_iter()
					.flatten()
					.filter_map(|account| {
						SsoAccount {
							id: account["accountId"].as_str()?.into(),
							name: account["accountName"].as_str()?.into(),
						}
						.xsome()
					}),
			);
			match page["nextToken"].as_str() {
				Some(token) => next = Some(token.to_string()),
				None => break,
			}
		}
		accounts.xok()
	}

	/// The roles (permission set names) the session holds in `account`.
	pub async fn roles(
		&self,
		session: &IdentityCenterSession,
		account: &SsoAccount,
	) -> Result<Vec<SmolStr>> {
		let mut roles = Vec::new();
		let mut next: Option<String> = None;
		loop {
			let mut request =
				AwsRequest::get("ListAccountRoles", "/assignment/roles")
					.with_query("account_id", account.id.as_str());
			if let Some(token) = next.take() {
				request = request.with_query("next_token", token);
			}
			let page = self
				.portal
				.send_json(self.portal_request(request, session))
				.await?;
			roles.extend(
				page["roleList"]
					.as_array()
					.into_iter()
					.flatten()
					.filter_map(|role| role["roleName"].as_str())
					.map(SmolStr::new),
			);
			match page["nextToken"].as_str() {
				Some(token) => next = Some(token.to_string()),
				None => break,
			}
		}
		roles.xok()
	}

	/// The account named `name`, among those the session is assigned.
	pub async fn account(
		&self,
		session: &IdentityCenterSession,
		name: &str,
	) -> Result<SsoAccount> {
		self.accounts(session)
			.await?
			.into_iter()
			.find(|account| account.name == name)
			.ok_or_else(|| {
				bevyhow!(
					"the Identity Center session is assigned no account named \
					`{name}`: the account's converge assigns the operator group"
				)
			})
	}

	/// A temporary credential for `role` (a permission set's name) in
	/// `account`, the admin set refused outside its window.
	pub async fn role_credentials(
		&self,
		session: &IdentityCenterSession,
		account: &SsoAccount,
		role: &str,
	) -> Result<AwsCredentials> {
		session.check_role(role, (self.clock)())?;
		if !self
			.roles(session, account)
			.await?
			.iter()
			.any(|held| held == role)
		{
			bevybail!(
				"the Identity Center session holds no `{role}` in `{}`: the \
				account's converge assigns the operator group both sets",
				account.name
			);
		}
		let answer = self
			.portal
			.send_json(
				self.portal_request(
					AwsRequest::get(
						"GetRoleCredentials",
						"/federation/credentials",
					)
					.with_query("role_name", role)
					.with_query("account_id", account.id.as_str()),
					session,
				),
			)
			.await?;
		let credentials = &answer["roleCredentials"];
		let field = |name: &str| {
			credentials[name].as_str().map(SmolStr::new).ok_or_else(|| {
				bevyhow!("GetRoleCredentials answered no `{name}`")
			})
		};
		AwsCredentials::new(field("accessKeyId")?, field("secretAccessKey")?)
			.with_session_token(field("sessionToken")?)
			.with_expiry(Timestamp::from_millis(
				credentials["expiration"].as_i64().unwrap_or_default(),
			))
			.xok()
	}

	/// End the session at Identity Center.
	pub async fn logout(&self, session: &IdentityCenterSession) -> Result {
		self.portal
			.send(
				self.portal_request(
					AwsRequest::post("Logout", "/logout"),
					session,
				),
			)
			.await?;
		OK
	}
}

/// An account the session is assigned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SsoAccount {
	/// The account id.
	pub id: SmolStr,
	/// The account's name in the Organization.
	pub name: SmolStr,
}

/// A live Identity Center sign in: the token set, the client that holds it,
/// and the instance it belongs to. Kept on tmpfs
/// (`$XDG_RUNTIME_DIR/beet/sso-session`, owner only), beside
/// [`AdminSession`], and never anywhere that survives a reboot.
#[derive(Clone, Serialize, Deserialize)]
pub struct IdentityCenterSession {
	region: SmolStr,
	start_url: SmolStr,
	client: SsoClient,
	tokens: OAuthTokens,
}

impl core::fmt::Debug for IdentityCenterSession {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("IdentityCenterSession")
			.field("start_url", &self.start_url)
			.field("tokens", &self.tokens)
			.finish_non_exhaustive()
	}
}

/// What [`IdentityCenterSession::read`] found on tmpfs.
#[derive(Debug, Clone)]
pub enum SsoState {
	/// A session that may still be refreshed.
	Live(IdentityCenterSession),
	/// One whose access token lapsed with no refresh token, its file just
	/// removed: distinguished from none so a caller can say why access
	/// changed under it.
	Lapsed,
	/// No session, the normal state.
	None,
}

impl IdentityCenterSession {
	/// The permission set every assigned account is reached by freely.
	pub const READ_SET: &'static str = "beet-read";
	/// The permission set minted only inside the admin window.
	pub const ADMIN_SET: &'static str = "beet-admin";
	/// How long after an interactive sign in the admin set may be minted.
	pub const ADMIN_WINDOW: Duration = Duration::from_secs(3600);
	/// The session file's name in the tmpfs directory.
	const SESSION_FILE: &'static str = "sso-session";
	/// The client registration's.
	const CLIENT_FILE: &'static str = "sso-client";

	/// The tmpfs directory sessions live in, [`AdminSession::dir`].
	pub fn dir() -> Result<PathBuf> { AdminSession::dir() }

	/// The instance this session signs in to.
	pub fn identity_center(&self) -> IdentityCenterClient {
		IdentityCenterClient::new(self.region.clone(), self.start_url.clone())
	}

	/// The instance's start url.
	pub fn start_url(&self) -> &str { &self.start_url }

	/// The token set.
	pub fn tokens(&self) -> &OAuthTokens { &self.tokens }

	/// Whether `now` is inside the admin window of the interactive sign in.
	pub fn admin_window_open(&self, now: Timestamp) -> bool {
		now.millis() - self.tokens.signed_in_at().millis()
			< Self::ADMIN_WINDOW.as_millis() as i64
	}

	/// Refuse `role` at `now` when it is the admin set outside its window.
	fn check_role(&self, role: &str, now: Timestamp) -> Result {
		if role == Self::ADMIN_SET && !self.admin_window_open(now) {
			bevybail!(
				"`{}` is minted only inside the hour after an interactive sign \
				in, and this one was at {}: `beet sso login` opens another \
				hour",
				Self::ADMIN_SET,
				self.tokens.signed_in_at().format_iso8601_secs()
			);
		}
		OK
	}

	/// The role a write reaches for at `now`: the admin set inside the
	/// window, else the read set.
	pub fn role_at(&self, now: Timestamp) -> &'static str {
		match self.admin_window_open(now) {
			true => Self::ADMIN_SET,
			false => Self::READ_SET,
		}
	}

	/// What is on tmpfs.
	pub fn read() -> Result<SsoState> { Self::read_in(&Self::dir()?) }

	/// [`read`](Self::read) the session in `dir`, a test's scratch directory.
	pub fn read_in(dir: &Path) -> Result<SsoState> {
		let path = dir.join(Self::SESSION_FILE);
		if !fs_ext::exists(&path)? {
			return SsoState::None.xok();
		}
		if !fs_ext::is_private(&path)? {
			fs_ext::remove(&path)?;
			bevybail!(
				"the Identity Center session at `{}` was readable by more than \
				its owner, so it has been removed rather than used",
				path.display()
			);
		}
		let Ok(session) =
			serde_json::from_str::<Self>(&fs_ext::read_to_string(&path)?)
		else {
			fs_ext::remove(&path)?;
			return SsoState::None.xok();
		};
		match session.tokens.refresh_token().is_some()
			|| session.tokens.is_fresh(Timestamp::now())
		{
			true => SsoState::Live(session).xok(),
			false => {
				fs_ext::remove(&path)?;
				SsoState::Lapsed.xok()
			}
		}
	}

	/// Write the session into `dir`, owner only.
	pub fn write_in(&self, dir: &Path) -> Result {
		fs_ext::create_dir_private(dir)?;
		fs_ext::write_private(
			dir.join(Self::SESSION_FILE),
			serde_json::to_string(self)?,
		)?;
		OK
	}

	/// Remove the session and this machine's client registration from
	/// `dir`.
	pub fn remove_in(dir: &Path) -> Result {
		for name in [Self::SESSION_FILE, Self::CLIENT_FILE] {
			let path = dir.join(name);
			if fs_ext::exists(&path)? {
				fs_ext::remove(&path)?;
			}
		}
		OK
	}
}

#[cfg(all(test, feature = "server", feature = "ureq"))]
mod test {
	use super::*;
	use base64::Engine;
	use sha2::Digest;
	use std::sync::Mutex;

	/// What the fake OAuth service and portal saw and hold.
	#[derive(Default)]
	struct Fake {
		registrations: u32,
		/// The challenge the last authorization url carried.
		challenge: Option<String>,
		/// Device polls answered so far.
		polls: u32,
		/// The access token issued last, which the portal accepts.
		token: u32,
	}

	type Shared = Arc<Mutex<Fake>>;

	/// A fake OIDC service and access portal on one local server.
	async fn fake() -> (String, Shared) {
		let shared = Shared::default();
		let state = shared.clone();
		let url = HttpServer::spawn_test(exchange_ext::handler_async(move |request: Request| {
			let state = state.clone();
			async move {
				let path = request.url().path_string();
				let bearer = request
					.headers
					.first_raw("x-amz-sso_bearer_token")
					.map(ToString::to_string);
				let account = request.get_param("account_id").map(ToString::to_string);
				let role = request.get_param("role_name").map(ToString::to_string);
				let body = request.body.into_bytes().await.unwrap_or_default();
				let body: Value = serde_json::from_slice(&body).unwrap_or_default();
				let mut fake = state.lock().unwrap();
				let error = |code: &str| {
					Response::from_status(StatusCode::BAD_REQUEST)
						.with_header("x-amzn-errortype", code)
						.with_body(format!(r#"{{"error":"{code}"}}"#))
				};
				let token = |fake: &mut Fake| {
					fake.token += 1;
					Response::ok_text(
						json!({
							"accessToken": format!("access-{}", fake.token),
							"refreshToken": format!("refresh-{}", fake.token),
							"expiresIn": 3600,
							"tokenType": "Bearer",
						})
						.to_string(),
					)
				};
				let live = |fake: &Fake| bearer.as_deref() == Some(format!("access-{}", fake.token).as_str());
				match path.as_str() {
					"/client/register" => {
						fake.registrations += 1;
						Response::ok_text(
							json!({
								"clientId": "client-1",
								"clientSecret": "client-secret",
								"clientSecretExpiresAt": Timestamp::now().secs() + 90 * 86400,
							})
							.to_string(),
						)
					}
					"/device_authorization" => Response::ok_text(
						json!({
							"deviceCode": "device-1",
							"userCode": "ABCD-EFGH",
							"verificationUri": "https://device.sso/",
							"verificationUriComplete": "https://device.sso/?user_code=ABCD-EFGH",
							"expiresIn": 600,
							"interval": 0,
						})
						.to_string(),
					),
					"/token" => match body["grantType"].as_str().unwrap_or_default() {
						"authorization_code" => {
							// PKCE, verified as the server does
							let verifier = body["codeVerifier"].as_str().unwrap_or_default();
							let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
								.encode(sha2::Sha256::digest(verifier.as_bytes()));
							match (Some(challenge) == fake.challenge, body["code"].as_str()) {
								(true, Some("auth-code")) => token(&mut fake),
								_ => error("InvalidGrantException"),
							}
						}
						IdentityCenterClient::DEVICE_GRANT => {
							fake.polls += 1;
							match fake.polls {
								1 => error("AuthorizationPendingException"),
								2 => error("SlowDownException"),
								_ => token(&mut fake),
							}
						}
						"refresh_token" => match body["refreshToken"].as_str() == Some(format!("refresh-{}", fake.token).as_str()) {
							true => token(&mut fake),
							false => error("InvalidGrantException"),
						},
						_ => error("UnsupportedGrantTypeException"),
					},
					"/assignment/accounts" if live(&fake) => Response::ok_text(
						json!({"accountList": [{"accountId": "222", "accountName": "beetmash"}]}).to_string(),
					),
					"/assignment/roles" if live(&fake) && account.as_deref() == Some("222") => {
						Response::ok_text(
							json!({"roleList": [
								{"roleName": "beet-admin", "accountId": "222"},
								{"roleName": "beet-read", "accountId": "222"},
							]})
							.to_string(),
						)
					}
					"/federation/credentials" if live(&fake) && account.as_deref() == Some("222") => {
						Response::ok_text(
							json!({"roleCredentials": {
								"accessKeyId": format!("ASIA-{}", role.unwrap_or_default()),
								"secretAccessKey": "secret",
								"sessionToken": "session",
								"expiration": Timestamp::now().millis() + 3_600_000,
							}})
							.to_string(),
						)
					}
					"/logout" if live(&fake) => Response::ok(),
					_ => Response::from_status(StatusCode::UNAUTHORIZED)
						.with_header("x-amzn-errortype", "UnauthorizedException"),
				}
			}
		}))
		.await;
		(url, shared)
	}

	fn identity_center(url: &str, shared: Shared) -> IdentityCenterClient {
		IdentityCenterClient::new(
			"us-east-1",
			"https://d-123.awsapps.com/start",
		)
		.with_endpoints(url, url)
		.with_presenter(move |prompt| {
			let shared = shared.clone();
			Box::pin(async move {
				// the person: approve in the browser, which redirects back
				if let SsoPrompt::Browser(url) = prompt {
					let url = Url::parse(&url)?;
					let param = |name: &str| {
						url.get_param(name).unwrap_or_default().to_string()
					};
					shared.lock().unwrap().challenge =
						Some(param("code_challenge"));
					let redirect = format!(
						"{}?code=auth-code&state={}",
						param("redirect_uri"),
						param("state")
					);
					// the redirect lands once the receiver is waiting
					async_ext::spawn(async move {
						time_ext::sleep_millis(50).await;
						Request::get(redirect).send().await.ok();
					})
					.detach();
				}
				OK
			})
		})
	}

	/// The browser login: registration once, the code redeemed with the
	/// verifier PKCE proves, and the portal reached with the token.
	#[beet_core::test]
	async fn signs_in_through_the_browser() {
		let (url, shared) = fake().await;
		let temp = TempDir::new().unwrap();
		let dir: &Path = temp.as_ref();
		let center = identity_center(&url, shared.clone());
		let world = crate::oauth::threaded_world();
		let session = center.login(dir, &world).await.unwrap();
		center
			.account(&session, "beetmash")
			.await
			.unwrap()
			.id
			.as_str()
			.xpect_eq("222");
		// a second login reuses the cached registration
		center.login(dir, &world).await.unwrap();
		shared.lock().unwrap().registrations.xpect_eq(1);
		format!("{session:?}").xnot().xpect_contains("access-");
	}

	/// The device login polls through pending and slow down to its token.
	#[beet_core::test]
	async fn signs_in_with_a_device_code() {
		let (url, shared) = fake().await;
		let temp = TempDir::new().unwrap();
		let dir: &Path = temp.as_ref();
		let session = identity_center(&url, shared.clone())
			.login_device_polled(dir, |interval, expires| {
				DeviceGrant::new(interval, expires)
					.with_slow_down_step(Duration::ZERO)
			})
			.await
			.unwrap();
		shared.lock().unwrap().polls.xpect_eq(3);
		session.tokens().refresh_token().xpect_eq(Some("refresh-1"));
	}

	static CLOCK: core::sync::atomic::AtomicI64 =
		core::sync::atomic::AtomicI64::new(0);
	fn clock() -> Timestamp {
		Timestamp::from_millis(
			Timestamp::now().millis()
				+ CLOCK.load(core::sync::atomic::Ordering::SeqCst),
		)
	}

	/// The read set is reached freely through refreshes; the admin set only
	/// inside the hour after the interactive sign in, which a refresh never
	/// reopens. The role's credential signs a request.
	#[beet_core::test]
	async fn admin_only_inside_the_window() {
		let (url, shared) = fake().await;
		let temp = TempDir::new().unwrap();
		let dir: &Path = temp.as_ref();
		let center = identity_center(&url, shared).with_clock(clock);
		let session = center
			.login_device_polled(dir, |interval, expires| {
				DeviceGrant::new(interval, expires)
					.with_slow_down_step(Duration::ZERO)
			})
			.await
			.unwrap();
		let account = center.account(&session, "beetmash").await.unwrap();
		center
			.role_credentials(
				&session,
				&account,
				IdentityCenterSession::ADMIN_SET,
			)
			.await
			.unwrap()
			.access_key_id()
			.xpect_eq("ASIA-beet-admin");
		// two hours on: the token is stale, so it refreshes, and the window
		// is shut
		CLOCK.store(2 * 3_600_000, core::sync::atomic::Ordering::SeqCst);
		let session = center.fresh(session).await.unwrap();
		session.tokens().access_token().xpect_eq("access-2");
		session
			.role_at(clock())
			.xpect_eq(IdentityCenterSession::READ_SET);
		center
			.role_credentials(
				&session,
				&account,
				IdentityCenterSession::ADMIN_SET,
			)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("only inside the hour");
		let read = center
			.role_credentials(
				&session,
				&account,
				IdentityCenterSession::READ_SET,
			)
			.await
			.unwrap();
		read.access_key_id().xpect_eq("ASIA-beet-read");
		SigV4::new("us-east-1", "sts")
			.sign(
				Request::get("https://sts.amazonaws.com/"),
				&read,
				Timestamp::now(),
			)
			.await
			.unwrap()
			.headers
			.first_raw("x-amz-security-token")
			.xpect_eq(Some("session"));
		CLOCK.store(0, core::sync::atomic::Ordering::SeqCst);
		center.logout(&session).await.unwrap();
	}

	/// A session round trips owner only; one with nothing left to refresh is
	/// lapsed and removed, and a missing one is none.
	#[beet_core::test]
	fn a_session_lapses() {
		let temp = TempDir::new().unwrap();
		let dir: &Path = temp.as_ref();
		let session = |tokens: OAuthTokens| IdentityCenterSession {
			region: "us-east-1".into(),
			start_url: "https://d-123.awsapps.com/start".into(),
			client: SsoClient {
				client_id: "client".into(),
				client_secret: "secret".into(),
				expires_at: Timestamp::now() + Duration::from_secs(86400),
				start_url: "https://d-123.awsapps.com/start".into(),
			},
			tokens,
		};
		matches!(IdentityCenterSession::read_in(dir).unwrap(), SsoState::None)
			.xpect_true();
		session(OAuthTokens::signed_in(
			"access",
			Some("refresh".into()),
			Duration::from_secs(60),
			Timestamp::now(),
		))
		.write_in(dir)
		.unwrap();
		fs_ext::is_private(dir.join(IdentityCenterSession::SESSION_FILE))
			.unwrap()
			.xpect_true();
		matches!(
			IdentityCenterSession::read_in(dir).unwrap(),
			SsoState::Live(_)
		)
		.xpect_true();
		session(OAuthTokens::signed_in(
			"access",
			None,
			Duration::from_secs(1),
			Timestamp::from_secs(0),
		))
		.write_in(dir)
		.unwrap();
		matches!(
			IdentityCenterSession::read_in(dir).unwrap(),
			SsoState::Lapsed
		)
		.xpect_true();
		fs_ext::exists(dir.join(IdentityCenterSession::SESSION_FILE))
			.unwrap()
			.xpect_false();
	}
}
