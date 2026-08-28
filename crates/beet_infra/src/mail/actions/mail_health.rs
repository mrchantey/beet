//! What a watcher checks before it starts reading logs.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// `<MailHealth/>` — assert the two surfaces the box is judged on from outside
/// it: the SMTP greeting a sending server sees, and the JMAP session a client
/// and an agent see.
///
/// Cheap and read-only, so it is what a `watch` runs before it starts tailing:
/// the reason to look at logs is almost always that one of these two is wrong,
/// and knowing WHICH before the first line scrolls past is most of the
/// diagnosis. It is not the [`MailProbe`], which sends real mail and takes
/// minutes; nothing here leaves the box's front door.
///
/// The banner is asserted to CARRY THE HOSTNAME rather than merely to exist. A
/// greeting naming anything else (`localhost`, an EC2 internal name) is the
/// single most common cause of a domain being greylisted into oblivion, and it
/// is invisible from the inside: the server is up, mail is queued, and
/// deliveries simply take hours.
#[derive(Debug, Clone, Get, SetWith, Component, Reflect)]
#[reflect(Component, Default)]
#[require(MailHealthAction)]
pub struct MailHealth {
	/// How long to wait on each check. Short: this is a liveness question, and
	/// a slow answer is itself the finding.
	timeout: Duration,
}

impl Default for MailHealth {
	fn default() -> Self {
		Self {
			timeout: Duration::from_secs(30),
		}
	}
}

impl MailHealth {
	/// The port a peer MTA dials, which is the one whose greeting is a
	/// reputation input.
	pub const SMTP_PORT: u16 = 25;

	/// The SMTP reply code that opens a session. Anything else at all — a
	/// `421`, a `554` — is a server that is listening and refusing.
	pub const READY_CODE: &'static str = "220";
}

/// Runs both checks, reporting each and failing on the first that does not
/// hold.
#[action(handler_only)]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn MailHealthAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let health = cx
		.caller
		.get_cloned::<MailHealth>()
		.await
		.unwrap_or_default();
	let mail = cx
		.caller
		.with_state::<MailQuery, _>(|entity, query| query.resolve(entity))
		.await??;
	let hostname = mail.mail_box.hostname().to_string();

	check_banner(&hostname, *health.timeout()).await?;
	check_jmap(&hostname).await?;
	Ok(Pass(cx.input))
}

/// Open an SMTP session and read the greeting, which `curl` does for a plain
/// `smtp://` url: connect, `EHLO`, `QUIT`, nothing sent.
///
/// `curl` rather than a socket of our own because the deploy already depends on
/// it for the probe's submission leg, and because the check that matters is the
/// one a real client's stack performs.
async fn check_banner(hostname: &str, timeout: Duration) -> Result {
	let output = ChildProcess::new("curl")
		.with_args([
			"--silent".to_string(),
			"--show-error".to_string(),
			"--verbose".to_string(),
			"--max-time".to_string(),
			timeout.as_secs().to_string(),
			format!("smtp://{hostname}:{}", MailHealth::SMTP_PORT),
		])
		.run_async()
		.await
		.map_err(|err| {
			bevyhow!(
				"{hostname}:{} did not open an SMTP session, so no mail is \
				arriving at all. {err}",
				MailHealth::SMTP_PORT
			)
		})?;
	// the greeting arrives on the trace stream, prefixed `< ` like every other
	// line curl reads from the peer
	let banner = String::from_utf8_lossy(&output.stderr)
		.lines()
		.find_map(|line| {
			line.trim_start_matches("< ")
				.starts_with(MailHealth::READY_CODE)
				.then(|| line.trim_start_matches("< ").trim().to_string())
		})
		.ok_or_else(|| {
			bevyhow!(
				"{hostname}:{} answered without a {} greeting",
				MailHealth::SMTP_PORT,
				MailHealth::READY_CODE
			)
		})?;
	if !banner.contains(hostname) {
		bevybail!(
			"the SMTP greeting is `{banner}`, which does not name {hostname}: \
			a receiving server compares this to the reverse record, and a \
			mismatch is scored as spam long before anybody notices"
		);
	}
	info!("smtp banner: {banner}");
	Ok(())
}

/// Ask for the JMAP session document, unauthenticated: the question is whether
/// the box serves a trusted certificate on 443 and answers at all, not whether
/// this deployer may sign in, so a `401` is a pass and only a transport failure
/// or a `5xx` is not.
async fn check_jmap(hostname: &str) -> Result {
	let url = format!("https://{hostname}{}", JmapClient::SESSION_PATH);
	match Request::get(&url).send().await {
		Ok(response) if response.status().as_u16() < 500 => {
			info!("jmap answering on 443 ({})", response.status());
			Ok(())
		}
		Ok(response) => bevybail!(
			"{url} answered {}: the server is reachable and failing",
			response.status()
		),
		Err(err) => bevybail!(
			"{url} is unreachable, which is the certificate, the https \
			listener or dns. {err}"
		),
	}
}
