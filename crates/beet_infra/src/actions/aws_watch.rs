use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Tails the CloudWatch logs of a [`WatchTarget`], resolved against the nearest
/// ancestor [`Stack`] when the tail runs, so a watch verb declares WHAT it
/// follows and never restates the app identity.
#[derive(Debug, Clone, Default, Get, SetWith, Component, Reflect)]
#[reflect(Component, Default)]
#[require(AwsWatchAction)]
pub struct AwsWatch {
	/// Which deployed thing's logs to follow.
	target: WatchTarget,
	/// Optional timeout after which the tail process is killed.
	/// When `None`, follows indefinitely until interrupted.
	#[set_with(unwrap_option)]
	timeout: Option<Duration>,
}

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
	pub fn new(log_group: impl Into<SmolStr>) -> Self {
		Self {
			target: WatchTarget::LogGroup(log_group.into()),
			timeout: None,
		}
	}

	pub fn for_target(target: WatchTarget) -> Self {
		Self {
			target,
			timeout: None,
		}
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
	let timeout = *watch.timeout();
	let (region, log_group) = cx
		.caller
		.with_state::<StackQuery, _>(move |entity, query| {
			let stack = query.resolve(entity);
			(stack.region().clone(), watch.target().log_group(&stack))
		})
		.await?;

	info!("tailing CloudWatch log group: {log_group} (region: {region})");

	// spawn aws logs tail with inherited stdout/stderr for streaming output.
	// drop a possibly-empty inherited `AWS_PROFILE` (see `build_docker_image`).
	let mut child = ChildProcess::new("aws")
		.without_env("AWS_PROFILE")
		.with_args([
			"logs",
			"tail",
			log_group.as_str(),
			"--follow",
			"--region",
			region.as_str(),
			"--format",
			"short",
		])
		.spawn()?;

	// if timeout is set, wait then kill; otherwise follow indefinitely
	if let Some(timeout) = timeout {
		time_ext::sleep(timeout).await;
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
