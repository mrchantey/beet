//! AWS CLI process constructors shared by the deploy actions that shell out.
//!
//! beet carries the S3 sdk and nothing else, so every other AWS service a
//! deploy verb touches is reached through the `aws` cli. That is one line of
//! boilerplate each time and one footgun each time, so it lives here once.
use beet_core::prelude::*;

/// Creates an `aws <service> <args..> --region <region>` invocation.
///
/// Drops a possibly-empty inherited `AWS_PROFILE`, which the cli reads as a
/// profile literally named `""` and rejects, rather than falling back to the
/// explicit credentials in the environment.
pub fn service<'a>(
	service: &'a str,
	region: &str,
	args: impl IntoIterator<Item = &'a str>,
) -> ChildProcess {
	ChildProcess::new("aws")
		.without_env("AWS_PROFILE")
		.with_args(
			[service]
				.into_iter()
				.chain(args)
				.map(SmolStr::from)
				.chain([SmolStr::from("--region"), SmolStr::from(region)]),
		)
}

/// An `aws ec2` invocation in `region`, see [`service`].
pub fn ec2<'a>(
	region: &str,
	args: impl IntoIterator<Item = &'a str>,
) -> ChildProcess {
	self::service("ec2", region, args)
}

/// An `aws ssm` invocation in `region`, see [`service`].
pub fn ssm<'a>(
	region: &str,
	args: impl IntoIterator<Item = &'a str>,
) -> ChildProcess {
	self::service("ssm", region, args)
}
