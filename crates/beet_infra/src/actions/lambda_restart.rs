//! `<LambdaRestart/>`: a function's warm execution environments retired.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// `<LambdaRestart/>`: after a content sync, retire the function's warm
/// execution environments, so the next request boots afresh from the repo
/// store.
///
/// The Lambda counterpart of `<LightsailRestart/>`, for the same reason: a
/// page's bytes are read from the store per request, but a boot resolves some
/// things ONCE: `<RoutesDir/>` discovery, the entry document itself, and the
/// derived files a declaration reads when it is declared (a
/// `<StandardSitePub>`'s index). A warm environment keeps the world it built
/// until Lambda retires it, so without this a sync's new page or index serves
/// only once the function happens to go cold.
///
/// Lambda has no restart call, so this points the function at the code it
/// already runs, the live version's artifact ([`AdoptCurrentDeploy`] names
/// it), and waits for the update to land: every invocation after it runs in a
/// fresh environment. The bucket, key and code are the ones the stack's tofu
/// state holds, so the next plan sees no drift.
///
/// Declared under the sync sequence after the [`HomeSync`] that publishes and
/// before the `<CloudflarePurgeCache/>`: purging while the old environments
/// still serve re-caches their responses.
#[action]
#[derive(Debug, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn LambdaRestart(
	/// The function's block label, the [`LambdaBlock`] default unless its
	/// stack declares another.
	#[field(default = LambdaBlock::default().label().clone())]
	label: SmolStr,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let (region, function, bucket, key) = cx
		.caller
		.with_state::<(StackQuery, HomeStoreQuery), _>(
			move |entity, (stacks, repos)| -> Result<_> {
				let stack = stacks.resolve(entity);
				let block = LambdaBlock::default().with_label(label);
				(
					stack.aws_region()?.clone(),
					block.ident(&stack).primary_identifier().to_string(),
					repos.get(entity)?.bucket_name()?,
					block.code_key(&stacks.deploy_id()),
				)
					.xok()
			},
		)
		.await??;
	LambdaFunction::new(
		AwsClient::new(AwsService::LAMBDA, region),
		function.clone(),
	)
	.update_code(&bucket, key.as_str())
	.await?
	// the update is asynchronous: an invocation before it lands still
	// meets a warm environment
	.await_update(LambdaFunction::POLL_INTERVAL)
	.await?;
	info!("restarted {function}: the next request boots from the home store");
	Pass(cx.input).xok()
}

/// One Lambda function, by name or arn, in its client's region.
struct LambdaFunction {
	client: AwsClient,
	name: String,
}

impl LambdaFunction {
	/// How often [`await_update`](Self::await_update) asks.
	const POLL_INTERVAL: Duration = Duration::from_secs(2);
	/// How many times [`await_update`](Self::await_update) asks before giving
	/// up, ~5 minutes at [`POLL_INTERVAL`](Self::POLL_INTERVAL).
	const POLL_ATTEMPTS: u32 = 150;

	fn new(client: AwsClient, name: impl Into<String>) -> Self {
		Self {
			client,
			name: name.into(),
		}
	}

	/// `lambda:UpdateFunctionCode`, pointing the function at the S3 object
	/// `bucket`/`key`.
	async fn update_code(&self, bucket: &str, key: &str) -> Result<&Self> {
		self.client
			.send_json(
				AwsRequest::put(
					"UpdateFunctionCode",
					format!("/2015-03-31/functions/{}/code", self.name),
				)
				.with_json(&value!({ "S3Bucket": bucket, "S3Key": key }))?,
			)
			.await?;
		self.xok()
	}

	/// Poll `lambda:GetFunctionConfiguration` every `interval` until the last
	/// update is `Successful`, erroring on `Failed` with its reason, or when it
	/// is still in progress after [`POLL_ATTEMPTS`](Self::POLL_ATTEMPTS).
	async fn await_update(&self, interval: Duration) -> Result {
		let request = || {
			AwsRequest::get(
				"GetFunctionConfiguration",
				format!("/2015-03-31/functions/{}/configuration", self.name),
			)
		};
		for _ in 0..Self::POLL_ATTEMPTS {
			let config = self.client.send_json(request()).await?;
			match config["LastUpdateStatus"].as_str().ok() {
				Some("Successful") => return Ok(()),
				Some("Failed") => bevybail!(
					"{}: the code update failed: {}",
					self.name,
					config["LastUpdateStatusReason"]
						.as_str()
						.ok()
						.unwrap_or("no reason given")
				),
				_ => time_ext::sleep(interval).await,
			}
		}
		bevybail!(
			"{}: the code update did not land within {:?}",
			self.name,
			interval * Self::POLL_ATTEMPTS
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::sync::Arc;
	use std::sync::Mutex;

	/// Every request a [`function`] sent: its method, path and json body.
	type Calls = Arc<Mutex<Vec<(HttpMethod, String, Value)>>>;

	const ARN: &str = "arn:aws:lambda:ap-southeast-2:123456789012:function:beetmash-prod--function";

	/// The function [`ARN`] whose Lambda answers `statuses` in turn, the
	/// `LastUpdateStatus` of each configuration read.
	fn function(statuses: Vec<&'static str>) -> (LambdaFunction, Calls) {
		let calls = Calls::default();
		let recorded = calls.clone();
		let statuses = Arc::new(Mutex::new(statuses.into_iter()));
		let client = AwsClient::new(AwsService::LAMBDA, "ap-southeast-2")
			.with_credentials(AwsCredentials::new("AKID", "secret"))
			.with_transport(move |request| {
				let (recorded, statuses) = (recorded.clone(), statuses.clone());
				Box::pin(async move {
					let method = *request.method();
					let path = request.url().to_string();
					let bytes = request.body.into_bytes().await?;
					let body = match bytes.is_empty() {
						true => Value::Null,
						false => json_ext::from_slice(&bytes)?,
					};
					recorded.lock().unwrap().push((method, path, body));
					let status =
						statuses.lock().unwrap().next().unwrap_or("InProgress");
					Response::ok()
						.with_body(
							value!({
								"FunctionName": "beetmash-prod--function",
								"State": "Active",
								"LastUpdateStatus": status,
								"LastUpdateStatusReason": "the zip is not a zip",
							})
							.to_json_string(),
						)
						.xok()
				})
			});
		(LambdaFunction::new(client, ARN), calls)
	}

	/// The update is a PUT of the bucket and key to the function's code, then
	/// configuration reads until the update lands.
	#[beet_core::test]
	async fn restarts_until_successful() {
		let (function, calls) = function(vec![
			"InProgress",
			"InProgress",
			"InProgress",
			"Successful",
		]);
		function
			.update_code("beetmash-prod--repo", "deploys/abc/lambda.zip")
			.await
			.unwrap()
			.await_update(Duration::ZERO)
			.await
			.unwrap();
		let calls = calls.lock().unwrap().clone();
		calls.len().xpect_eq(4);
		calls[0].clone().xpect_eq((
			HttpMethod::Put,
			format!("https://lambda.ap-southeast-2.amazonaws.com/2015-03-31/functions/{ARN}/code"),
			value!({ "S3Bucket": "beetmash-prod--repo", "S3Key": "deploys/abc/lambda.zip" }),
		));
		calls[3].clone().xpect_eq((
			HttpMethod::Get,
			format!("https://lambda.ap-southeast-2.amazonaws.com/2015-03-31/functions/{ARN}/configuration"),
			Value::Null,
		));
	}

	/// A failed update errors with its reason rather than waiting it out.
	#[beet_core::test]
	async fn a_failed_update_errors() {
		let (function, _) = function(vec!["Failed"]);
		function
			.await_update(Duration::ZERO)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("the zip is not a zip");
	}
}
