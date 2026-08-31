//! The end-to-end assertion that the mail stack actually carries mail.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde_json::Value;
use serde_json::json;

/// `<MailProbe/>` — send a message out through the whole stack and receive one
/// back through it, then assert the receiving end believes both.
///
/// Two half-loops rather than one round trip, because the two legs prove
/// different things and neither can be inferred from the other:
///
/// - **Outbound**: authenticate to the box's own submission port as the probe
///   mailbox and send to `success@simulator.amazonses.com`. This proves the
///   submission listener bound, the certificate is trusted, the account exists
///   with the credential parameter store says it has, and the queue handed the
///   message to SES rather than dialling an MX itself.
/// - **Inbound**: ask SES to send from the publication domain to the probe
///   mailbox, then read it out over JMAP. This proves the `MX` resolves to the
///   box, that port 25 accepts, that the message reached a mailbox, and — the
///   part no other check reaches — that a receiving server evaluating our own
///   `SPF`, `DKIM` and `DMARC` records passes all three.
///
/// The `Authentication-Results` assertion is the point of the whole action. A
/// deliverable mail stack is not one that sends, it is one whose mail
/// authenticates, and the only honest way to test that is to be the recipient.
///
/// Works inside the SES sandbox: the simulator addresses are always permitted
/// as recipients, and the probe mailbox is verified as a recipient identity
/// once, which needs no production access.
#[derive(Debug, Clone, Get, SetWith, Component, Reflect)]
#[reflect(Component, Default)]
#[require(MailProbeAction)]
pub struct MailProbe {
	/// The mailbox both legs use, by localpart. Declared as a [`Mailbox`] on
	/// exactly one of the stack's domains, which is how the probe finds the
	/// address without restating the domain that holds it.
	mailbox: SmolStr,
	/// The domain the inbound leg is sent FROM, ie the publication domain whose
	/// deliverability this proves. Named rather than guessed: which domain
	/// sends is a decision.
	sender_domain: SmolStr,
	/// The localpart on [`sender_domain`](Self::sender_domain) the inbound leg
	/// comes from.
	sender: SmolStr,
	/// How long to wait for the inbound message to arrive.
	timeout: Duration,
	/// The gap between mailbox polls.
	poll: Duration,
}

impl Default for MailProbe {
	fn default() -> Self { Self::new("probe", "") }
}

impl MailProbe {
	/// The SES address that accepts anything and delivers nowhere, so an
	/// outbound probe costs no reputation and reaches no human.
	pub const SIMULATOR: &'static str = "success@simulator.amazonses.com";

	/// The authentication verdicts the inbound leg must carry. Every one of
	/// them is a record this stack publishes, so a failure names the record to
	/// look at.
	pub const REQUIRED_RESULTS: &'static [&'static str] =
		&["spf=pass", "dkim=pass", "dmarc=pass"];

	pub fn new(
		mailbox: impl Into<SmolStr>,
		sender_domain: impl Into<SmolStr>,
	) -> Self {
		Self {
			mailbox: mailbox.into(),
			sender_domain: sender_domain.into(),
			sender: "news".into(),
			timeout: Duration::from_secs(300),
			poll: Duration::from_secs(10),
		}
	}

	/// The subject of one probe run, carrying a token so the poll reads THIS
	/// run's message rather than the previous deploy's.
	pub fn subject(token: &str) -> String { format!("beet mail probe {token}") }

	/// Whether `results` (an `Authentication-Results` header) reports every
	/// verdict in [`REQUIRED_RESULTS`](Self::REQUIRED_RESULTS).
	pub fn authenticates(results: &str) -> Vec<&'static str> {
		let normalized = results.to_ascii_lowercase().replace(' ', "");
		Self::REQUIRED_RESULTS
			.iter()
			.filter(|required| !normalized.contains(*required))
			.copied()
			.collect()
	}
}

/// Runs both legs, failing on the first one that does not hold.
#[action(handler_only)]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn MailProbeAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let probe = cx.caller.get_cloned::<MailProbe>().await?;
	let mail = cx.caller.with_world(MailStack::resolve).await??;

	let domain = mail.domain_holding(probe.mailbox())?;
	let address = format!("{}@{}", probe.mailbox(), domain.domain());
	let sender_domain = mail.domain_named(probe.sender_domain())?;
	let from = format!("{}@{}", probe.sender(), sender_domain.domain());

	// the same parameter `StalwartProvision` minted the account with, so the
	// probe never holds a credential of its own.
	let secret = AccountPlan::secret_ref(
		mail.mail_box.label(),
		probe.mailbox(),
		&domain.slug(),
	);
	let region = mail.stack.region().clone();
	let password = ssm_ext::get(&region, &secret.name(&mail.stack))
		.await?
		.ok_or_else(|| {
			bevyhow!(
				"no credential for {address}: <StalwartProvision/> mints it, so \
				run the provision before the probe"
			)
		})?;

	// one token per run, so both legs and the mailbox poll agree on which
	// message this is.
	let token = EnsureSecret::new(secret.clone())
		.with_length(16)
		.generate()?;

	send_outbound(&mail, &address, &password, &token, &mail.project.work_dir())
		.await?;
	send_inbound(&region, sender_domain, &from, &address, &token).await?;
	assert_inbound(
		&mail,
		&address,
		&password,
		&token,
		*probe.timeout(),
		*probe.poll(),
	)
	.await?;

	info!(
		"mail probe passed: {address} sends through SES and receives authenticated mail"
	);
	Pass(cx.input).xok()
}

/// The outbound leg: submit as the probe mailbox over implicit TLS on 465.
///
/// `curl` rather than a client of our own, since SMTP submission with implicit
/// TLS and AUTH is a protocol we would otherwise implement to send one message.
/// It also means the probe exercises the listener exactly as a real mail client
/// does, TLS verification included.
async fn send_outbound(
	mail: &MailStack,
	address: &str,
	password: &str,
	token: &str,
	work_dir: &AbsPathBuf,
) -> Result {
	let host = mail.mail_box.hostname();
	let body = format!(
		"From: <{address}>\r\n\
		To: <{}>\r\n\
		Subject: {}\r\n\
		\r\n\
		Outbound leg of the beet mail probe.\r\n",
		MailProbe::SIMULATOR,
		MailProbe::subject(token)
	);
	let message = work_dir.join("mail-probe-outbound.eml");
	fs_ext::write_async(&message, body.as_bytes()).await?;

	info!("submitting to {host}:465 as {address}");
	ChildProcess::new("curl")
		.with_args([
			"--silent".to_string(),
			"--show-error".to_string(),
			"--fail".to_string(),
			format!("smtps://{host}:465"),
			"--mail-from".to_string(),
			address.to_string(),
			"--mail-rcpt".to_string(),
			MailProbe::SIMULATOR.to_string(),
			"--user".to_string(),
			format!("{address}:{password}"),
			"--upload-file".to_string(),
			message.display().to_string(),
		])
		.with_secret(password)
		.run_async()
		.await
		.map_err(|err| {
			bevyhow!(
				"the outbound leg failed: {host} did not accept a submission \
				from {address}. {err}"
			)
		})?;
	info!("relay accepted the outbound probe");
	Ok(())
}

/// The inbound leg: ask SES to send a message the box must accept and
/// authenticate.
async fn send_inbound(
	region: &str,
	sender_domain: &MailDomainBlock,
	from: &str,
	address: &str,
	token: &str,
) -> Result {
	info!("asking SES to send {from} -> {address}");
	ChildProcess::new("aws")
		.without_env("AWS_PROFILE")
		.with_args([
			"sesv2".to_string(),
			"send-email".to_string(),
			"--from-email-address".to_string(),
			from.to_string(),
			"--destination".to_string(),
			format!("ToAddresses={address}"),
			// through the domain's own configuration set, so a bounce or a
			// complaint on the probe lands in the same event stream as real
			// mail rather than silently nowhere.
			"--configuration-set-name".to_string(),
			sender_domain.configuration_set_name(),
			"--content".to_string(),
			json!({
				"Simple": {
					"Subject": { "Data": MailProbe::subject(token) },
					"Body": { "Text": { "Data": "Inbound leg of the beet mail probe." } },
				}
			})
			.to_string(),
			"--region".to_string(),
			region.to_string(),
		])
		.run_async()
		.await?;
	Ok(())
}

/// Poll the mailbox over JMAP until this run's message arrives, then assert
/// what the box's own authentication checks concluded about it.
async fn assert_inbound(
	mail: &MailStack,
	address: &str,
	password: &str,
	token: &str,
	timeout: Duration,
	poll: Duration,
) -> Result {
	let client = JmapClient::connect(
		format!("https://{}", mail.mail_box.hostname()),
		address,
		password,
	)
	.await?;
	let account = client.mail_account()?.to_string();
	let subject = MailProbe::subject(token);
	let attempts = (timeout.as_secs() / poll.as_secs().max(1)).max(1);

	for attempt in 1..=attempts {
		if let Some(results) =
			find_probe_message(&client, &account, &subject).await?
		{
			info!("the inbound probe arrived after {attempt} attempt(s)");
			let missing = MailProbe::authenticates(&results);
			if !missing.is_empty() {
				bevybail!(
					"the inbound probe arrived but did not authenticate \
					({}): Authentication-Results was `{results}`",
					missing.join(", ")
				);
			}
			info!("spf, dkim and dmarc all pass on inbound mail");
			return Ok(());
		}
		time_ext::sleep(poll).await;
	}
	bevybail!(
		"the inbound probe never arrived at {address}: the MX, the box's port \
		25, or the delivery into the mailbox is where to look"
	)
}

/// This run's message, as its `Authentication-Results` header, or `None` while
/// it has not arrived.
async fn find_probe_message(
	client: &JmapClient,
	account: &str,
	subject: &str,
) -> Result<Option<String>> {
	let query = client
		.call_mail(
			"Email/query",
			json!({
				"accountId": account,
				"filter": { "subject": subject },
				"limit": 1,
			}),
		)
		.await?;
	let Some(id) = query["ids"][0].as_str() else {
		return Ok(None);
	};
	// the header as text, since the verdicts are what matters and not the
	// structure a parsed form would give
	let get = client
		.call_mail(
			"Email/get",
			json!({
				"accountId": account,
				"ids": [id],
				"properties": ["subject", "header:Authentication-Results:asText"],
			}),
		)
		.await?;
	authentication_results(&get).xmap(Some).xok()
}

/// The `Authentication-Results` header out of an `Email/get` response, empty
/// when the message carries none (which is itself a failure the caller
/// reports, since the box adds one to everything it accepts).
fn authentication_results(response: &Value) -> String {
	response["list"][0]["header:Authentication-Results:asText"]
		.as_str()
		.unwrap_or_default()
		.to_string()
}

#[cfg(test)]
mod tests {
	use super::*;

	/// All three verdicts or the probe fails, and the error names the ones that
	/// did not hold so the reader knows which record to look at.
	#[beet_core::test]
	fn every_verdict_must_pass() {
		MailProbe::authenticates(
			"mx.beetmash.com; spf=pass smtp.mailfrom=bounce.news.beetmash.com; \
			dkim=pass header.i=@news.beetmash.com; dmarc=pass header.from=news.beetmash.com",
		)
		.is_empty()
		.xpect_true();
	}

	/// A partial pass is the interesting failure: DKIM alone still delivers to
	/// plenty of receivers, which is exactly why a probe that accepted it would
	/// let a broken SPF record ship.
	#[beet_core::test]
	fn a_partial_pass_is_a_failure() {
		MailProbe::authenticates(
			"mx; spf=fail; dkim=pass header.i=@news.beetmash.com; dmarc=fail",
		)
		.xpect_eq(vec!["spf=pass", "dmarc=pass"]);
		MailProbe::authenticates("").xpect_eq(vec![
			"spf=pass",
			"dkim=pass",
			"dmarc=pass",
		]);
	}

	/// Receivers format the header freely, so the match is whitespace- and
	/// case-insensitive rather than an exact string.
	#[beet_core::test]
	fn the_header_is_matched_loosely() {
		MailProbe::authenticates("SPF = PASS; DKIM = PASS; DMARC = PASS")
			.is_empty()
			.xpect_true();
	}

	/// Each run carries its own token, or the poll would find the previous
	/// deploy's message and pass without either leg having run.
	#[beet_core::test]
	fn each_run_has_its_own_subject() {
		let subject = MailProbe::subject("abc123");
		subject.as_str().xpect_contains("abc123");
		(subject != MailProbe::subject("def456")).xpect_true();
	}

	/// The header is absent from a message the box did not authenticate, which
	/// the caller reports as a failure rather than reading as a pass.
	#[beet_core::test]
	fn a_message_without_the_header_reads_as_empty() {
		authentication_results(
			&serde_json::json!({ "list": [{ "subject": "x" }] }),
		)
		.is_empty()
		.xpect_true();
	}
}
