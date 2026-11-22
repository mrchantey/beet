//! `sso`: a person's IAM Identity Center sign in, and the invite that adds
//! one.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// `<SsoRoutes/>`: the Identity Center verbs, authored under
/// `<Route path="sso">` beside `admin`:
///
/// ```sh
/// beet sso login                 # through the browser, PKCE
/// beet sso login --no-browser    # a url and a code, from an ssh session
/// beet sso status
/// beet sso logout
/// beet sso invite pete@example.com --given=Pete --family=Hayman
/// ```
///
/// A sign in is the human path into every member account the operator group
/// is assigned (see [`IdentityCenterClient`] for the session rules): `beet aws
/// --account=<name>` then reaches the account as `beet-read`, or as
/// `beet-admin` inside the hour after the sign in, and the access portal at
/// the start url opens each account's console.
///
/// ## Joining
///
/// `invite` (as the management account's administrator, `beet admin`)
/// creates the person's user and adds it to the operator group; the rest is
/// the person's own. Identity Center sends no email for a user an api made,
/// so the operator opens the console once, IAM Identity Center > Users >
/// the user > Reset password > Send an email; the person follows the email,
/// sets a password, and registers an MFA device at the first sign in, which
/// the instance's MFA policy (a console setting) requires.
#[template]
pub fn SsoRoutes() -> impl Bundle {
	Children::spawn((
		Spawn(SsoLogin),
		Spawn(SsoLogout),
		Spawn(SsoStatus),
		Spawn(SsoInvite),
	))
}

/// Sign in to the declared Identity Center, keeping the session on tmpfs.
#[action(route = "login")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn SsoLogin(
	/// Sign in with a url and a code to approve from any machine, rather than
	/// through a browser on this one, ie over ssh.
	#[param]
	no_browser: bool,
	cx: ActionContext<Request>,
) -> Result<Response> {
	let client = SsoLogin::client(&cx.caller).await?;
	let dir = IdentityCenterSession::dir()?;
	let session = match no_browser {
		true => client.login_device(&dir).await?,
		false => client.login(&dir, cx.caller.world()).await?,
	};
	session.write_in(&dir)?;
	let accounts = client.accounts(&session).await?;
	Response::ok_text(SsoStatus::describe(&session, &accounts)).xok()
}

impl SsoLogin {
	/// The declared instance's sign in: its start url declared, else derived
	/// from the instance `beet-agent` finds.
	async fn client(caller: &AsyncEntity) -> Result<IdentityCenterClient> {
		let (center, region) = IdentityCenter::require(caller).await?;
		let start_url = match center.start_url {
			Some(start_url) => start_url,
			None => IdentityCenterAdmin::new(
				&region,
				AgentIdentity::credentials().await?,
			)
			.instance()
			.await?
			.ok_or_else(|| {
				bevyhow!(
					"the Organization has no Identity Center instance yet: \
						enabling it is a console step, see `AwsAccount`'s docs"
				)
			})?
			.start_url()
			.into(),
		};
		IdentityCenterClient::new(region, start_url).xok()
	}
}

/// End the Identity Center session, here and at Identity Center.
#[action(route = "logout")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn SsoLogout() -> Result<Response> {
	let dir = IdentityCenterSession::dir()?;
	let text = match IdentityCenterSession::read_in(&dir)? {
		SsoState::Live(session) => {
			// a token Identity Center already ended is the same outcome
			if let Err(err) = session.identity_center().logout(&session).await {
				warn!("Identity Center refused the sign out: {err}");
			}
			"signed out of Identity Center\n"
		}
		SsoState::Lapsed => "the Identity Center session had already lapsed\n",
		SsoState::None => "not signed in to Identity Center\n",
	};
	IdentityCenterSession::remove_in(&dir)?;
	Response::ok_text(text).xok()
}

/// Name the Identity Center session: who it signs in to, the accounts it
/// reaches, and the time left on it and on its admin window, on stderr.
#[action(route = "status")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn SsoStatus() -> Result<Response> {
	let dir = IdentityCenterSession::dir()?;
	let text = match IdentityCenterSession::read_in(&dir)? {
		SsoState::Live(session) => {
			let client = session.identity_center();
			let session = client.fresh(session).await?;
			session.write_in(&dir)?;
			let accounts = client.accounts(&session).await?;
			SsoStatus::describe(&session, &accounts)
		}
		SsoState::Lapsed => {
			"the Identity Center session has lapsed: `beet sso login` opens \
			another"
				.to_string()
		}
		SsoState::None => "not signed in: `beet sso login`".to_string(),
	};
	info!("{}", text.trim_end());
	Response::ok().xok()
}

impl SsoStatus {
	/// A session at a glance.
	fn describe(
		session: &IdentityCenterSession,
		accounts: &[SsoAccount],
	) -> String {
		let now = Timestamp::now();
		let window = match session.admin_window_open(now) {
			true => format!(
				"`{}` open for {}",
				IdentityCenterSession::ADMIN_SET,
				Self::remaining(
					session.tokens().signed_in_at()
						+ IdentityCenterSession::ADMIN_WINDOW,
					now
				)
			),
			false => format!(
				"`{}` closed, `{}` only",
				IdentityCenterSession::ADMIN_SET,
				IdentityCenterSession::READ_SET
			),
		};
		let accounts = accounts
			.iter()
			.map(|account| format!("\n  {} ({})", account.name, account.id))
			.collect::<String>();
		format!(
			"signed in to {}, the token good for {}, {window}; reaches:{accounts}\n",
			session.start_url(),
			Self::remaining(session.tokens().expires_at(), now)
		)
	}

	/// The time from `now` to `until`, ie `48m`.
	fn remaining(until: Timestamp, now: Timestamp) -> String {
		AwsExec::describe_remaining(
			((until.millis() - now.millis()) / 1000).max(0),
		)
	}
}

/// Invite a person: their Identity Center user, created when absent, added
/// to the operator group, as the management account's administrator.
#[action(route = "invite/:email")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn SsoInvite(
	/// The person's given name, else the email's local part.
	#[param]
	given: Option<String>,
	/// The person's family name, else blank.
	#[param]
	family: Option<String>,
	cx: ActionContext<Request>,
) -> Result<Response> {
	let email = cx
		.input
		.get_param("email")
		.map(str::trim)
		.filter(|email| email.contains('@'))
		.map(SmolStr::new)
		.ok_or_else(|| {
			bevyhow!(
				"an invite names an email, ie `sso/invite pete@example.com`"
			)
		})?;
	let (center, region) = IdentityCenter::require(&cx.caller).await?;
	let session = AdminSession::acquire(AdminSession::DEFAULT_SECS).await?;
	let admin =
		IdentityCenterAdmin::new(&region, session.credentials().clone());
	let instance = admin.instance().await?.ok_or_else(|| {
		bevyhow!("the Organization has no Identity Center instance yet: enabling it is a console step")
	})?;
	let group = admin
		.find_group(&instance, &center.group)
		.await?
		.ok_or_else(|| {
			bevyhow!(
				"the Identity Center has no `{}` group yet: an elevated deploy of \
			the package declaring `<AwsAccount/>` creates it",
				center.group
			)
		})?;
	let given = given.unwrap_or_else(|| {
		email.split('@').next().unwrap_or_default().to_string()
	});
	let family = family.unwrap_or_default();
	let (user, created) = match admin.find_user(&instance, &email).await? {
		Some(user) => (user, false),
		None => (
			admin
				.create_user(&instance, &email, &given, &family)
				.await?,
			true,
		),
	};
	let added = admin.ensure_member(&instance, &group, &user).await?;
	Response::ok_text(format!(
		"{email}: user {}, {} the `{}` group. Now send the sign in email from \
		the console (IAM Identity Center > Users > {email} > Reset password > \
		Send an email); they set a password and register an MFA device, then \
		`beet sso login` at {}\n",
		match created {
			true => "created",
			false => "exists",
		},
		match added {
			true => "added to",
			false => "already in",
		},
		center.group,
		center
			.start_url
			.clone()
			.unwrap_or_else(|| instance.start_url().into()),
	))
	.xok()
}
