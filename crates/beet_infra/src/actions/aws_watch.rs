use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// The log group a watch follows, named by the deploy target rather than by the
/// provider's string, so the group is composed from the resolved [`Stack`] at
/// tail time.
#[derive(Debug, Clone, PartialEq, Eq, Reflect)]
pub enum WatchTarget {
	/// A literal CloudWatch log group, for a group beet does not name.
	LogGroup(SmolStr),
	/// A lambda function's `/aws/lambda/<function>` group, by block label.
	Lambda(SmolStr),
	/// A provisioned box's `/<app>/<label>/<stage>` group, by block label: the
	/// group its cloud-init CloudWatch agent forwards to. Both box blocks
	/// compose it identically ([`LightsailBlock::log_group`],
	/// `StalwartBlock::log_group`).
	Instance(SmolStr),
	/// A fargate service's `/ecs/<app>/<stage>` group.
	Fargate,
}

impl Default for WatchTarget {
	fn default() -> Self { Self::LogGroup(default()) }
}

impl WatchTarget {
	/// The CloudWatch group this target resolves to in `stack`.
	pub fn log_group(&self, stack: &ResolvedStack) -> String {
		match self {
			Self::LogGroup(group) => group.to_string(),
			Self::Lambda(label) => format!(
				"/aws/lambda/{}",
				stack
					.resource_ident(format!("{label}--function"))
					.primary_identifier()
			),
			Self::Instance(label) => {
				format!("/{}/{label}/{}", stack.app_name(), stack.stage())
			}
			Self::Fargate => {
				format!("/ecs/{}/{}", stack.app_name(), stack.stage())
			}
		}
	}
}

impl AwsWatch {
	/// Follow a literal CloudWatch log group.
	pub fn new(log_group: impl Into<SmolStr>) -> Self {
		Self::for_target(WatchTarget::LogGroup(log_group.into()))
	}

	/// Follow the group `target` resolves to.
	pub fn for_target(target: WatchTarget) -> Self {
		Self {
			target,
			timeout: None,
		}
	}

	/// Stop the tail after `timeout`, rather than following indefinitely.
	pub fn with_timeout(mut self, timeout: Duration) -> Self {
		self.timeout = Some(timeout);
		self
	}
}

/// Tails the CloudWatch logs of a [`WatchTarget`], resolved against the nearest
/// ancestor [`Stack`] when the tail runs, so a watch verb declares WHAT it
/// follows and never restates the app identity.
///
/// Polls CloudWatch Logs `FilterLogEvents` as the launch's own credential, in
/// the nearest ancestor [`Stack`]'s region, from ten minutes ago,
/// printing each event to stdout as `<utc timestamp> <message>`.
#[action]
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn AwsWatch(
	/// Which deployed thing's logs to follow.
	#[field]
	target: WatchTarget,
	/// Optional timeout after which the tail stops.
	/// When absent, follows indefinitely until interrupted.
	#[field]
	timeout: Option<Duration>,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let (region, log_group) = cx
		.caller
		.with_state::<StackQuery, _>(move |entity, query| -> Result<_> {
			let stack = query.resolve(entity);
			(stack.aws_region()?.clone(), target.log_group(&stack)).xok()
		})
		.await??;

	info!("tailing CloudWatch log group: {log_group} (region: {region})");

	let follow = LogTail::new(
		AwsClient::new(AwsService::LOGS, region),
		log_group,
		Timestamp::now(),
	)
	.follow();
	match timeout {
		Some(timeout) => match async_ext::timeout(timeout, follow).await {
			Ok(result) => result?,
			Err(_) => info!("watch timed out after {timeout:?}"),
		},
		None => follow.await?,
	}

	Pass(cx.input).xok()
}

/// A CloudWatch log group followed by `FilterLogEvents` polls, printing each
/// event once.
///
/// Each poll reads from the newest timestamp printed, inclusive, since a
/// later event may still land at that same millisecond; the event ids
/// printed at it are remembered so the overlap is not printed twice.
struct LogTail {
	client: AwsClient,
	log_group: String,
	/// The newest event timestamp printed, in epoch millis.
	cursor: i64,
	/// The ids of the events printed at [`cursor`](Self::cursor).
	seen: HashSet<String>,
}

impl LogTail {
	/// How far back the first poll reads.
	const SINCE: Duration = Duration::from_secs(10 * 60);
	/// The pause between polls.
	const POLL_INTERVAL: Duration = Duration::from_secs(2);

	/// Tail `log_group` from [`SINCE`](Self::SINCE) before `now`.
	fn new(
		client: AwsClient,
		log_group: impl Into<String>,
		now: Timestamp,
	) -> Self {
		Self {
			client,
			log_group: log_group.into(),
			cursor: now.millis() - Self::SINCE.as_millis() as i64,
			seen: default(),
		}
	}

	/// Poll and print until the future is dropped.
	async fn follow(mut self) -> Result {
		loop {
			for line in self.poll().await? {
				cross_log!("{line}");
			}
			time_ext::sleep(Self::POLL_INTERVAL).await;
		}
	}

	/// One poll: every page of events since the cursor, answering the lines
	/// not yet printed, oldest first, and advancing the cursor past them.
	async fn poll(&mut self) -> Result<Vec<String>> {
		let mut events = Vec::new();
		let mut next_token = None::<String>;
		loop {
			let mut body = serde_json::json!({
				"logGroupName": self.log_group,
				"startTime": self.cursor,
			});
			if let Some(token) = &next_token {
				body["nextToken"] = token.as_str().into();
			}
			let page = self.client.call_json("FilterLogEvents", &body).await?;
			if let Some(page_events) = page["events"].as_array() {
				events.extend(page_events.iter().cloned());
			}
			next_token = page["nextToken"].as_str().map(ToString::to_string);
			if next_token.is_none() {
				break;
			}
		}
		events.sort_by_key(|event| {
			event["timestamp"].as_i64().unwrap_or_default()
		});

		let mut lines = Vec::new();
		for event in events {
			let timestamp = event["timestamp"].as_i64().unwrap_or_default();
			let id = event["eventId"].as_str().unwrap_or_default().to_string();
			if timestamp < self.cursor
				|| (timestamp == self.cursor && self.seen.contains(&id))
			{
				continue;
			}
			if timestamp > self.cursor {
				self.cursor = timestamp;
				self.seen.clear();
			}
			self.seen.insert(id);
			lines.push(Self::line(
				timestamp,
				event["message"].as_str().unwrap_or_default(),
			));
		}
		lines.xok()
	}

	/// An event as printed: its UTC second and its message, ie
	/// `2026-11-22T03:14:15 GET /about 200`.
	fn line(timestamp: i64, message: &str) -> String {
		let time = Timestamp::from_millis(timestamp).format_iso8601_secs();
		format!(
			"{} {}",
			time.trim_end_matches('Z'),
			message.trim_end_matches(['\n', '\r'])
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::Value;
	use serde_json::json;
	use std::sync::Arc;
	use std::sync::Mutex;

	/// A tail of `/ecs/beetmash/prod` whose Logs answers `pages` in turn,
	/// recording each request body.
	fn tail(pages: Vec<Value>) -> (LogTail, Arc<Mutex<Vec<Value>>>) {
		let calls = Arc::new(Mutex::new(Vec::new()));
		let recorded = calls.clone();
		let pages = Arc::new(Mutex::new(pages.into_iter()));
		let client = AwsClient::new(AwsService::LOGS, "ap-southeast-2")
			.with_credentials(AwsCredentials::new("AKID", "secret"))
			.with_transport(move |request| {
				let (recorded, pages) = (recorded.clone(), pages.clone());
				Box::pin(async move {
					request
						.headers
						.first_raw("x-amz-target")
						.unwrap()
						.xpect_eq("Logs_20140328.FilterLogEvents");
					let body = serde_json::from_slice::<Value>(
						&request.body.into_bytes().await?,
					)?;
					recorded.lock().unwrap().push(body);
					let page = pages
						.lock()
						.unwrap()
						.next()
						.ok_or_else(|| bevyhow!("no page left"))?;
					Response::ok().with_body(page.to_string()).xok()
				})
			});
		let now = Timestamp::from_millis(1_800_000_600_000);
		(LogTail::new(client, "/ecs/beetmash/prod", now), calls)
	}

	fn event(id: &str, timestamp: i64, message: &str) -> Value {
		json!({
			"logStreamName": "ecs/app/0123",
			"timestamp": timestamp,
			"message": message,
			"ingestionTime": timestamp + 50,
			"eventId": id,
		})
	}

	/// A poll pages through `nextToken`, and the next reads from the newest
	/// timestamp printed, skipping the events already printed at it.
	#[beet_core::test]
	async fn pages_and_skips_printed_events() {
		let (mut tail, calls) = tail(vec![
			json!({
				"events": [event("1", 1_800_000_001_000, "booting\n")],
				"nextToken": "page-2",
				"searchedLogStreams": [],
			}),
			json!({
				"events": [
					event("2", 1_800_000_002_000, "GET /about 200"),
					event("3", 1_800_000_002_000, "GET / 200"),
				],
			}),
			// the second poll overlaps the newest millisecond
			json!({
				"events": [
					event("2", 1_800_000_002_000, "GET /about 200"),
					event("3", 1_800_000_002_000, "GET / 200"),
					event("4", 1_800_000_002_000, "GET /blog 200"),
					event("5", 1_800_000_003_000, "GET /docs 200"),
				],
			}),
		]);
		tail.poll().await.unwrap().xpect_eq(vec![
			"2027-01-15T08:00:01 booting".to_string(),
			"2027-01-15T08:00:02 GET /about 200".to_string(),
			"2027-01-15T08:00:02 GET / 200".to_string(),
		]);
		tail.poll().await.unwrap().xpect_eq(vec![
			"2027-01-15T08:00:02 GET /blog 200".to_string(),
			"2027-01-15T08:00:03 GET /docs 200".to_string(),
		]);
		calls.lock().unwrap().clone().xpect_eq(vec![
			json!({ "logGroupName": "/ecs/beetmash/prod", "startTime": 1_800_000_000_000_i64 }),
			json!({
				"logGroupName": "/ecs/beetmash/prod",
				"startTime": 1_800_000_000_000_i64,
				"nextToken": "page-2",
			}),
			json!({ "logGroupName": "/ecs/beetmash/prod", "startTime": 1_800_000_002_000_i64 }),
		]);
	}
}
