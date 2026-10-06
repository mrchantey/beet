//! `admin`: an administrator session, bought with a code from a phone.

use crate::prelude::*;
use beet_core::prelude::*;
use std::path::PathBuf;

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
/// It is read from the terminal and handed to the `aws` child in a `0600` file
/// on tmpfs, never as an argument and never as an environment variable: argv is
/// readable by any process on the machine through `/proc/<pid>/cmdline`, an
/// environment is readable by its children, and a shell argument lands in
/// history. A pipe would be better still and the cli does not support one — see
/// [`write_request`](AdminElevate::write_request).
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
	match params.nested_args.is_empty() {
		// one verb, nothing kept: the grant lives as long as the child
		false => {
			let route = params.nested_args.join(" ");
			info!(
				"running `{route}` as the `{}` role, keeping nothing",
				AgentIdentity::ADMIN_ROLE
			);
			let status = session
				.sdk_vars()
				.iter()
				.fold(
					ChildProcess::new(AdminElevate::own_binary()?),
					|process, (_, value)| process.with_secret(value.clone()),
				)
				// NOT `without_launch_env`: this child is THIS launch, so it
				// must resolve the same workspace and the same entry. The only
				// thing added is the role's credentials.
				.without_env("AWS_PROFILE")
				.with_args(params.nested_args.iter().map(String::as_str))
				.with_envs(session.sdk_vars())
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
	/// This same binary, which is what `-- <verb>` runs.
	///
	/// The nested arguments are a beet ROUTE, not a command on `PATH`:
	/// `beet admin -- deployer/mint --stage=prod` is the documented mint path,
	/// and `deployer/mint` is a route this binary serves rather than a program
	/// anything could execute. Spawning the current executable also means the
	/// path works on a machine where `beet` was never installed onto `PATH`,
	/// which is most of them — a previous version spawned the first argument
	/// directly and died on `No such file or directory` AFTER the code had been
	/// typed and spent.
	fn own_binary() -> Result<String> {
		std::env::current_exe()
			.map(|path| path.to_string_lossy().to_string())
			.map_err(|err| {
				bevyhow!(
					"cannot find this binary to run `-- <verb>` with ({err}): \
					run `beet admin` on its own and the session will be \
					waiting for the next command"
				)
			})
	}

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
			// the agent pair carries no token of its own, so an inherited one
			// would make it authenticate as nobody
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
		let (request_path, request) =
			Self::write_request(&role, &device, code, seconds)?;
		let result = agent
			.iter()
			.fold(ChildProcess::new("aws"), |process, (_, value)| {
				process.with_secret(value.clone())
			})
			.without_launch_env()
			.without_env("AWS_PROFILE")
			// the agent pair carries no token of its own, so an inherited one
			// would make it authenticate as nobody
			.without_env("AWS_SESSION_TOKEN")
			.with_secret(code)
			.with_args([
				"sts",
				"assume-role",
				"--cli-input-json",
				request.as_str(),
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
			.await;
		// the request holds the code, so it goes the moment the call returns,
		// whichever way it went
		fs_ext::remove(&request_path)?;
		let body = result.map_err(|err| Self::explain(err, &device))?;
		Self::parse_credentials(&body, seconds)
	}

	/// Write the request where the cli can read it: a `0600` file on tmpfs,
	/// returning the path and the `file://` argument naming it.
	///
	/// **The cli cannot read `--cli-input-json` from a pipe.** `file:///dev/stdin`
	/// is rejected as `Invalid JSON received` even from a plain shell
	/// redirect, so the obvious way to keep the code out of argv is not
	/// available and this is the next one: `$XDG_RUNTIME_DIR` is tmpfs, so the
	/// code never reaches a disk, [`fs_ext::write_private`] creates the file
	/// owner-only before a byte of it exists, and the caller removes it as soon
	/// as the call returns.
	///
	/// Worth the file rather than `--token-code` on argv because argv is
	/// readable by every process on the machine through `/proc/<pid>/cmdline`,
	/// where this is readable by its owner alone. `with_secret` governs what
	/// beet writes down, not what the kernel shows about a child.
	fn write_request(
		role: &str,
		device: &str,
		code: &str,
		seconds: i64,
	) -> Result<(PathBuf, String)> {
		let path = AdminSession::path()?.with_file_name("assume-request.json");
		if let Some(parent) = path.parent() {
			fs_ext::create_dir_private(parent)?;
		}
		fs_ext::write_private(
			&path,
			Self::assume_request(role, device, code, seconds),
		)?;
		let arg = format!("file://{}", path.display());
		(path, arg).xok()
	}

	/// The `sts:AssumeRole` request as json. Hand-built rather than through
	/// `serde_json`, which this crate links only behind its `json` feature;
	/// every field is an arn, a fixed name or digits, so there is nothing to
	/// escape.
	fn assume_request(
		role: &str,
		device: &str,
		code: &str,
		seconds: i64,
	) -> String {
		format!(
			"{{\"RoleArn\":\"{role}\",\
			 \"RoleSessionName\":\"beet-admin\",\
			 \"SerialNumber\":\"{device}\",\
			 \"TokenCode\":\"{code}\",\
			 \"DurationSeconds\":{seconds}}}"
		)
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

	/// `-- <verb>` runs THIS binary, and the path it resolves has to exist: the
	/// previous version spawned the first argument as a program, so
	/// `beet admin -- deployer/mint` died on `No such file or directory` AFTER
	/// the code had been typed and spent. That is the expensive kind of bug,
	/// since the cost is a credential rather than a retry.
	#[beet_core::test]
	fn the_nested_verb_runs_this_binary() {
		let path = AdminElevate::own_binary().unwrap();
		std::path::Path::new(&path).is_file().xpect_true();
		// a route, not a program on PATH: `deployer/mint` is never executable
		std::path::Path::new("deployer/mint")
			.is_file()
			.xpect_false();
	}

	/// The DELIVERY, not just the string. The previous version of this verb
	/// built correct json and handed it to the cli as `file:///dev/stdin`,
	/// which the cli rejects as `Invalid JSON received` even from a plain shell
	/// redirect — so a test that only checked the json passed while the verb
	/// could not work at all, and the cost of finding out was a spent mfa code.
	/// This asserts the three properties that failure had: a `file://`
	/// argument, a file that EXISTS at that path, and owner-only permissions.
	#[beet_core::test]
	fn the_request_reaches_the_cli_as_a_private_file() {
		// no `XDG_RUNTIME_DIR` on this host means no tmpfs to write to
		if AdminSession::path().is_err() {
			return;
		}
		let (path, arg) = AdminElevate::write_request(
			"arn:aws:iam::1234:role/beet-admin",
			"arn:aws:iam::1234:mfa/beet-agent",
			"123456",
			1800,
		)
		.unwrap();
		arg.as_str()
			.xpect_starts_with("file://")
			// a pipe is what the cli cannot read, so it must not be one
			.xnot()
			.xpect_contains("/dev/stdin");
		fs_ext::exists(&path).unwrap().xpect_true();
		fs_ext::is_private(&path).unwrap().xpect_true();
		// and the code is in it, since that is the whole reason for the file
		fs_ext::read_to_string(&path)
			.unwrap()
			.as_str()
			.xpect_contains("\"TokenCode\":\"123456\"");
		fs_ext::remove(&path).unwrap();
	}

	/// The request carries the code, and the shape is worth pinning separately
	/// from the delivery above.
	#[beet_core::test]
	fn builds_the_assume_request() {
		let body = AdminElevate::assume_request(
			"arn:aws:iam::1234:role/beet-admin",
			"arn:aws:iam::1234:mfa/beet-agent",
			"123456",
			1800,
		);
		body.as_str()
			.xpect_contains("\"TokenCode\":\"123456\"")
			.xpect_contains("\"RoleArn\":\"arn:aws:iam::1234:role/beet-admin\"")
			.xpect_contains(
				"\"SerialNumber\":\"arn:aws:iam::1234:mfa/beet-agent\"",
			)
			.xpect_contains("\"DurationSeconds\":1800")
			.xpect_contains("\"RoleSessionName\":\"beet-admin\"");
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
