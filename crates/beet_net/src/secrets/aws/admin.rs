//! `admin`: an administrator session, bought with a code from a phone.

use crate::prelude::*;
use beet_core::prelude::*;

/// Become an administrator for a session: the one path to tier 0a.
///
/// ```sh
/// beet admin                                  # an hour, stored on tmpfs
/// beet admin --duration=8h                    # a working day
/// beet admin -- shared/plan --stage=prod       # one command, nothing kept
/// beet admin --account=beetmash -- mail/plan   # one command inside a member
/// ```
///
/// Opens the global document for [`beet-agent`](AgentIdentity)'s pair, prompts
/// for the six-digit code on the terminal, and calls `sts:AssumeRole` on the
/// `beet-admin` role with that code, signed in this process
/// ([`AwsSts::assume_role`]). Without `--`, the temporary credentials go
/// to `$XDG_RUNTIME_DIR/beet/admin-session` (tmpfs, mode 600) and every later
/// [`beet aws`](AwsExec) uses them until they lapse. With `-- <command>`, the
/// live session runs the command when it has twenty minutes or more left,
/// else a fresh one is bought for it and persisted nowhere, so one code
/// covers a batch of commands.
///
/// With `--account=<name>` the session is chained into that member account of
/// the Organization ([`MemberAccount::ADMIN_ROLE`]) for the child, an hour at
/// most since STS caps a chained role there. Kept on tmpfs, the session stays
/// the management account's and the verb proves it reaches the member, so one
/// code opens the hour every `beet aws --account=<name>` chains from.
///
/// ## The code is typed, and that is the design
///
/// It is read from the terminal and travels only inside the one signed
/// `sts:AssumeRole` request body this process sends: never an argument, never
/// an environment variable, never a file. argv is readable by any process on
/// the machine through `/proc/<pid>/cmdline`, an environment by its children,
/// and a shell argument lands in history; a request body is readable by this
/// process and AWS alone, and an error that echoes the code has it redacted.
///
/// More importantly, a typed code is the one thing an automated process cannot
/// supply, which is exactly why the role's trust policy conditions on it:
/// `aws:MultiFactorAuthPresent` must be true, and no amount of held credential
/// makes it so.
///
/// So an agent cannot mint this session. An agent running in the terminal where
/// the operator just typed a code CAN use it, for as long as the operator sized
/// it, and that is intended rather than a leak: the human factor was required,
/// the grant is bounded, and it expires on its own.
#[action(route = "admin")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn AdminElevate(
	/// How long the session lasts, `30m` to `12h`, default `1h`. STS enforces
	/// its own floor of 15 minutes and the role's 12-hour ceiling.
	#[param]
	duration: Option<String>,
	/// The member account of the Organization to act in, by name, ie the
	/// package's own. Unset is the management account.
	#[param]
	account: Option<String>,
	/// A command to run under fresh credentials, everything after `--`.
	/// Nothing is persisted, so the grant lives exactly as long as the
	/// command.
	#[param]
	nested_args: Vec<String>,
	cx: ActionContext<Request>,
) -> Result<Response> {
	let seconds = AdminElevate::parse_duration(duration.as_deref())?;
	let session = match nested_args.is_empty() {
		// a session kept on tmpfs is fresh, sized as asked
		true => AdminSession::mint(seconds).await?,
		// a command runs in the live one when it has time to spare
		false => AdminSession::acquire(seconds).await?,
	};
	// the member and the session inside it, when one is named
	let member = match account.as_deref() {
		Some(name) => {
			let member =
				MemberAccount::find(session.credentials().clone(), name)
					.await?;
			let inside = AdminSession::new(
				member
					.assume(
						session.credentials().clone(),
						MemberAccount::ADMIN_ROLE,
					)
					.await?,
			)?;
			Some((member, inside))
		}
		None => None,
	};
	match nested_args.is_empty() {
		// one verb, nothing kept: the grant lives as long as the child
		false => {
			let route = nested_args.join(" ");
			let vars = match &member {
				Some((member, inside)) => {
					info!(
						"running `{route}` as `{}` in `{}` ({}), keeping nothing",
						MemberAccount::ADMIN_ROLE,
						member.name,
						member.id
					);
					inside.sdk_vars()
				}
				None => {
					info!(
						"running `{route}` as the `{}` role, keeping nothing",
						AgentIdentity::ADMIN_ROLE
					);
					session.sdk_vars()
				}
			};
			let status = vars
				.iter()
				.fold(
					// NOT `without_launch_env`: this child is THIS launch, so it
					// must resolve the same workspace and the same entry. The
					// only thing added is the role's credentials.
					ChildProcess::this_launch(&nested_args)?,
					|process, (_, value)| process.with_secret(value.clone()),
				)
				.without_env("AWS_PROFILE")
				.with_envs(vars.clone())
				.spawn()?
				.status()
				.await?;
			match status.success() {
				true => Response::ok().xok(),
				false => bevybail!(
					"`{route}` exited with {}",
					status.code().unwrap_or(-1)
				),
			}
		}
		true => {
			session.write()?;
			// the expiry and nothing else: the credential itself never prints
			let reach = match &member {
				Some((member, _)) => format!(
					"; reaches `{0}` ({1}), which `beet aws --account={0}` \
					chains into for each call\n",
					member.name, member.id
				),
				None => "\n".to_string(),
			};
			Response::ok_text(format!(
				"administrator until {} ({} from now), on tmpfs at {}{reach}",
				Timestamp::from_secs(session.expires()).format_iso8601(),
				AwsExec::describe_remaining(session.remaining_secs()),
				AdminSession::path()?.display()
			))
			.xok()
		}
	}
}

impl AdminSession {
	/// The grant a session is bought for unless asked otherwise, an hour:
	/// long enough for an elevated deploy, short enough that forgetting it
	/// costs nothing.
	pub const DEFAULT_SECS: i64 = 3600;

	/// The least a reused session must have left: an elevated deploy, a
	/// mail box replaced and provisioned, takes most of it.
	const REUSE_FLOOR: i64 = 20 * 60;

	/// An administrator session for one elevated command: the live one on
	/// tmpfs when `beet admin` left one with time to spare, else a fresh one
	/// of `seconds` bought with a code from the phone, asked for on the
	/// terminal and kept nowhere. Without a terminal the ask is the error.
	pub async fn acquire(seconds: i64) -> Result<Self> {
		if let Ok(SessionState::Live(session)) = Self::read()
			&& session.remaining_secs() >= Self::REUSE_FLOOR
		{
			return session.xok();
		}
		Self::mint(seconds).await
	}

	/// A fresh session of `seconds`: `beet-agent`'s pair from the global
	/// document, the six-digit code from the phone, and `sts:AssumeRole` on
	/// `beet-admin` with it.
	pub async fn mint(seconds: i64) -> Result<Self> {
		let sts = AwsSts::new(AgentIdentity::credentials().await?);
		let account = AdminElevate::account_id(&sts).await?;
		let code = OperatorPrompt::ask(AdminElevate::code_ask()).await?;
		AdminElevate::check_code(&code)?;
		AdminElevate::assume(&sts, &account, &code, seconds).await
	}
}

impl AdminElevate {
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
			return AdminSession::DEFAULT_SECS.xok();
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

	/// The ask for the six-digit code: a person's, so where no person can
	/// type it the refusal says to run the command in a terminal.
	fn code_ask() -> OperatorAsk {
		OperatorAsk::code(
			"an mfa code",
			format!(
				"six-digit code for the `{}` mfa device: ",
				AgentIdentity::USER
			),
			6,
		)
		.with_instead("run `beet admin` in a terminal, then retry")
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
	async fn account_id(sts: &AwsSts) -> Result<SmolStr> {
		sts.caller_identity()
			.await
			.map(|identity| identity.account)
			.map_err(|err| Self::explain(err, ""))
	}

	/// Trade the code for the role's temporary credentials.
	async fn assume(
		sts: &AwsSts,
		account: &str,
		code: &str,
		seconds: i64,
	) -> Result<AdminSession> {
		// the device is named for the user, so its arn is composed the same way
		let device =
			format!("arn:aws:iam::{account}:mfa/{}", AgentIdentity::USER);
		let credentials = sts
			.assume_role(&AssumeRole {
				role_arn: format!(
					"arn:aws:iam::{account}:role/{}",
					AgentIdentity::ADMIN_ROLE
				)
				.into(),
				session_name: AgentIdentity::ADMIN_ROLE.into(),
				duration_secs: seconds,
				mfa: Some((device.as_str().into(), code.into())),
			})
			.await
			.map_err(|err| Self::explain(err, &device))?;
		// the answer's own expiry rather than now plus the request, since STS
		// clamps the duration to the role's ceiling without saying so
		AdminSession::new(credentials)
	}

	/// Turn the three failures that actually happen into what to do about
	/// them, by the error code STS answered. An `AccessDenied` here is almost
	/// never a permissions problem: it is a code that has rolled, or a device
	/// that is not registered yet. A key STS does not recognize is a pair
	/// rolled at the account without the global document following it.
	fn explain(err: BevyError, device: &str) -> BevyError {
		let Some(error) = AwsError::of(&err) else {
			return err;
		};
		let text = err.to_string();
		match error.error_code() {
			"AccessDenied"
				if error.message().contains("MultiFactorAuthentication") =>
			{
				bevyhow!(
					"that code was not accepted. A code is valid for about \
					thirty seconds, so the usual cause is a stale one: wait for \
					the next and try again. {text}"
				)
			}
			"AccessDenied" => bevyhow!(
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
			),
			"InvalidClientTokenId" | "SignatureDoesNotMatch" => bevyhow!(
				"`{}`'s pair does not answer `sts:GetCallerIdentity`, so there \
				is nobody to elevate FROM: the key may have been rolled at the \
				account without the global document following it. {text}",
				AgentIdentity::USER
			),
			_ => err,
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

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

	/// STS whose every answer is `status` and `body`, keeping each request
	/// body it was sent.
	fn sts(
		status: StatusCode,
		body: &'static str,
	) -> (AwsSts, std::sync::Arc<std::sync::Mutex<Vec<String>>>) {
		let sent = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
		let seen = sent.clone();
		let client = AwsClient::new(AwsService::STS, "us-east-1")
			.with_credentials(AwsCredentials::new("AKID", "secret"))
			.with_transport(move |request| {
				let seen = seen.clone();
				Box::pin(async move {
					let sent = request.body.into_bytes().await?;
					seen.lock()
						.unwrap()
						.push(String::from_utf8(sent.to_vec())?);
					Response::from_status(status).with_body(body).xok()
				})
			});
		(AwsSts::from_client(client), sent)
	}

	/// The code rides the one signed request, beside the composed role and
	/// device arns, and the session keeps the answer's OWN expiry: STS clamps
	/// a duration to the role's ceiling without saying so, and a session that
	/// claims more than it has is a command that fails halfway.
	#[beet_core::test]
	async fn assumes_with_the_code() {
		let (sts, sent) = sts(
			StatusCode::OK,
			"<AssumeRoleResponse><AssumeRoleResult><Credentials>\
			<AccessKeyId>ASIAEXAMPLE</AccessKeyId>\
			<SecretAccessKey>secrethalf</SecretAccessKey>\
			<SessionToken>sessiontoken</SessionToken>\
			<Expiration>2026-09-30T12:00:00Z</Expiration>\
			</Credentials></AssumeRoleResult></AssumeRoleResponse>",
		);
		let session = AdminElevate::assume(&sts, "1234", "123456", 1800)
			.await
			.unwrap();
		session.expires().xpect_eq(
			Timestamp::parse_rfc3339("2026-09-30T12:00:00Z")
				.unwrap()
				.secs(),
		);
		sent.lock().unwrap()[0]
			.as_str()
			.xpect_contains(
				"RoleArn=arn%3Aaws%3Aiam%3A%3A1234%3Arole%2Fbeet-admin",
			)
			.xpect_contains(
				"SerialNumber=arn%3Aaws%3Aiam%3A%3A1234%3Amfa%2Fbeet-agent",
			)
			.xpect_contains("TokenCode=123456")
			.xpect_contains("DurationSeconds=1800");
	}

	/// A stale code reads as one, and never carries the code itself.
	#[beet_core::test]
	async fn a_stale_code_says_so() {
		let (sts, _) = sts(
			StatusCode::FORBIDDEN,
			"<ErrorResponse><Error><Type>Sender</Type><Code>AccessDenied</Code>\
			<Message>MultiFactorAuthentication failed with invalid MFA one \
			time pass code 123456</Message></Error></ErrorResponse>",
		);
		AdminElevate::assume(&sts, "1234", "123456", 1800)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("thirty seconds")
			.xnot()
			.xpect_contains("123456");
	}

	/// An unregistered device is the expected first failure, so it must read
	/// as an instruction rather than as a permissions bug, and a rolled key as
	/// the document lagging the account.
	#[beet_core::test]
	async fn explains_by_code() {
		let (denied, _) = sts(
			StatusCode::FORBIDDEN,
			"<ErrorResponse><Error><Code>AccessDenied</Code><Message>User is \
			not authorized to perform: sts:AssumeRole</Message></Error>\
			</ErrorResponse>",
		);
		AdminElevate::assume(&denied, "1234", "123456", 1800)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("Assign MFA device")
			.xpect_contains("unreachable by design");
		let (rolled, _) = sts(
			StatusCode::FORBIDDEN,
			"<ErrorResponse><Error><Code>InvalidClientTokenId</Code><Message>\
			The security token included in the request is invalid</Message>\
			</Error></ErrorResponse>",
		);
		AdminElevate::account_id(&rolled)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("rolled at the account");
	}

	/// A code is a person's: refused headless, and never asked of a provider
	/// no person stands behind.
	#[beet_core::test]
	fn a_headless_code_is_refused() {
		RefusalPrompt::new("this launch is headless")
			.refusal(&AdminElevate::code_ask())
			.to_string()
			.xpect_contains("an mfa code is typed by a person")
			.xpect_contains("run `beet admin` in a terminal");
	}
}
