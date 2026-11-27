//! `aws`: the aws cli as this machine's identity, whichever one that is.

use crate::prelude::*;
use beet_core::prelude::*;

/// Run the aws cli as this machine's own identity: `beet-agent` normally, or
/// the `beet-admin` role while a session minted by [`AdminSession`] is live.
///
/// ```sh
/// beet aws -- sts get-caller-identity
/// beet aws -- iam list-roles
/// beet aws -- logs tail /beet-site/main-lightsail/prod --region us-west-2
/// beet aws --account=beetmash -- s3 ls      # inside a package's own account
/// ```
///
/// With `--account=<name>` the call runs inside that member account of the
/// Organization. A live Identity Center session ([`IdentityCenterSession`],
/// `beet sso login`) reaches it first, as `beet-admin` inside the hour after
/// its interactive sign in and `beet-read` after. Without one, the
/// administrator session chains into it ([`MemberAccount::ADMIN_ROLE`]) when
/// one is live, else `beet-agent` assumes its read role
/// ([`MemberAccount::READ_ROLE`]), so the same split of read by default and
/// write behind a human act holds in every account.
///
/// **A regional service needs its region naming**, as the third line does.
/// There is no `~/.aws/config` on a beet machine to supply one ambiently --
/// that is the point, since ambient configuration is what made every tool on
/// the machine an administrator -- so `iam` and `sts` work bare while `logs`,
/// `lightsail` and the rest want `--region`. An exported `AWS_REGION` also
/// works, since only this launch's own variables are stripped.
///
/// ## It always says which
///
/// Every run names the identity on stderr before the child starts, because the
/// two differ by everything that matters and nobody -- operator or agent --
/// should have to infer which one a command ran as. Under a session it also
/// prints what is left of it, so the answer to "why did that just start
/// failing" is on screen.
///
/// stderr, never stdout: the child's stdout is the answer, and a caller piping
/// `beet aws -- ... | jq` gets json rather than a preamble.
///
/// ## Why this exists rather than `aws` directly
///
/// Because the safe path has to be the shortest path. The alternative to one
/// obvious command is an agent under time pressure reaching for whatever
/// credential is nearest, and the nearest one used to be an administrator in
/// `~/.aws/credentials` that every tool on the machine read without being
/// asked. This reads one sealed document, hands two variables to one child, and
/// authenticates nothing else.
#[action(route = "aws")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn AwsExec(
	/// The member account of the Organization to run inside, by name, ie the
	/// package's own. Unset is the management account.
	#[param]
	account: Option<String>,
	/// The aws cli arguments, everything after `--`.
	#[param]
	nested_args: Vec<String>,
	cx: ActionContext<Request>,
) -> Result<Response> {
	if nested_args.is_empty() {
		bevybail!(
			"arguments are required after `--`, ie `beet aws -- sts \
			get-caller-identity`"
		);
	}
	// a person's own sign in reaches a named account before any machine
	// identity does
	if let Some(name) = account.as_deref()
		&& let Some((credentials, who)) = AwsExec::through_sso(name).await?
	{
		info!("aws as {who}");
		return AwsExec::run(&credentials, &nested_args, true).await;
	}
	// a live session wins: the operator typed a code to get it, and silently
	// running as the weaker identity would waste that and confuse the failure
	let state = AdminSession::read()?;
	let (credentials, who) = match &state {
		SessionState::Live(session) => (
			session.credentials().clone(),
			format!(
				"the `{}` role, {} left on the session",
				AgentIdentity::ADMIN_ROLE,
				AwsExec::describe_remaining(session.remaining_secs())
			),
		),
		// a lapsed session says so rather than quietly demoting: the command
		// that worked a minute ago is about to fail, and this is why
		SessionState::Expired => {
			warn!(
				"the administrator session has expired, so this falls back to \
				`{}`: `beet admin` mints another with one code",
				AgentIdentity::USER
			);
			(
				AgentIdentity::credentials().await?,
				format!("`{}`, the session having lapsed", AgentIdentity::USER),
			)
		}
		SessionState::None => (
			AgentIdentity::credentials().await?,
			format!("`{}`, which can read and not write", AgentIdentity::USER),
		),
	};
	// inside a member, by the role this identity holds there
	let (credentials, who) = match account.as_deref() {
		Some(name) => {
			let role = match &state {
				SessionState::Live(_) => MemberAccount::ADMIN_ROLE,
				_ => MemberAccount::READ_ROLE,
			};
			let member = MemberAccount::find(credentials.clone(), name).await?;
			let inside = member.assume(credentials, role).await?;
			(
				inside,
				format!(
					"{who}, as `{role}` in `{}` ({})",
					member.name, member.id
				),
			)
		}
		None => (credentials, who),
	};
	info!("aws as {who}");
	AwsExec::run(
		&credentials,
		&nested_args,
		matches!(state, SessionState::Live(_)),
	)
	.await
}

impl AwsExec {
	/// The child as `credentials`, its failure naming the stronger identity
	/// one command away unless `elevated` already is.
	async fn run(
		credentials: &AwsCredentials,
		nested_args: &[String],
		elevated: bool,
	) -> Result<Response> {
		let status = AgentIdentity::aws(
			&credentials.env_vars(),
			nested_args.iter().map(String::as_str),
		)
		.spawn()?
		.status()
		.await?;
		match (status.success(), elevated) {
			(true, _) => Response::ok().xok(),
			(false, true) => {
				bevybail!("`aws` exited with {}", status.code().unwrap_or(-1))
			}
			// the cli already printed its own error, so this adds only the
			// thing the cli cannot know: that a stronger identity is one
			// command away
			(false, false) => bevybail!(
				"`aws` exited with {}. If that was an `AccessDenied`, this ran \
				as `{}`, which is read-only by design: `beet admin` mints an \
				administrator session with one code from your phone, or `beet \
				sso login` opens an hour of `beet-admin` in a member account, \
				and this verb then uses it automatically",
				status.code().unwrap_or(-1),
				AgentIdentity::USER
			),
		}
	}

	/// The account `name` through a live Identity Center session: its role
	/// credential and who that is, the refreshed session written back, `None`
	/// when there is no session.
	async fn through_sso(
		name: &str,
	) -> Result<Option<(AwsCredentials, String)>> {
		let dir = IdentityCenterSession::dir()?;
		let session = match IdentityCenterSession::read_in(&dir)? {
			SsoState::Live(session) => session,
			SsoState::Lapsed => {
				warn!(
					"the Identity Center session has lapsed, so this falls back \
					to this machine's identity: `beet sso login` opens another"
				);
				return None.xok();
			}
			SsoState::None => return None.xok(),
		};
		let center = session.identity_center();
		let session = center.fresh(session).await?;
		session.write_in(&dir)?;
		let account = center.account(&session, name).await?;
		let role = session.role_at(Timestamp::now());
		let credentials =
			center.role_credentials(&session, &account, role).await?;
		Some((
			credentials,
			format!(
				"your Identity Center sign in, as `{role}` in `{}` ({})",
				account.name, account.id
			),
		))
		.xok()
	}

	/// A remaining duration a human reads at a glance, ie `48m` or `2h11m`.
	pub fn describe_remaining(secs: i64) -> String {
		match (secs / 3600, (secs % 3600) / 60) {
			(0, mins) => format!("{mins}m"),
			(hours, 0) => format!("{hours}h"),
			(hours, mins) => format!("{hours}h{mins}m"),
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// The remaining time reads at a glance, since it is the thing that
	/// decides whether a long command will outlive its own credential.
	#[beet_core::test]
	fn describes_what_is_left() {
		AwsExec::describe_remaining(0).xpect_eq("0m");
		AwsExec::describe_remaining(60 * 48).xpect_eq("48m");
		AwsExec::describe_remaining(3600 * 2).xpect_eq("2h");
		AwsExec::describe_remaining(3600 * 2 + 60 * 11).xpect_eq("2h11m");
	}
}
