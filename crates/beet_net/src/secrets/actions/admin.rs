//! `admin`: an administrator session, bought with a code from a phone.

use crate::prelude::*;
use beet_core::prelude::*;

/// Request params for [`AdminElevate`], surfaced in `--help`.
#[derive(Reflect)]
struct AdminParams {
	/// How long the session lasts, `30m` to `12h`, default `1h`. STS enforces
	/// its own floor of 15 minutes and the role's 12-hour ceiling.
	duration: Option<String>,
	/// A command to run under fresh credentials, everything after `--`.
	/// Nothing is persisted, so the grant lives exactly as long as the
	/// command.
	nested_args: Vec<String>,
}

/// Become an administrator for a session: the one path to tier 0a.
///
/// ```sh
/// beet admin                                  # an hour, stored on tmpfs
/// beet admin --duration=8h                    # a working day
/// beet admin -- deployer/mint --stage=prod     # one command, nothing kept
/// ```
///
/// Opens the global document for [`beet-agent`](AgentIdentity)'s pair, prompts
/// for the six-digit code on the terminal, and calls `sts:AssumeRole` on the
/// `beet-admin` role with that code. Without `--`, the temporary credentials go
/// to `$XDG_RUNTIME_DIR/beet/admin-session` (tmpfs, mode 600) and every later
/// [`beet aws`](AwsExec) uses them until they lapse. With `-- <command>`, they
/// reach that one child and are persisted nowhere.
///
/// ## The code is typed, and that is the design
///
/// It is read from the terminal and never from an argument or an environment
/// variable, because both are readable by other processes and one of them lands
/// in shell history. More importantly it is the one thing an automated process
/// cannot supply, which is exactly why the role's trust policy conditions on
/// it: `aws:MultiFactorAuthPresent` must be true, and no amount of held
/// credential makes it so.
///
/// So an agent cannot mint this session. An agent running in the terminal where
/// the operator just typed a code CAN use it, for as long as the operator sized
/// it, and that is intended rather than a leak: the human factor was required,
/// the grant is bounded, and it expires on its own.
#[action]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(
	PathPartial = PathPartial::new("admin"),
	ParamsPartial = ParamsPartial::new::<AdminParams>()
)]
pub async fn AdminElevate(cx: ActionContext<Request>) -> Result<Response> {
	let params = cx.input.parse_params::<AdminParams>()?;
	let seconds = AdminElevate::parse_duration(params.duration.as_deref())?;
	let agent = AgentIdentity::sdk_pair().await?;
	let account = AdminElevate::account_id(&agent).await?;
	let code = terminal_ext::read_secret_line(&format!(
		"six-digit code for the `{}` mfa device: ",
		AgentIdentity::USER
	))?;
	AdminElevate::check_code(&code)?;
	let session =
		AdminElevate::assume(&agent, &account, &code, seconds).await?;
	match params.nested_args.split_first() {
		// one command, nothing kept: the grant lives as long as the child
		Some((command, args)) => {
			info!(
				"running `{command}` as the `{}` role, keeping nothing",
				AgentIdentity::ADMIN_ROLE
			);
			let status = session
				.sdk_vars()
				.iter()
				.fold(
					ChildProcess::new(command.as_str()),
					|process, (_, value)| process.with_secret(value.clone()),
				)
				.without_launch_env()
				.without_env("AWS_PROFILE")
				.without_env("AWS_SESSION_TOKEN")
				.without_env("AWS_SESSION_TOKEN")
				// and any stale token: a long-lived pair plus a foreign session token
				// authenticates as nobody. Removals apply before additions, so a
				// session re-adds its own token over this.
				.without_env("AWS_SESSION_TOKEN")
				.with_args(args.iter().map(String::as_str))
				.with_envs(session.sdk_vars())
				.spawn()?
				.status()
				.await?;
			match status.success() {
				true => Response::ok().xok(),
				false => bevybail!(
					"`{command}` exited with {}",
					status.code().unwrap_or(-1)
				),
			}
		}
		None => {
			session.write()?;
			// the expiry and nothing else: the credential itself never prints
			Response::ok_text(format!(
				"administrator until {} ({} from now), on tmpfs at {}\n",
				Timestamp::from_secs(session.expires).format_iso8601(),
				AwsExec::describe_remaining(session.remaining_secs()),
				AdminSession::path()?.display()
			))
			.xok()
		}
	}
}

impl AdminElevate {
	/// The default grant, an hour: long enough for a mint or a rotation, short
	/// enough that forgetting it costs nothing.
	const DEFAULT: i64 = 3600;
	/// The role's own `MaxSessionDuration`, which STS refuses to exceed.
	const CEILING: i64 = 43200;
	/// STS's floor for an assumed role.
	const FLOOR: i64 = 900;

	/// `30m`, `2h`, `90m`, or a bare number of seconds. Bounded here as well
	/// as by STS so the failure names the limit rather than arriving as an
	/// api error after the code has been typed.
	fn parse_duration(value: Option<&str>) -> Result<i64> {
		let Some(value) =
			value.map(str::trim).filter(|value| !value.is_empty())
		else {
			return Self::DEFAULT.xok();
		};
		let (digits, multiplier) = match value.chars().last() {
			Some('h') => (&value[..value.len() - 1], 3600),
			Some('m') => (&value[..value.len() - 1], 60),
			Some('s') => (&value[..value.len() - 1], 1),
			_ => (value, 1),
		};
		let seconds = digits
			.parse::<i64>()
			.map_err(|_| {
				bevyhow!(
					"`--duration={value}` is not a duration: write `30m`, \
					`1h` or `8h`"
				)
			})?
			.saturating_mul(multiplier);
		if !(Self::FLOOR..=Self::CEILING).contains(&seconds) {
			bevybail!(
				"`--duration={value}` is {seconds}s, outside what the role \
				allows: between {}m and {}h",
				Self::FLOOR / 60,
				Self::CEILING / 3600
			);
		}
		seconds.xok()
	}

	/// Six digits, checked here so a typo costs a re-prompt rather than a
	/// round trip and a burnt code.
	fn check_code(code: &str) -> Result {
		match code.len() == 6 && code.chars().all(|char| char.is_ascii_digit())
		{
			true => OK,
			false => bevybail!(
				"an mfa code is six digits; got {} character(s). The device is \
				the `{}` entry in your authenticator, not the `pete` one",
				code.len(),
				AgentIdentity::USER
			),
		}
	}

	/// This account, asked as the agent, so the role and device arns are
	/// composed rather than written down anywhere.
	async fn account_id(agent: &[(SmolStr, SmolStr)]) -> Result<String> {
		agent
			.iter()
			.fold(ChildProcess::new("aws"), |process, (_, value)| {
				process.with_secret(value.clone())
			})
			.without_launch_env()
			.without_env("AWS_PROFILE")
			.without_env("AWS_SESSION_TOKEN")
			// and any stale token: a long-lived pair plus a foreign session token
			// authenticates as nobody. Removals apply before additions, so a
			// session re-adds its own token over this.
			.without_env("AWS_SESSION_TOKEN")
			.with_args([
				"sts",
				"get-caller-identity",
				"--query",
				"Account",
				"--output",
				"text",
			])
			.with_envs(agent.to_vec())
			.run_async_stdout()
			.await
			.map(|out| out.trim().to_string())
			.map_err(|err| {
				bevyhow!(
					"`{}`'s pair does not answer `sts:GetCallerIdentity`, so \
					there is nobody to elevate FROM: the key may have been \
					rotated at the account without the global document \
					following it. {err}",
					AgentIdentity::USER
				)
			})
	}

	/// Trade the code for the role's temporary credentials.
	async fn assume(
		agent: &[(SmolStr, SmolStr)],
		account: &str,
		code: &str,
		seconds: i64,
	) -> Result<AdminSession> {
		let role = format!(
			"arn:aws:iam::{account}:role/{}",
			AgentIdentity::ADMIN_ROLE
		);
		// the device is named for the user, so its arn is composed the same way
		let device =
			format!("arn:aws:iam::{account}:mfa/{}", AgentIdentity::USER);
		let body = agent
			.iter()
			.fold(ChildProcess::new("aws"), |process, (_, value)| {
				process.with_secret(value.clone())
			})
			.without_launch_env()
			.without_env("AWS_PROFILE")
			.without_env("AWS_SESSION_TOKEN")
			// and any stale token: a long-lived pair plus a foreign session token
			// authenticates as nobody. Removals apply before additions, so a
			// session re-adds its own token over this.
			.without_env("AWS_SESSION_TOKEN")
			.with_secret(code)
			.with_args([
				"sts",
				"assume-role",
				"--role-arn",
				role.as_str(),
				"--role-session-name",
				"beet-admin",
				"--serial-number",
				device.as_str(),
				"--token-code",
				code,
				"--duration-seconds",
				seconds.to_string().as_str(),
				// the four fields as text rather than the json document: this
				// crate links no json parser outside its `json` feature, and
				// the cli's own `--query` is the narrower ask anyway
				"--query",
				"Credentials.[AccessKeyId,SecretAccessKey,SessionToken,Expiration]",
				"--output",
				"text",
			])
			.with_envs(agent.to_vec())
			.run_async_stdout()
			.await
			.map_err(|err| Self::explain(err, &device))?;
		Self::parse_credentials(&body, seconds)
	}

	/// The four tab-separated fields `--query` asked for, in that order.
	/// Separated from the call so the shape is testable without an account.
	fn parse_credentials(body: &str, seconds: i64) -> Result<AdminSession> {
		let fields = body.trim().split_whitespace().collect::<Vec<_>>();
		let [key_id, secret, token, expiration] = fields.as_slice() else {
			bevybail!(
				"`sts:AssumeRole` answered {} field(s) rather than four, so 				there is no session to keep",
				fields.len()
			);
		};
		AdminSession {
			key_id: (*key_id).into(),
			secret: (*secret).into(),
			token: (*token).into(),
			// the answer's own expiry rather than now plus the request, since
			// STS clamps the duration to the role's ceiling without saying so
			expires: Timestamp::parse_rfc3339(expiration)
				.map(|stamp| stamp.secs())
				.unwrap_or_else(|| Timestamp::now().secs() + seconds),
		}
		.xok()
	}

	/// Turn the three failures that actually happen into what to do about
	/// them. An `AccessDenied` here is almost never a permissions problem: it
	/// is a device that is not registered yet, or a code that has rolled.
	fn explain(err: BevyError, device: &str) -> BevyError {
		let text = err.to_string();
		if text.contains("MultiFactorAuthentication failed")
			|| text.contains("invalid MFA one time pass")
		{
			return bevyhow!(
				"that code was not accepted. A code is valid for about thirty \
				seconds, so the usual cause is a stale one: wait for the next \
				and try again. {text}"
			);
		}
		if text.contains("not authorized to perform: sts:AssumeRole")
			|| text.contains("AccessDenied")
		{
			return bevyhow!(
				"`{}` may not assume the `{}` role with that code. If you have \
				not registered the device yet, that is the cause and it is \
				expected: the role's trust policy requires an mfa code, so \
				until a device named `{}` exists at `{device}` the role is \
				unreachable by design. Register it in the console, IAM > \
				Users > {} > Security credentials > Assign MFA device, naming \
				it `{}`. {text}",
				AgentIdentity::USER,
				AgentIdentity::ADMIN_ROLE,
				AgentIdentity::USER,
				AgentIdentity::USER,
				AgentIdentity::USER
			);
		}
		err
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use beet_core::prelude::*;

	/// A duration is bounded here rather than at the api, so a bad one costs a
	/// re-run and not a burnt code.
	#[beet_core::test]
	fn parses_and_bounds_a_duration() {
		AdminElevate::parse_duration(None).unwrap().xpect_eq(3600);
		AdminElevate::parse_duration(Some("30m"))
			.unwrap()
			.xpect_eq(1800);
		AdminElevate::parse_duration(Some("12h"))
			.unwrap()
			.xpect_eq(43200);
		AdminElevate::parse_duration(Some("1800"))
			.unwrap()
			.xpect_eq(1800);
		// past the role's own ceiling, and under STS's floor
		AdminElevate::parse_duration(Some("13h"))
			.unwrap_err()
			.to_string()
			.xpect_contains("outside what the role allows");
		AdminElevate::parse_duration(Some("5m"))
			.unwrap_err()
			.to_string()
			.xpect_contains("outside what the role allows");
		AdminElevate::parse_duration(Some("soon"))
			.unwrap_err()
			.to_string()
			.xpect_contains("not a duration");
	}

	/// Six digits or a re-prompt, and the message names WHICH device, since
	/// the authenticator holds two entries for this account.
	#[beet_core::test]
	fn checks_the_code_shape() {
		AdminElevate::check_code("123456").unwrap();
		AdminElevate::check_code("12345")
			.unwrap_err()
			.to_string()
			.xpect_contains("six digits")
			.xpect_contains("beet-agent");
		AdminElevate::check_code("12345a").unwrap_err();
	}

	/// The four fields, in the order `--query` asked for them, with the
	/// answer's OWN expiry: STS clamps a duration to the role's ceiling
	/// without saying so, and a session that claims more than it has is a
	/// command that fails halfway.
	#[beet_core::test]
	fn parses_the_credentials() {
		let session = AdminElevate::parse_credentials(
			"ASIAEXAMPLE\tsecrethalf\tsessiontoken\t2026-09-30T12:00:00Z\n",
			3600,
		)
		.unwrap();
		session.key_id.as_str().xpect_eq("ASIAEXAMPLE");
		session.token.as_str().xpect_eq("sessiontoken");
		session.expires.xpect_eq(
			Timestamp::parse_rfc3339("2026-09-30T12:00:00Z")
				.unwrap()
				.secs(),
		);
		// a short answer is no session rather than a half one
		AdminElevate::parse_credentials("ASIAEXAMPLE\tsecrethalf", 3600)
			.unwrap_err()
			.to_string()
			.xpect_contains("rather than four");
	}

	/// An unregistered device is the expected first failure, so it must read
	/// as an instruction rather than as a permissions bug.
	#[beet_core::test]
	fn an_unregistered_device_says_so() {
		AdminElevate::explain(
			bevyhow!("AccessDenied: not authorized to perform: sts:AssumeRole"),
			"arn:aws:iam::1234:mfa/beet-agent",
		)
		.to_string()
		.xpect_contains("Assign MFA device")
		.xpect_contains("unreachable by design");
		AdminElevate::explain(
			bevyhow!("MultiFactorAuthentication failed"),
			"arn:aws:iam::1234:mfa/beet-agent",
		)
		.to_string()
		.xpect_contains("thirty");
	}
}
