//! The reverse-dns half of the box's identity: the record, and both halves of
//! its lifecycle.
use crate::actions::aws_cli_ext;
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde_json::Value;




/// `<EipReverseRecord up={$attach_up} down={$attach_down}/>` — the box's reverse
/// record, declared as a thing the stack HAS rather than as a step some verb
/// does.
///
/// One tag, one file position, two actions: the request joins the deploy group
/// and the reset joins the teardown group, so both groups' member lists are
/// projections of the same insert order and cannot drift apart.
///
/// It is a declaration rather than a resource because terraform does not own a
/// PTR: AWS publishes it asynchronously against an address terraform allocated,
/// and refuses to release that address while it is set. Without the down half
/// the record outlives every destroy and takes the address with it.
#[template]
pub fn EipReverseRecord(
	/// The group the request joins, ie the stack's post-apply attach group.
	#[prop(required)]
	up: Entity,
	/// The group the reset joins, ie the stack's teardown attach group.
	#[prop(required)]
	down: Entity,
) -> impl Bundle {
	children![
		(EipReverseDns::default(), MemberOf(up)),
		(EipReverseDnsReset::default(), MemberOf(down)),
	]
}

impl EipReverseDns {
	/// The resolver the forward check asks. DNS-over-HTTPS rather than the
	/// system resolver, so the answer comes from the public internet's view
	/// rather than from whatever the deployer's machine caches.
	pub const RESOLVER: &'static str = "https://cloudflare-dns.com/dns-query";

	/// Whether an `aws ec2` failure is "that address does not exist".
	///
	/// The one failure a teardown reads as success: a destroy walks the
	/// declaration rather than a ledger, so it routinely addresses a resource
	/// that was never created or has already gone.
	fn is_missing(err: &BevyError) -> bool {
		err.to_string().contains("InvalidAllocationID.NotFound")
	}
}

/// Requests the reverse record and waits for it to land.
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
#[action]
#[derive(Component, Reflect)]
#[reflect(Component, Default)]
pub async fn EipReverseDns(
	/// How long to wait for the forward record to resolve, and then for AWS to
	/// publish the reverse one.
	#[field(default = Duration::from_secs(300))]
	timeout: Duration,
	/// The gap between attempts, at both gates.
	#[field(default = Duration::from_secs(10))]
	poll: Duration,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let mail = cx.caller.with_world(MailStack::resolve).await??;

	let region = mail.stack.region().clone();
	let hostname = mail.mail_box.hostname().clone();
	let address = mail.public_ip().await?;
	let allocation = mail.eip_allocation().await?;

	// AWS validates the forward record before it will publish the reverse one,
	// so waiting here turns "request silently rejected hours later" into a
	// deploy that says which record has not propagated.
	wait_for_forward(&hostname, &address, timeout, poll).await?;

	info!("requesting PTR {address} -> {hostname}");
	aws_cli_ext::ec2(&region, [
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
	match wait_for_ptr(&region, &allocation, &hostname, timeout, poll).await? {
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
		let output = aws_cli_ext::ec2(region, [
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


/// Clears the box's reverse record, then waits until AWS will actually let go
/// of the address.
/// The teardown half of [`EipReverseRecord`].
///
/// It runs BEFORE `tofu destroy`, because the record converges after the apply
/// that allocated the address, and teardown order is convergence order
/// reversed. That ordering is the whole point: AWS refuses `ReleaseAddress`
/// while a PTR is set, so terraform cannot take the address back until this has
/// run.
///
/// The wait is the subtle part, and it is why this exists at all.
/// `describe-addresses-attribute` reports the record gone within seconds while
/// `ReleaseAddress` keeps failing with `InvalidAddress.PtrSet` for minutes
/// afterwards: the two are different views of the same eventually consistent
/// state. So the record reading clear is the *start* of the wait, not the end
/// of it. Every attempt probes the release path itself with a dry run and, once
/// nothing refuses, requires the address to keep reading releasable for
/// `settle` before handing over to terraform.
///
/// A no-op when there is no address, when it never carried a record, and when
/// the stack's state is already gone: a teardown walks declarations rather than
/// a ledger of what was created, so every down-action has to succeed against a
/// resource that was never there.
#[action]
#[derive(Component, Reflect)]
#[reflect(Component, Default)]
pub async fn EipReverseDnsReset(
	/// How long to wait for the address to become releasable before giving up
	/// and saying so.
	#[field(default = Duration::from_secs(1200))]
	timeout: Duration,
	/// The gap between attempts.
	#[field(default = Duration::from_secs(15))]
	poll: Duration,
	/// How long the address must read releasable before the wait ends, see the
	/// eventual consistency note above.
	#[field(default = Duration::from_secs(180))]
	settle: Duration,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let mail = cx.caller.with_world(MailStack::resolve).await??;
	let region = mail.stack.region().clone();
	// no state, no address, nothing to reset: the resource may never have been
	// created, and a destroy walks the declaration either way.
	let Some(allocation) = mail
		.eip_allocation()
		.await
		.ok()
		.map(|allocation| allocation.trim().to_string())
		.filter(|allocation| !allocation.is_empty())
	else {
		info!("no elastic ip in this stack's state, nothing to reset");
		return Pass(cx.input).xok();
	};

	info!("resetting the PTR on {allocation}");
	if let Err(err) = aws_cli_ext::ec2(&region, [
		"reset-address-attribute",
		"--allocation-id",
		&allocation,
		"--attribute",
		"domain-name",
	])
	.run_async()
	.await
	{
		// an address that is already gone is a clean outcome, anything else is not
		if !EipReverseDns::is_missing(&err) {
			return Err(err);
		}
		info!("{allocation} is already gone");
		return Pass(cx.input).xok();
	}

	wait_until_releasable(&region, &allocation, timeout, poll, settle).await?;
	Pass(cx.input).xok()
}

/// Poll until `allocation` has read releasable for `settle`, ie until the
/// release path itself stops refusing and keeps not refusing.
async fn wait_until_releasable(
	region: &str,
	allocation: &str,
	timeout: Duration,
	poll: Duration,
	settle: Duration,
) -> Result {
	let started = Instant::now();
	let mut clear_since: Option<Instant> = None;
	while started.elapsed() < timeout {
		match refusal(region, allocation).await? {
			Some(reason) => {
				// a reading that refuses restarts the settle: the state went
				// backwards, so anything counted before it means nothing.
				clear_since = None;
				info!(
					"{allocation} is not releasable yet after {:?}: {reason}",
					started.elapsed()
				);
			}
			None => {
				let since = *clear_since.get_or_insert_with(Instant::now);
				if since.elapsed() >= settle {
					info!(
						"{allocation} has been releasable for {:?}, terraform can take it back",
						since.elapsed()
					);
					return Ok(());
				}
				info!(
					"{allocation} reads releasable, confirming for another {:?}",
					settle.saturating_sub(since.elapsed())
				);
			}
		}
		time_ext::sleep(poll).await;
	}
	bevybail!(
		"{allocation} still had a reverse record {timeout:?} after it was reset, \
		so terraform will not be able to release it. AWS publishes and withdraws \
		a PTR asynchronously and gives itself hours for it; re-run the destroy \
		later rather than releasing the address by hand."
	)
}

/// Why `allocation` cannot be released right now, or [`None`] if nothing
/// refuses it.
///
/// Two readings, because neither alone is the answer: the attribute says what
/// the record service holds, and a dry-run release says what the release path
/// thinks, which is the one that actually blocks terraform.
async fn refusal(region: &str, allocation: &str) -> Result<Option<String>> {
	let attribute = aws_cli_ext::ec2(region, [
		"describe-addresses-attribute",
		"--allocation-ids",
		allocation,
		"--attribute",
		"domain-name",
		"--output",
		"json",
	])
	.run_async_stdout()
	.await;
	match attribute {
		// the address itself is gone, so nothing is holding it
		Err(err) if EipReverseDns::is_missing(&err) => return Ok(None),
		Err(err) => return Err(err),
		Ok(body) if ptr_is_pending(&body)? => {
			return Ok(Some("the record is still published".to_string()));
		}
		Ok(_) => {}
	}
	match aws_cli_ext::ec2(region, [
		"release-address",
		"--allocation-id",
		allocation,
		"--dry-run",
	])
	.run_async()
	.await
	{
		// a dry run always "fails"; this is the success answer
		Err(err) if err.to_string().contains("DryRunOperation") => Ok(None),
		Err(err) if EipReverseDns::is_missing(&err) => Ok(None),
		Err(err) if err.to_string().contains("InvalidAddress.PtrSet") => {
			Ok(Some("the release path still reports a PTR".to_string()))
		}
		// any other refusal (a permission the deploy role lacks, say) is not
		// this action's business: the attribute already read clear.
		Err(_) | Ok(_) => Ok(None),
	}
}

/// Whether a `describe-addresses-attribute` body still reports a record, either
/// published or on its way to being one.
fn ptr_is_pending(body: &str) -> Result<bool> {
	let value: Value = serde_json::from_str(body)?;
	let address = &value["Addresses"][0];
	let published = address["PtrRecord"].as_str().unwrap_or_default();
	let updating = address["PtrRecordUpdate"]["Value"]
		.as_str()
		.unwrap_or_default();
	Ok(!published.is_empty() || !updating.is_empty())
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

	/// The teardown reads a record that is still on its way OUT as still there:
	/// `PtrRecordUpdate` is the work in progress either direction, and treating
	/// a pending withdrawal as done is how a destroy reaches
	/// `ReleaseAddress` before AWS has let go.
	#[beet_core::test]
	fn a_pending_withdrawal_still_counts_as_a_record() {
		let withdrawing = r#"{"Addresses":[{"PublicIp":"1.2.3.4","PtrRecord":"","PtrRecordUpdate":{"Value":"mail.beetmash.com","Status":"PENDING"}}]}"#;
		super::ptr_is_pending(withdrawing).unwrap().xpect_true();
		let done = r#"{"Addresses":[{"PublicIp":"1.2.3.4","PtrRecord":"","PtrRecordUpdate":{}}]}"#;
		super::ptr_is_pending(done).unwrap().xpect_false();
		let published = r#"{"Addresses":[{"PublicIp":"1.2.3.4","PtrRecord":"mail.beetmash.com."}]}"#;
		super::ptr_is_pending(published).unwrap().xpect_true();
	}

	/// An address that was never allocated is a clean teardown, not a failure:
	/// a destroy walks the declaration rather than a ledger of what was made.
	#[beet_core::test]
	fn a_missing_address_is_not_a_failure() {
		EipReverseDns::is_missing(&bevyhow!(
			"An error occurred (InvalidAllocationID.NotFound) when calling the ReleaseAddress operation"
		))
		.xpect_true();
		EipReverseDns::is_missing(&bevyhow!("AccessDenied")).xpect_false();
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
