use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Tails CloudWatch logs for a given log group.
/// Use [`AwsWatch::for_lambda`] or [`AwsWatch::for_lightsail`]
/// for convenient construction from a [`Stack`].
#[derive(Debug, Clone, Get, SetWith, Component)]
#[require(AwsWatchAction)]
pub struct AwsWatch {
	/// The CloudWatch log group to tail.
	log_group: SmolStr,
	/// Optional timeout after which the tail process is killed.
	/// When `None`, follows indefinitely until interrupted.
	#[set_with(unwrap_option)]
	timeout: Option<Duration>,
}

impl AwsWatch {
	pub fn new(log_group: impl Into<SmolStr>) -> Self {
		Self {
			log_group: log_group.into(),
			timeout: None,
		}
	}

	/// Create an [`AwsWatch`] for a Lambda function's CloudWatch log group.
	/// The log group follows the AWS convention `/aws/lambda/{function-name}`.
	pub fn for_lambda(stack: &Stack, block: &LambdaBlock) -> Self {
		let func_ident =
			stack.resource_ident(format!("{}--function", block.label()));
		Self::new(format!(
			"/aws/lambda/{}",
			func_ident.primary_identifier()
		))
	}

	/// Create an [`AwsWatch`] for a Lightsail instance's CloudWatch log group.
	/// Uses the convention `/{app-name}/{label}/{stage}`.
	pub fn for_lightsail(stack: &Stack, block: &LightsailBlock) -> Self {
		Self::new(format!("/{}/{}/{}", stack.app_name(), block.label(), stack.stage()))
	}
}

/// Tails CloudWatch logs via `aws logs tail --follow`.
/// Reads the log group from the sibling [`AwsWatch`] component
/// and the AWS region from the nearest ancestor [`Stack`].
#[action]
#[derive(Default, Component)]
pub async fn AwsWatchAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let watch = cx.caller.get_cloned::<AwsWatch>().await?;
	let region = cx
		.caller
		.with_state::<AncestorQuery<&Stack>, _>(|entity, query| {
			query.get(entity).map(|stack| stack.aws_region().clone())
		})
		.await?;

	info!(
		"tailing CloudWatch log group: {} (region: {region})",
		watch.log_group()
	);

	// spawn aws logs tail with inherited stdout/stderr for streaming output
	let mut child = ChildProcess::new("aws")
		.with_args([
			"logs",
			"tail",
			watch.log_group().as_str(),
			"--follow",
			"--region",
			region.as_str(),
			"--format",
			"short",
		])
		.spawn()?;

	// if timeout is set, wait then kill; otherwise follow indefinitely
	if let Some(timeout) = watch.timeout() {
		time_ext::sleep(*timeout).await;
		child.kill().ok();
		info!("watch timed out after {timeout:?}");
	} else {
		let status = child.status().await?;
		if !status.success() {
			bevybail!("aws logs tail exited with status: {status}");
		}
	}

	Pass(cx.input).xok()
}
