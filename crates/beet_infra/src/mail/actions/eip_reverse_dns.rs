//! The reverse-dns half of the box's identity.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde_json::Value;

/// `<EipReverseDns/>` — after the apply, point the box's elastic IP back at its
/// own hostname.
///
/// A `PTR` is not decoration for an MTA. Receiving servers check that the
/// address a connection comes from resolves back to a name, and that the name
/// resolves forward to that address; a mismatch is one of the oldest and most
/// widely deployed spam signals there is. Ours is outbound-insurance rather
/// than the delivery path (everything relays through SES), but the box still
/// speaks SMTP to the world on 25 and it costs one api call to be legible.
///
/// The name is the box's, never a mail domain's: the hostname stays put across
/// a domain cutover, which is exactly why the riskiest step does not have to
/// touch this at all.
#[derive(Debug, Clone, Get, SetWith, Component, Reflect)]
#[reflect(Component, Default)]
#[require(EipReverseDnsAction)]
pub struct EipReverseDns {
	/// How long to wait for the forward record to resolve, and then for AWS to
	/// publish the reverse one.
	timeout: Duration,
	/// The gap between attempts, at both gates.
	poll: Duration,
}

impl Default for EipReverseDns {
	fn default() -> Self {
		Self {
			timeout: Duration::from_secs(300),
			poll: Duration::from_secs(10),
		}
	}
}

impl EipReverseDns {
	/// The resolver the forward check asks. DNS-over-HTTPS rather than the
	/// system resolver, so the answer comes from the public internet's view
	/// rather than from whatever the deployer's machine caches.
	pub const RESOLVER: &'static str = "https://cloudflare-dns.com/dns-query";
}

/// Requests the reverse record and waits for it to land.
#[action(handler_only)]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn EipReverseDnsAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let settings = cx
		.caller
		.get_cloned::<EipReverseDns>()
		.await
		.unwrap_or_default();
	let mail = cx.caller.with_world(MailStack::resolve).await??;

	let region = mail.stack.region().clone();
	let hostname = mail.mail_box.hostname().clone();
	let address = mail.public_ip().await?;
	let allocation = mail.eip_allocation().await?;

	// AWS validates the forward record before it will publish the reverse one,
	// so waiting here turns "request silently rejected hours later" into a
	// deploy that says which record has not propagated.
	wait_for_forward(
		&hostname,
		&address,
		*settings.timeout(),
		*settings.poll(),
	)
	.await?;

	info!("requesting PTR {address} -> {hostname}");
	ec2(&region, [
		"modify-address-attribute",
		"--allocation-id",
		&allocation,
		"--domain-name",
		&hostname,
	])
	.run_async()
	.await?;

	// publication is asynchronous and AWS gives itself hours, so a PTR still
	// pending is reported rather than failing the deploy: nothing downstream
	// waits on it, and the request is already lodged.
	match wait_for_ptr(
		&region,
		&allocation,
		&hostname,
		*settings.timeout(),
		*settings.poll(),
	)
	.await?
	{
		true => info!("{address} resolves back to {hostname}"),
		false => info!(
			"the PTR for {address} is still pending at AWS; it publishes \
			without further action"
		),
	}
	Pass(cx.input).xok()
}

/// Poll until `hostname` resolves forward to `address`.
async fn wait_for_forward(
	hostname: &str,
	address: &str,
	timeout: Duration,
	poll: Duration,
) -> Result {
	let attempts = (timeout.as_secs() / poll.as_secs().max(1)).max(1);
	let mut seen = None;
	for attempt in 1..=attempts {
		let answers = resolve_a(hostname).await.unwrap_or_default();
		if answers.iter().any(|answer| answer == address) {
			info!(
				"{hostname} resolves to {address} after {attempt} attempt(s)"
			);
			return Ok(());
		}
		seen = Some(answers);
		time_ext::sleep(poll).await;
	}
	bevybail!(
		"{hostname} does not resolve to {address} (saw {:?}), so AWS will \
		refuse the reverse record: the apply publishes the A record, so this \
		is propagation rather than a missing declaration",
		seen.unwrap_or_default()
	)
}

/// The `A` records `hostname` currently resolves to, over DNS-over-HTTPS.
async fn resolve_a(hostname: &str) -> Result<Vec<String>> {
	let response = Request::get(format!(
		"{}?name={hostname}&type=A",
		EipReverseDns::RESOLVER
	))
	.with_accept(MediaType::other("application/dns-json"))
	.send()
	.await?;
	if !response.status().is_ok() {
		bevybail!("dns lookup for {hostname} failed: {}", response.status());
	}
	let body: Value = response.json().await?;
	body["Answer"]
		.as_array()
		.map(|answers| {
			answers
				.iter()
				.filter(|answer| answer["type"] == 1)
				.filter_map(|answer| answer["data"].as_str().map(String::from))
				.collect::<Vec<_>>()
		})
		.unwrap_or_default()
		.xok()
}

/// Poll the address attribute until the PTR reads back as `hostname`. `false`
/// when it is still pending, which is a normal outcome rather than a failure.
async fn wait_for_ptr(
	region: &str,
	allocation: &str,
	hostname: &str,
	timeout: Duration,
	poll: Duration,
) -> Result<bool> {
	let attempts = (timeout.as_secs() / poll.as_secs().max(1)).max(1);
	for _ in 1..=attempts {
		let output = ec2(region, [
			"describe-addresses-attribute",
			"--allocation-ids",
			allocation,
			"--attribute",
			"domain-name",
			"--output",
			"json",
		])
		.run_async_stdout()
		.await?;
		if ptr_is_published(&output, hostname)? {
			return Ok(true);
		}
		time_ext::sleep(poll).await;
	}
	Ok(false)
}

/// Whether a `describe-addresses-attribute` body reports `hostname` as the
/// published record rather than a pending update.
///
/// The two fields differ by one letter of meaning: `PtrRecord` is what resolves
/// today, `PtrRecordUpdate` is what AWS is still working on. Reading the wrong
/// one reports success the moment the request is accepted.
fn ptr_is_published(body: &str, hostname: &str) -> Result<bool> {
	let value: Value = serde_json::from_str(body)?;
	// AWS returns the name fully qualified, ie with the root label
	let published = value["Addresses"][0]["PtrRecord"]
		.as_str()
		.unwrap_or_default()
		.trim_end_matches('.')
		.to_string();
	Ok(published == hostname)
}

/// An `aws ec2` invocation in `region`. Drops a possibly-empty inherited
/// `AWS_PROFILE`, which the cli reads as a profile literally named `""`.
fn ec2<'a>(
	region: &str,
	args: impl IntoIterator<Item = &'a str>,
) -> ChildProcess {
	ChildProcess::new("aws")
		.without_env("AWS_PROFILE")
		.with_args(
			["ec2"]
				.into_iter()
				.chain(args)
				.map(SmolStr::from)
				.chain([SmolStr::from("--region"), SmolStr::from(region)]),
		)
}

#[cfg(test)]
mod tests {
	use super::*;

	/// A request that has been accepted but not published reads as pending, not
	/// as done: `PtrRecordUpdate` is the work in progress and `PtrRecord` is
	/// what the internet sees.
	#[beet_core::test]
	fn a_pending_update_is_not_a_published_record() {
		let pending = r#"{"Addresses":[{"PublicIp":"1.2.3.4","PtrRecord":"","PtrRecordUpdate":{"Value":"mail.beetmash.com","Status":"PENDING"}}]}"#;
		ptr_is_published(pending, "mail.beetmash.com")
			.unwrap()
			.xpect_false();
	}

	/// AWS returns the name fully qualified; the block declares it without the
	/// root label, and comparing the two raw would never match.
	#[beet_core::test]
	fn the_published_record_matches_the_declared_hostname() {
		let done = r#"{"Addresses":[{"PublicIp":"1.2.3.4","PtrRecord":"mail.beetmash.com."}]}"#;
		ptr_is_published(done, "mail.beetmash.com")
			.unwrap()
			.xpect_true();
		ptr_is_published(done, "mail.example.com")
			.unwrap()
			.xpect_false();
	}
}
