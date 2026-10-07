//! `aws`: the aws cli as this machine's identity, whichever one that is.

use crate::prelude::*;
use beet_core::prelude::*;

/// Request params for [`AwsExec`], surfaced in `--help`.
#[derive(Reflect)]
struct AwsParams {
	/// The aws cli arguments, everything after `--`.
	nested_args: Vec<String>,
}

/// Run the aws cli as this machine's own identity: `beet-agent` normally, or
/// the `beet-admin` role while a session minted by [`AdminSession`] is live.
///
/// ```sh
/// beet aws -- sts get-caller-identity
/// beet aws -- iam list-roles
/// beet aws -- logs tail /beet-site/main-lightsail/prod --region us-west-2
/// ```
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
#[action]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(
	PathPartial = PathPartial::new("aws"),
	ParamsPartial = ParamsPartial::new::<AwsParams>()
)]
pub async fn AwsExec(cx: ActionContext<Request>) -> Result<Response> {
	let params = cx.input.parse_params::<AwsParams>()?;
	if params.nested_args.is_empty() {
		bevybail!(
			"arguments are required after `--`, ie `beet aws -- sts \
			get-caller-identity`"
		);
	}
	// a live session wins: the operator typed a code to get it, and silently
	// running as the weaker identity would waste that and confuse the failure
	let state = AdminSession::read()?;
	let (vars, who) = match &state {
		SessionState::Live(session) => (
			session.sdk_vars(),
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
				AgentIdentity::sdk_pair().await?,
				format!("`{}`, the session having lapsed", AgentIdentity::USER),
			)
		}
		SessionState::None => (
			AgentIdentity::sdk_pair().await?,
			format!("`{}`, which can read and not write", AgentIdentity::USER),
		),
	};
	info!("aws as {who}");
	let status = vars
		.iter()
		.fold(ChildProcess::new("aws"), |process, (_, value)| {
			process.with_secret(value.clone())
		})
		.without_launch_env()
		// an inherited profile would address a different identity than the one
		// just named
		.without_env("AWS_PROFILE")
		// and any stale token: a long-lived pair plus a foreign session token
		// authenticates as nobody. Removals apply before additions, so a
		// session re-adds its own token over this.
		.without_env("AWS_SESSION_TOKEN")
		.with_args(params.nested_args.iter().map(String::as_str))
		.with_envs(vars)
		.spawn()?
		.status()
		.await?;
	match status.success() {
		true => Response::ok().xok(),
		// the cli already printed its own error, so this adds only the thing
		// the cli cannot know: that a stronger identity is one command away
		false => match matches!(state, SessionState::Live(_)) {
			true => {
				bevybail!("`aws` exited with {}", status.code().unwrap_or(-1))
			}
			false => bevybail!(
				"`aws` exited with {}. If that was an `AccessDenied`, this ran \
				as `{}`, which is read-only by design: `beet admin` mints an \
				administrator session with one code from your phone, and this \
				verb then uses it automatically",
				status.code().unwrap_or(-1),
				AgentIdentity::USER
			),
		},
	}
}

impl AwsExec {
	/// A remaining duration a human reads at a glance, ie `48m` or `2h11m`.
	pub(crate) fn describe_remaining(secs: i64) -> String {
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
