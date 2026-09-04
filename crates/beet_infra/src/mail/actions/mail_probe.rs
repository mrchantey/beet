//! The end-to-end assertion that the mail stack actually carries mail.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde_json::Value;
use serde_json::json;

/// `<MailProbe/>`: send a message out through the whole stack and receive one
/// back through it, then assert the receiving end believes both.
///
/// Two half-loops rather than one round trip, because the two legs prove
/// different things and neither can be inferred from the other:
///
/// - **Outbound**: authenticate to the box's own submission port as the probe
///   mailbox and send to the sending domain's relay sink. This proves the
///   submission listener bound, the certificate is trusted, the account exists
///   with the credential parameter store says it has, and the queue took the
///   route its sender domain declares.
/// - **Inbound**: ask the relay to send from the publication domain to the probe
///   mailbox, then read it out over JMAP. This proves the `MX` resolves to the
///   box, that port 25 accepts, that the message reached a mailbox, and (the
///   part no other check reaches) that a receiving server evaluating our own
///   `SPF`, `DKIM` and `DMARC` records reaches the verdicts the sending domain's
///   relay makes possible.
///
/// The `Authentication-Results` assertion is the point of the whole action. A
/// deliverable mail stack is not one that sends, it is one whose mail
/// authenticates, and the only honest way to test that is to be the recipient.
///
/// Both legs and the verdicts they demand are decided by the SENDER domain's
/// relay, since that is what decides what a receiver can see. See
/// [`sink`](Self::sink) and [`required_results`](Self::required_results).
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

impl MailProbe {
	/// How many times the comail send api is asked before the probe gives up,
	/// counting only the answers worth re-asking (a `5xx`).
	pub const SEND_ATTEMPTS: usize = 3;

	/// The gap between those attempts.
	pub const SEND_RETRY: Duration = Duration::from_secs(5);
}

/// What a relay's send api answering `status` means for the probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendOutcome {
	/// The message was accepted: assert on what arrives.
	Sent,
	/// Refused for a reason that says nothing about this stack, so the
	/// assertion is skipped and the deploy passes.
	Skip,
	/// Worth asking again.
	Retry,
	/// The enrolment or the request is wrong, and asking again will not change
	/// that.
	Failed,
}

impl SendOutcome {
	/// Read a send api's status.
	///
	/// The one interesting case is `429`. Comail's warming curve caps a fresh
	/// member at two messages an hour for its first week
	/// (`internal/relay/warming.go:41-51`), so a deploy during warming can be
	/// legally refused, and comail's own smoke test treats exactly this as a
	/// skip rather than a regression (`cmd/smoke-test/main.go:30-36`). Every
	/// other `4xx` is terminal: `DOMAIN_MISMATCH` and `AUTH_REQUIRED` are the
	/// enrolment being wrong, and re-asking answers the same.
	pub fn of(status: StatusCode) -> Self {
		match status {
			status if status.is_ok() => Self::Sent,
			StatusCode::TOO_MANY_REQUESTS => Self::Skip,
			status if status.is_client_error() => Self::Failed,
			_ => Self::Retry,
		}
	}
}

impl Default for MailProbe {
	fn default() -> Self { Self::new("probe", "") }
}

impl MailProbe {
	/// The SES address that accepts anything and delivers nowhere, so an
	/// outbound probe costs no reputation and reaches no human.
	pub const SIMULATOR: &'static str = "success@simulator.amazonses.com";

	/// The verdicts every mode's inbound leg must carry: a signature this stack
	/// holds the key for, and a DMARC pass built on it.
	pub const REQUIRED_RESULTS: &'static [&'static str] =
		&["dkim=pass", "dmarc=pass"];

	/// The verdict that only holds where the envelope sender is a domain we
	/// publish SPF for.
	pub const ALIGNED_SPF: &'static str = "spf=pass";

	/// The blackhole an outbound probe is addressed to: the relay's own, else
	/// comail's smoke sink, whose whole purpose is being reachable over a real
	/// MX and dropping what arrives.
	///
	/// A direct-delivering domain borrows comail's, deliberately: a direct send
	/// to a THIRD PARTY over a real MX lookup is exactly what direct mode has
	/// to prove, and a sink that accepts and drops is the only kind that can be
	/// probed on every deploy without costing somebody a mailbox.
	pub fn sink(relay: &RelayMode) -> String {
		match relay {
			RelayMode::Ses(_) => Self::SIMULATOR.to_string(),
			RelayMode::Comail(comail) => comail.sink(),
			RelayMode::None => ComailRelay::default().sink(),
		}
	}

	/// The authentication verdicts the inbound leg must carry, per the SENDER
	/// domain's relay. Every one of them is a record this stack publishes, so a
	/// failure names the record to look at.
	///
	/// Aligned SPF is required of every mode BUT comail, and its absence there
	/// is by design rather than a gap: comail rewrites the envelope sender per
	/// recipient to a VERP address at its own domain
	/// (`internal/relay/queue.go:622-633`), so the domain SPF is evaluated
	/// against is never the header From's and can never align. DMARC passes on
	/// the DKIM signature instead, which is why that half is not negotiable.
	/// A receiver still REPORTS an `spf=pass` for `atmos.email`, so the check
	/// is dropped rather than inverted: what a comail probe must not do is
	/// treat that unaligned pass as proof of anything.
	pub fn required_results(relay: &RelayMode) -> Vec<&'static str> {
		match relay {
			RelayMode::Comail(_) => Self::REQUIRED_RESULTS.to_vec(),
			RelayMode::None | RelayMode::Ses(_) => Self::REQUIRED_RESULTS
				.iter()
				.copied()
				.chain([Self::ALIGNED_SPF])
				.collect(),
		}
	}

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

	/// Which of `relay`'s [`required_results`](Self::required_results) the
	/// `Authentication-Results` header `results` does NOT report.
	pub fn authenticates(
		relay: &RelayMode,
		results: &str,
	) -> Vec<&'static str> {
		let normalized = results.to_ascii_lowercase().replace(' ', "");
		Self::required_results(relay)
			.into_iter()
			.filter(|required| !normalized.contains(required))
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

	// both legs are the SENDER domain's, since its relay is what decides how
	// the message leaves and what a receiver can conclude about it
	let relay = mail.relay(sender_domain).clone();
	send_outbound(
		&mail,
		&relay,
		&address,
		&password,
		&token,
		&mail.project.work_dir(),
	)
	.await?;
	if !send_inbound(
		&mail,
		&relay,
		&region,
		sender_domain,
		&from,
		&address,
		&token,
	)
	.await?
	{
		return Pass(cx.input).xok();
	}
	assert_inbound(
		&mail,
		&relay,
		&address,
		&password,
		&token,
		*probe.timeout(),
		*probe.poll(),
	)
	.await?;

	info!(
		"mail probe passed: {address} sends through {} and receives \
		authenticated mail",
		relay.label()
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
	relay: &RelayMode,
	address: &str,
	password: &str,
	token: &str,
	work_dir: &AbsPathBuf,
) -> Result {
	let host = mail.mail_box.hostname();
	let sink = MailProbe::sink(relay);
	let body = format!(
		"From: <{address}>\r\n\
		To: <{sink}>\r\n\
		Subject: {}\r\n\
		\r\n\
		Outbound leg of the beet mail probe.\r\n",
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
			sink.clone(),
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
	info!("the outbound probe to {sink} was accepted for delivery");
	Ok(())
}

/// The inbound leg: ask the sending domain's relay to send a message the box
/// must accept and authenticate.
///
/// `false` when the send was refused for a reason that says nothing about this
/// stack, ie a warming-tier rate limit, in which case the caller skips the
/// assertion rather than failing a deploy on somebody else's quota.
async fn send_inbound(
	mail: &MailStack,
	relay: &RelayMode,
	region: &str,
	sender_domain: &MailDomainBlock,
	from: &str,
	address: &str,
	token: &str,
) -> Result<bool> {
	info!("asking {} to send {from} -> {address}", relay.label());
	match relay {
		RelayMode::Ses(_) => {
			send_inbound_ses(region, sender_domain, from, address, token)
				.await?;
			Ok(true)
		}
		RelayMode::Comail(comail) => {
			send_inbound_comail(
				mail,
				comail,
				region,
				sender_domain,
				from,
				address,
				token,
			)
			.await
		}
		// nothing external to ask: the message is submitted through the box's
		// own port as the sending domain, so it takes the local route and
		// never leaves. The leg still proves the submission listener accepts
		// as this domain and that delivery into the mailbox works, and it
		// proves NOTHING about how a remote receiver would judge the mail,
		// because no remote receiver is involved. The outbound leg above is
		// where direct delivery is actually exercised.
		RelayMode::None => {
			warn!(
				"'{}' has no relay, so the inbound leg is submitted locally \
				and its authentication verdicts are this box judging itself",
				sender_domain.domain()
			);
			send_inbound_local(mail, region, from, address, token).await?;
			Ok(true)
		}
	}
}

/// The SES arm: the api that already holds the sending identity.
async fn send_inbound_ses(
	region: &str,
	sender_domain: &MailDomainBlock,
	from: &str,
	address: &str,
	token: &str,
) -> Result {
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

/// The comail arm: the HTTP send api, authenticated with the enrolled pair.
///
/// A `429` is a neutral SKIP rather than a failure. The warming curve caps a
/// fresh member at two messages an hour for its first week
/// (`internal/relay/warming.go:41-51`), so a deploy during warming can legally
/// be refused, and comail's own smoke test treats exactly this case as a skip
/// (`cmd/smoke-test/main.go:30-36`). Every other `4xx` is terminal, because
/// `DOMAIN_MISMATCH` and `AUTH_REQUIRED` are the enrolment being wrong.
async fn send_inbound_comail(
	mail: &MailStack,
	comail: &ComailRelay,
	region: &str,
	sender_domain: &MailDomainBlock,
	from: &str,
	address: &str,
	token: &str,
) -> Result<bool> {
	let slug = sender_domain.slug();
	let credential = |secret: SecretRef| {
		let name = secret.name(&mail.stack);
		async move {
			ssm_ext::get(region, &name).await?.ok_or_else(|| {
				bevyhow!(
					"{name} does not exist, so the probe cannot send as \
					'{}': run <ComailEnroll/>",
					sender_domain.domain()
				)
			})
		}
	};
	let did = credential(ComailRelay::did_secret(&slug)).await?;
	let api_key = credential(ComailRelay::api_key_secret(&slug)).await?;

	// retried on 5xx only: a relay restarting is not a stack fault, and a 4xx
	// will answer the same however many times it is asked.
	for attempt in 1..=MailProbe::SEND_ATTEMPTS {
		let response = Request::post(comail.send_url())
			.with_auth_bearer(&api_key)
			.with_header_raw("x-atmos-did", &did)
			.with_json_body(&json!({
				"from": from,
				"to": [address],
				"subject": MailProbe::subject(token),
				"text": "Inbound leg of the beet mail probe.",
			}))?
			.send()
			.await?;
		let status = response.status();
		let body = response.text().await.unwrap_or_default();
		match SendOutcome::of(status) {
			SendOutcome::Sent => return Ok(true),
			SendOutcome::Skip => {
				warn!(
					"comail rate-limited the probe ({body}), which during the \
					14-day warming window is the expected answer rather than a \
					fault: skipping the inbound assertion"
				);
				return Ok(false);
			}
			SendOutcome::Failed => bevybail!(
				"comail refused the inbound probe ({status}): {body}. A \
				`DOMAIN_MISMATCH` means the key's enrolled domain is not \
				'{}', which is one domain per api key.",
				sender_domain.domain()
			),
			SendOutcome::Retry => {
				warn!("comail answered {status} on attempt {attempt}: {body}")
			}
		}
		time_ext::sleep(MailProbe::SEND_RETRY).await;
	}
	bevybail!(
		"comail never accepted the inbound probe after {} attempts",
		MailProbe::SEND_ATTEMPTS
	)
}

/// The direct arm: submit through the box's own port as the sending domain,
/// which is the only send path a stack with no relay has.
async fn send_inbound_local(
	mail: &MailStack,
	region: &str,
	from: &str,
	address: &str,
	token: &str,
) -> Result {
	let localpart = from.split('@').next().unwrap_or_default();
	let sender_domain = from.split('@').nth(1).unwrap_or_default();
	let secret = AccountPlan::secret_ref(
		mail.mail_box.label(),
		localpart,
		&MailDomainBlock::slug_of(sender_domain),
	);
	let name = secret.name(&mail.stack);
	let password = ssm_ext::get(region, &name).await?.ok_or_else(|| {
		bevyhow!(
			"no credential for {from}: a direct-delivering stack sends its \
			own inbound leg, so '{localpart}' must be a declared mailbox on \
			'{sender_domain}' that <StalwartProvision/> has minted"
		)
	})?;
	let host = mail.mail_box.hostname();
	let body = format!(
		"From: <{from}>\r\n\
		To: <{address}>\r\n\
		Subject: {}\r\n\
		\r\n\
		Inbound leg of the beet mail probe.\r\n",
		MailProbe::subject(token)
	);
	let message = mail.project.work_dir().join("mail-probe-inbound.eml");
	fs_ext::write_async(&message, body.as_bytes()).await?;
	ChildProcess::new("curl")
		.with_args([
			"--silent".to_string(),
			"--show-error".to_string(),
			"--fail".to_string(),
			format!("smtps://{host}:465"),
			"--mail-from".to_string(),
			from.to_string(),
			"--mail-rcpt".to_string(),
			address.to_string(),
			"--user".to_string(),
			format!("{from}:{password}"),
			"--upload-file".to_string(),
			message.display().to_string(),
		])
		.with_secret(&password)
		.run_async()
		.await?;
	Ok(())
}

/// Poll the mailbox over JMAP until this run's message arrives, then assert
/// what the box's own authentication checks concluded about it.
async fn assert_inbound(
	mail: &MailStack,
	relay: &RelayMode,
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
			let missing = MailProbe::authenticates(relay, &results);
			if !missing.is_empty() {
				bevybail!(
					"the inbound probe arrived but did not authenticate \
					({}): Authentication-Results was `{results}`",
					missing.join(", ")
				);
			}
			info!(
				"inbound mail carries {}",
				MailProbe::required_results(relay).join(", ")
			);
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

	/// The SES arm, ie the sender domain whose envelope domain is one we
	/// publish SPF for: all three verdicts or the probe fails, and the error
	/// names the ones that did not hold so the reader knows which record to
	/// look at.
	#[beet_core::test]
	fn every_verdict_must_pass() {
		MailProbe::authenticates(
			&RelayMode::Ses(SesRelay::default()),
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
		let ses = RelayMode::Ses(SesRelay::default());
		MailProbe::authenticates(
			&ses,
			"mx; spf=fail; dkim=pass header.i=@news.beetmash.com; dmarc=fail",
		)
		.xpect_eq(vec!["dmarc=pass", "spf=pass"]);
		MailProbe::authenticates(&ses, "").xpect_eq(vec![
			"dkim=pass",
			"dmarc=pass",
			"spf=pass",
		]);
	}

	/// Comail rewrites the envelope sender to a VERP address at its own domain
	/// (`internal/relay/queue.go:622-633`), so SPF is evaluated against
	/// `atmos.email` and can never ALIGN with the header From. DMARC rides the
	/// member-domain DKIM signature instead, so those two are demanded and the
	/// alignment nobody can produce is not.
	///
	/// Demanding `spf=pass` here would fail every comail deploy; treating the
	/// unaligned pass a receiver DOES report as proof would be worse, which is
	/// why the verdict is dropped rather than matched loosely.
	#[beet_core::test]
	fn comail_is_judged_on_dkim_rather_than_aligned_spf() {
		let comail = RelayMode::Comail(ComailRelay::default());
		MailProbe::authenticates(
			&comail,
			"mx.beetmash.com; spf=pass smtp.mailfrom=atmos.email; \
			dkim=pass header.i=@news.beetmash.com; dmarc=pass header.from=news.beetmash.com",
		)
		.is_empty()
		.xpect_true();
		// ..and the signature is not optional: without it there is nothing at
		// all for DMARC to pass on
		MailProbe::authenticates(&comail, "spf=pass; dkim=fail; dmarc=fail")
			.xpect_eq(vec!["dkim=pass", "dmarc=pass"]);
	}

	/// A warming-tier `429` is a neutral SKIP rather than a failure: a fresh
	/// member is capped at two messages an hour for its first week, so a deploy
	/// during warming can legally be refused and comail's own smoke test reads
	/// it the same way. Every other `4xx` is the enrolment being wrong and
	/// re-asking answers the same, so it is terminal.
	#[beet_core::test]
	fn a_warming_rate_limit_skips_rather_than_fails() {
		SendOutcome::of(StatusCode::OK).xpect_eq(SendOutcome::Sent);
		SendOutcome::of(StatusCode::TOO_MANY_REQUESTS)
			.xpect_eq(SendOutcome::Skip);
		// `DOMAIN_MISMATCH`, ie a key enrolled for another domain
		SendOutcome::of(StatusCode::FORBIDDEN).xpect_eq(SendOutcome::Failed);
		SendOutcome::of(StatusCode::UNAUTHORIZED).xpect_eq(SendOutcome::Failed);
		// a relay restarting is not a stack fault
		SendOutcome::of(StatusCode::BAD_GATEWAY).xpect_eq(SendOutcome::Retry);
	}

	/// Each mode is probed against the sink its own relay can reach: the SES
	/// simulator, comail's accept-and-drop blackhole, and (for a direct
	/// domain) that same blackhole reached over a real MX lookup, which is
	/// what direct delivery has to prove.
	#[beet_core::test]
	fn each_mode_probes_a_sink_it_can_reach() {
		MailProbe::sink(&RelayMode::Ses(SesRelay::default()))
			.as_str()
			.xpect_eq("success@simulator.amazonses.com");
		MailProbe::sink(&RelayMode::Comail(ComailRelay::default()))
			.as_str()
			.xpect_eq("smoke-sink@atmos.email");
		MailProbe::sink(&RelayMode::None)
			.as_str()
			.xpect_eq("smoke-sink@atmos.email");
	}

	/// Receivers format the header freely, so the match is whitespace- and
	/// case-insensitive rather than an exact string.
	#[beet_core::test]
	fn the_header_is_matched_loosely() {
		MailProbe::authenticates(
			&RelayMode::Ses(SesRelay::default()),
			"SPF = PASS; DKIM = PASS; DMARC = PASS",
		)
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
