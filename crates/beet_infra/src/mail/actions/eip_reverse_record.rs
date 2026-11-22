//! The reverse-dns half of the box's identity: the record, and both halves of
//! its lifecycle.
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
	/// rather than from whatever the deploying machine caches.
	pub const RESOLVER: &'static str = "https://cloudflare-dns.com/dns-query";

	/// Whether an EC2 failure is "that address does not exist".
	///
	/// The one failure a teardown reads as success: a destroy walks the
	/// declaration rather than a ledger, so it routinely addresses a resource
	/// that was never created or has already gone.
	fn is_missing(err: &BevyError) -> bool {
		AwsError::is(err, &["InvalidAllocationID.NotFound"])
	}
}

/// The address attribute calls on one elastic IP, over an EC2 client.
struct ElasticAddress {
	client: AwsClient,
	allocation: String,
}

/// The reverse record of an address as `DescribeAddressesAttribute` reports
/// it.
///
/// The two fields differ by one letter of meaning: `ptr_record` is what
/// resolves today, `ptr_record_update` is what AWS is still working on, in
/// either direction. Reading the wrong one reports success the moment a
/// request is accepted.
#[derive(Debug, Default, Clone, PartialEq)]
struct PtrRecord {
	/// Fully qualified, ie with the root label, empty when none is published.
	ptr_record: String,
	/// The value of a pending update, empty when none is pending.
	ptr_record_update: String,
}

/// What a dry-run `ReleaseAddress` says of an address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReleaseProbe {
	/// `DryRunOperation`: the release would go through.
	Allowed,
	/// `InvalidAddress.PtrSet`: the release path still holds a record.
	PtrSet,
	/// `UnauthorizedOperation`: the caller may not release it at all.
	Unauthorized,
	/// The address is already gone.
	Missing,
}

impl ElasticAddress {
	fn new(region: &str, allocation: impl Into<String>) -> Self {
		Self {
			client: AwsClient::new(AwsService::EC2, region),
			allocation: allocation.into(),
		}
	}

	/// `ModifyAddressAttribute`: request `hostname` as the reverse record.
	async fn request_ptr(&self, hostname: &str) -> Result {
		self.client
			.call_query("ModifyAddressAttribute", [
				("AllocationId", self.allocation.as_str()),
				("DomainName", hostname),
			])
			.await?;
		Ok(())
	}

	/// `DescribeAddressesAttribute` of the `domain-name` attribute.
	async fn ptr(&self) -> Result<PtrRecord> {
		self.client
			.call_query("DescribeAddressesAttribute", [
				("AllocationId.1", self.allocation.as_str()),
				("Attribute", "domain-name"),
			])
			.await?
			.xmap(|response| PtrRecord::parse(&response))
			.xok()
	}

	/// `ResetAddressAttribute` of the `domain-name` attribute.
	async fn reset_ptr(&self) -> Result {
		self.client
			.call_query("ResetAddressAttribute", [
				("AllocationId", self.allocation.as_str()),
				("Attribute", "domain-name"),
			])
			.await?;
		Ok(())
	}

	/// A dry-run `ReleaseAddress`, which always answers an error: its code
	/// is the reading. Any other error is returned as it is.
	async fn probe_release(&self) -> Result<ReleaseProbe> {
		let Err(err) = self
			.client
			.call_query("ReleaseAddress", [
				("AllocationId", self.allocation.as_str()),
				("DryRun", "true"),
			])
			.await
		else {
			bevybail!(
				"a dry-run ReleaseAddress of {} released it",
				self.allocation
			);
		};
		match AwsError::code(&err) {
			Some("DryRunOperation") => ReleaseProbe::Allowed.xok(),
			Some("InvalidAddress.PtrSet") => ReleaseProbe::PtrSet.xok(),
			Some("UnauthorizedOperation") => ReleaseProbe::Unauthorized.xok(),
			_ if EipReverseDns::is_missing(&err) => ReleaseProbe::Missing.xok(),
			_ => Err(err),
		}
	}
}

impl PtrRecord {
	/// The first address of a `DescribeAddressesAttribute` response.
	fn parse(response: &XmlElement) -> Self {
		let text = |path: &str| {
			response
				.text_at(&format!("addressSet/item/{path}"))
				.unwrap_or_default()
				.to_string()
		};
		Self {
			ptr_record: text("ptrRecord"),
			ptr_record_update: text("ptrRecordUpdate/value"),
		}
	}

	/// Whether `hostname` is the published record rather than a pending
	/// update. AWS answers the name fully qualified, the block declares it
	/// without the root label.
	fn is_published(&self, hostname: &str) -> bool {
		self.ptr_record.trim_end_matches('.') == hostname
	}

	/// Whether any record remains, published or on its way in or out.
	fn is_pending(&self) -> bool {
		!self.ptr_record.is_empty() || !self.ptr_record_update.is_empty()
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
	let mail = MailStack::of(&cx.caller).await?;

	let region = mail.stack.aws_region()?.clone();
	let hostname = mail.mail_box.hostname().clone();
	let address = mail.public_ip().await?;
	let allocation = mail.eip_allocation().await?;

	// AWS validates the forward record before it will publish the reverse one,
	// so waiting here turns "request silently rejected hours later" into a
	// deploy that says which record has not propagated.
	wait_for_forward(&hostname, &address, timeout, poll).await?;

	info!("requesting PTR {address} -> {hostname}");
	let elastic = ElasticAddress::new(&region, allocation);
	elastic.request_ptr(&hostname).await?;

	// publication is asynchronous and AWS gives itself hours, so a PTR still
	// pending is reported rather than failing the deploy: nothing downstream
	// waits on it, and the request is already lodged.
	match wait_for_ptr(&elastic, &hostname, timeout, poll).await? {
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
	elastic: &ElasticAddress,
	hostname: &str,
	timeout: Duration,
	poll: Duration,
) -> Result<bool> {
	let attempts = (timeout.as_secs() / poll.as_secs().max(1)).max(1);
	for _ in 1..=attempts {
		if elastic.ptr().await?.is_published(hostname) {
			return Ok(true);
		}
		time_ext::sleep(poll).await;
	}
	Ok(false)
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
	let mail = MailStack::of(&cx.caller).await?;
	let region = mail.stack.aws_region()?.clone();
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
	let elastic = ElasticAddress::new(&region, allocation.clone());
	if let Err(err) = elastic.reset_ptr().await {
		// an address that is already gone is a clean outcome, anything else is not
		if !EipReverseDns::is_missing(&err) {
			return Err(err);
		}
		info!("{allocation} is already gone");
		return Pass(cx.input).xok();
	}

	wait_until_releasable(&elastic, timeout, poll, settle).await?;
	Pass(cx.input).xok()
}

/// Poll until `allocation` has read releasable for `settle`, ie until the
/// release path itself stops refusing and keeps not refusing.
async fn wait_until_releasable(
	elastic: &ElasticAddress,
	timeout: Duration,
	poll: Duration,
	settle: Duration,
) -> Result {
	let allocation = &elastic.allocation;
	let started = Instant::now();
	let mut clear_since: Option<Instant> = None;
	while started.elapsed() < timeout {
		match refusal(elastic).await? {
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

/// Why the address cannot be released right now, or [`None`] if nothing
/// refuses it.
///
/// Two readings, because neither alone is the answer: the attribute says what
/// the record service holds, and a dry-run release says what the release path
/// thinks, which is the one that actually blocks terraform.
async fn refusal(elastic: &ElasticAddress) -> Result<Option<String>> {
	match elastic.ptr().await {
		// the address itself is gone, so nothing is holding it
		Err(err) if EipReverseDns::is_missing(&err) => return Ok(None),
		Err(err) => return Err(err),
		Ok(ptr) if ptr.is_pending() => {
			return Ok(Some("the record is still published".to_string()));
		}
		Ok(_) => {}
	}
	match elastic.probe_release().await? {
		ReleaseProbe::PtrSet => {
			Ok(Some("the release path still reports a PTR".to_string()))
		}
		ReleaseProbe::Allowed | ReleaseProbe::Missing => Ok(None),
		// a permission the deploy role lacks is not this action's business:
		// the attribute already read clear
		ReleaseProbe::Unauthorized => {
			debug!(
				"the dry-run release of {} is unauthorized, the attribute reads clear",
				elastic.allocation
			);
			Ok(None)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::sync::Arc;
	use std::sync::Mutex;

	/// One call's form params.
	type Params = MultiMap<SmolStr, SmolStr>;

	/// Every call an [`ElasticAddress`] made.
	type Calls = Arc<Mutex<Vec<Params>>>;

	/// The address `eipalloc-1` in `ap-southeast-2`, its EC2 answering
	/// `answer` given each call's form params, every call recorded.
	fn recording(
		answer: impl 'static + Send + Sync + Fn(&Params) -> Response,
	) -> (ElasticAddress, Calls) {
		let calls = Calls::default();
		let recorded = calls.clone();
		let answer = Arc::new(answer);
		let client = AwsClient::new(AwsService::EC2, "ap-southeast-2")
			.with_credentials(AwsCredentials::new("AKID", "secret"))
			.with_transport(move |request| {
				let (recorded, answer) = (recorded.clone(), answer.clone());
				Box::pin(async move {
					request
						.headers
						.first_raw("authorization")
						.unwrap()
						.xpect_contains("/ap-southeast-2/ec2/aws4_request");
					let body = request.body.into_bytes().await?;
					let params =
						Url::parse_query_string(str::from_utf8(&body)?);
					let response = answer(&params);
					recorded.lock().unwrap().push(params);
					response.xok()
				})
			});
		let elastic = ElasticAddress {
			client,
			allocation: "eipalloc-1".into(),
		};
		(elastic, calls)
	}

	/// The value of `key` in `params`.
	fn param<'a>(params: &'a Params, key: &str) -> Option<&'a str> {
		params.get(key).map(SmolStr::as_str)
	}

	/// An EC2 error answer carrying `code`.
	fn error(status: StatusCode, code: &str) -> Response {
		Response::from_status(status).with_body(format!(
			"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
			<Response><Errors><Error><Code>{code}</Code>\
			<Message>no</Message></Error></Errors>\
			<RequestID>req-1</RequestID></Response>"
		))
	}

	/// A `DescribeAddressesAttribute` answer with `inner` as its one address.
	fn attribute(inner: &str) -> Response {
		Response::ok().with_body(format!(
			"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
			<DescribeAddressesAttributeResponse xmlns=\"http://ec2.amazonaws.com/doc/2016-11-15/\">\
			<requestId>req-1</requestId><addressSet><item>\
			<publicIp>1.2.3.4</publicIp><allocationId>eipalloc-1</allocationId>\
			{inner}</item></addressSet></DescribeAddressesAttributeResponse>"
		))
	}

	const PENDING: &str = "<ptrRecordUpdate><value>mail.beetmash.com</value>\
		<status>PENDING</status></ptrRecordUpdate>";
	const PUBLISHED: &str = "<ptrRecord>mail.beetmash.com.</ptrRecord>";

	/// The request names the address and the hostname, and the read back
	/// asks for the `domain-name` attribute of that one address.
	#[beet_core::test]
	async fn requests_and_reads_the_record() {
		let (elastic, calls) =
			recording(|params| match param(params, "Action") {
				Some("ModifyAddressAttribute") => Response::ok().with_body(
					"<ModifyAddressAttributeResponse><address>\
					<allocationId>eipalloc-1</allocationId></address>\
					</ModifyAddressAttributeResponse>",
				),
				_ => attribute(PUBLISHED),
			});
		elastic.request_ptr("mail.beetmash.com").await.unwrap();
		elastic
			.ptr()
			.await
			.unwrap()
			.is_published("mail.beetmash.com")
			.xpect_true();
		let calls = calls.lock().unwrap().clone();
		param(&calls[0], "Version").xpect_eq(Some("2016-11-15"));
		param(&calls[0], "AllocationId").xpect_eq(Some("eipalloc-1"));
		param(&calls[0], "DomainName").xpect_eq(Some("mail.beetmash.com"));
		param(&calls[1], "Action").xpect_eq(Some("DescribeAddressesAttribute"));
		param(&calls[1], "AllocationId.1").xpect_eq(Some("eipalloc-1"));
		param(&calls[1], "Attribute").xpect_eq(Some("domain-name"));
	}

	/// A request that has been accepted but not published reads as pending, not
	/// as done: `ptrRecordUpdate` is the work in progress and `ptrRecord` is
	/// what the internet sees.
	#[beet_core::test]
	async fn a_pending_update_is_not_a_published_record() {
		let (elastic, _) = recording(|_| attribute(PENDING));
		let ptr = elastic.ptr().await.unwrap();
		ptr.is_published("mail.beetmash.com").xpect_false();
		ptr.ptr_record_update.as_str().xpect_eq("mail.beetmash.com");
	}

	/// The teardown reads a record that is still on its way OUT as still there:
	/// `ptrRecordUpdate` is the work in progress either direction, and treating
	/// a pending withdrawal as done is how a destroy reaches
	/// `ReleaseAddress` before AWS has let go.
	#[beet_core::test]
	async fn a_pending_withdrawal_still_counts_as_a_record() {
		for (inner, pending) in [
			(PENDING, true),
			("<ptrRecordUpdate/>", false),
			(PUBLISHED, true),
		] {
			let (elastic, _) = recording(move |_| attribute(inner));
			elastic.ptr().await.unwrap().is_pending().xpect_eq(pending);
		}
	}

	/// AWS returns the name fully qualified; the block declares it without the
	/// root label, and comparing the two raw would never match.
	#[beet_core::test]
	fn the_published_record_matches_the_declared_hostname() {
		let ptr = PtrRecord {
			ptr_record: "mail.beetmash.com.".into(),
			..default()
		};
		ptr.is_published("mail.beetmash.com").xpect_true();
		ptr.is_published("mail.example.com").xpect_false();
	}

	/// The dry-run release is read by its code: `DryRunOperation` would
	/// release, `UnauthorizedOperation` may not, a set PTR still blocks, a
	/// gone address is nothing to wait for, and anything else is an error.
	#[beet_core::test]
	async fn the_dry_run_release_is_read_by_its_code() {
		for (status, code, probe) in [
			(
				StatusCode::PRECONDITION_FAILED,
				"DryRunOperation",
				ReleaseProbe::Allowed,
			),
			(
				StatusCode::FORBIDDEN,
				"UnauthorizedOperation",
				ReleaseProbe::Unauthorized,
			),
			(
				StatusCode::BAD_REQUEST,
				"InvalidAddress.PtrSet",
				ReleaseProbe::PtrSet,
			),
			(
				StatusCode::BAD_REQUEST,
				"InvalidAllocationID.NotFound",
				ReleaseProbe::Missing,
			),
		] {
			let (elastic, calls) = recording(move |_| error(status, code));
			elastic.probe_release().await.unwrap().xpect_eq(probe);
			let calls = calls.lock().unwrap().clone();
			param(&calls[0], "Action").xpect_eq(Some("ReleaseAddress"));
			param(&calls[0], "AllocationId").xpect_eq(Some("eipalloc-1"));
			param(&calls[0], "DryRun").xpect_eq(Some("true"));
		}
		let (elastic, _) = recording(|_| {
			error(StatusCode::BAD_REQUEST, "InvalidParameterValue")
		});
		AwsError::code(&elastic.probe_release().await.unwrap_err())
			.xpect_eq(Some("InvalidParameterValue"));
	}

	/// The teardown waits on a record still published, and once it reads
	/// clear only the release path's own `PtrSet` holds it.
	#[beet_core::test]
	async fn a_refusal_is_a_record_or_a_ptr_set_release() {
		let (elastic, _) = recording(|_| attribute(PUBLISHED));
		refusal(&elastic).await.unwrap().xpect_some();
		let (elastic, _) = recording(|params| match param(params, "Action") {
			Some("ReleaseAddress") => {
				error(StatusCode::BAD_REQUEST, "InvalidAddress.PtrSet")
			}
			_ => attribute(""),
		});
		refusal(&elastic).await.unwrap().xpect_some();
		let (elastic, _) = recording(|params| match param(params, "Action") {
			Some("ReleaseAddress") => {
				error(StatusCode::PRECONDITION_FAILED, "DryRunOperation")
			}
			_ => attribute(""),
		});
		refusal(&elastic).await.unwrap().xpect_none();
	}

	/// An address that was never allocated is a clean teardown, not a failure:
	/// a destroy walks the declaration rather than a ledger of what was made.
	#[beet_core::test]
	async fn a_missing_address_is_not_a_failure() {
		let (elastic, _) = recording(|_| {
			error(StatusCode::BAD_REQUEST, "InvalidAllocationID.NotFound")
		});
		let err = elastic.reset_ptr().await.unwrap_err();
		EipReverseDns::is_missing(&err).xpect_true();
		refusal(&elastic).await.unwrap().xpect_none();
		EipReverseDns::is_missing(&bevyhow!("InvalidAllocationID.NotFound"))
			.xpect_false();
	}
}
