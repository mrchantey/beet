//! What a watcher checks before it starts reading logs.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde_json::Value;




impl MailHealth {
	/// The SMTP reply code that opens a session. Anything else at all — a
	/// `421`, a `554` — is a server that is listening and refusing.
	pub const READY_CODE: &'static str = "220";
}

/// Runs both checks, reporting each and failing on the first that does not
/// hold.
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
#[action]
#[derive(Component, Reflect)]
#[reflect(Component, Default)]
pub async fn MailHealth(
	/// How long to wait on each check. Short: this is a liveness question, and
	/// a slow answer is itself the finding.
	#[field(default = Duration::from_secs(30))]
	timeout: Duration,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let mail = cx.caller.with_world(MailStack::resolve).await??;
	let hostname = mail.mail_box.hostname().to_string();

	check_banner(&hostname, timeout).await?;
	check_jmap(&hostname).await?;
	if mail.mail_box.dane() {
		check_dane(&mail.mail_box, timeout).await?;
	}
	Ok(Pass(cx.input))
}

/// The published pin against the served key: the `TLSA` at
/// `_25._tcp.<hostname>` over DNS-over-HTTPS, which must be signed (`AD`,
/// since an unsigned pin is ignored by every validator) and must name the
/// SHA-256 of the key 443 serves for the hostname.
///
/// 443 rather than 25 because 25 is unreachable from most deploy machines,
/// and the two ports serve one certificate for one SNI name; the on-box read
/// of 25 itself is [`MailDane`]'s, at deploy. A mismatch here is the failure
/// DANE turns into refused sessions, and the fix is a deploy.
async fn check_dane(mail_box: &StalwartBlock, timeout: Duration) -> Result {
	let name = mail_box.tlsa_record_name();
	let response = Request::get(format!(
		"{}?name={name}&type=TLSA",
		EipReverseDns::RESOLVER
	))
	.with_accept(MediaType::other("application/dns-json"))
	.send()
	.await?;
	if !response.status().is_ok() {
		bevybail!("dns lookup for {name} failed: {}", response.status());
	}
	let body: Value = response.json().await?;
	let published = tlsa_pins(&body);
	if published.is_empty() {
		bevybail!(
			"{name} publishes no TLSA, but the box declares `dane=true`: the \
			pin is parked by `MailDane` and published by the apply after it, so \
			a deploy is what is missing"
		);
	}
	if body["AD"] != Value::Bool(true) {
		bevybail!(
			"{name} is not DNSSEC-signed (no AD flag), so its pin is ignored \
			by every validator: DNSSEC on the zone is the precondition"
		);
	}
	let served = served_pin(mail_box.hostname(), timeout).await?;
	if !published.contains(&served) {
		bevybail!(
			"{name} pins {published:?} but the box serves {served}: a \
			DANE-validating sender is refusing delivery, and a deploy re-pins \
			the served key"
		);
	}
	info!("dane: {name} pins the served key ({served})");
	Ok(())
}

/// The digests a DNS-over-HTTPS answer's `3 1 1` records carry.
///
/// Resolvers differ on the presentation: Google writes `3 1 1 f0d3…` and
/// Cloudflare `3 1 1 ( F0D3… )`, so the digest is whatever hex remains once
/// the triple, the brackets and the case are gone.
fn tlsa_pins(body: &Value) -> Vec<String> {
	let (usage, selector, matching) = StalwartBlock::TLSA_PARAMS;
	let prefix = format!("{usage} {selector} {matching} ");
	body["Answer"]
		.as_array()
		.into_iter()
		.flatten()
		.filter(|answer| answer["type"] == 52)
		.filter_map(|answer| answer["data"].as_str())
		.filter_map(|data| data.strip_prefix(&prefix))
		.map(|digest| {
			digest
				.chars()
				.filter(char::is_ascii_hexdigit)
				.collect::<String>()
				.to_lowercase()
		})
		.collect()
}

/// The SHA-256 of the key `hostname` serves on 443, through the same pipeline
/// [`MailDane`] runs on the box.
async fn served_pin(hostname: &str, timeout: Duration) -> Result<String> {
	let output = ChildProcess::new("sh")
		.with_args([
			"-c".to_string(),
			format!(
				"echo | timeout {} openssl s_client -connect {hostname}:443 \
				-servername {hostname} 2>/dev/null | {}",
				timeout.as_secs(),
				MailDane::SPKI_SHA256
			),
		])
		.run_async_stdout()
		.await?;
	let digest = output.trim().to_string();
	if digest.len() != 64 {
		bevybail!(
			"{hostname}:443 presented no parseable certificate (got {digest:?})"
		);
	}
	Ok(digest)
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
			format!("smtp://{hostname}:{}", StalwartBlock::SMTP_PORT),
		])
		.run_async()
		.await
		.map_err(|err| {
			bevyhow!(
				"{hostname}:{} did not open an SMTP session, so no mail is \
				arriving at all. {err}",
				StalwartBlock::SMTP_PORT
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
				StalwartBlock::SMTP_PORT,
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

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	/// Only `3 1 1` answers are pins this box publishes; another usage at the
	/// same name is somebody else's record and must not read as a match.
	#[beet_core::test]
	fn only_the_published_shape_counts_as_a_pin() {
		let body = json!({
			"AD": true,
			"Answer": [
				{ "type": 52, "data": "3 1 1 ABCD" },
				// Cloudflare's presentation, brackets and all
				{ "type": 52, "data": "3 1 1 ( F0D3 )" },
				{ "type": 52, "data": "2 1 1 ffff" },
				{ "type": 46, "data": "TLSA 13 5 300 ..." }
			]
		});
		tlsa_pins(&body).xpect_eq(vec!["abcd".to_string(), "f0d3".to_string()]);
		tlsa_pins(&json!({ "Status": 0 })).is_empty().xpect_true();
	}
}
