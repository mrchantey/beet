//! AWS CLI process constructors shared by the deploy actions that shell out.
//!
//! beet carries the S3 sdk and nothing else, so every other AWS service a
//! deploy verb touches is reached through the `aws` cli. That is one line of
//! boilerplate each time and one footgun each time, so it lives here once.
use crate::prelude::*;
use beet_core::prelude::*;

/// Creates an `aws <service> <args..> --region <region>` invocation, as the
/// deploy's own credential.
pub fn service<'a>(
	service: &'a str,
	region: &str,
	args: impl IntoIterator<Item = &'a str>,
) -> ChildProcess {
	ChildProcess::new("aws").with_args(
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

/// An `aws iam` invocation, see [`service`]. IAM is global, so the region is
/// the global endpoint's own rather than any stack's.
pub fn iam<'a>(args: impl IntoIterator<Item = &'a str>) -> ChildProcess {
	self::service("iam", crate::bindings::aws::region::US_EAST_1, args)
}

/// An `aws sts` invocation, see [`iam`] for the region.
pub fn sts<'a>(args: impl IntoIterator<Item = &'a str>) -> ChildProcess {
	self::service("sts", crate::bindings::aws::region::US_EAST_1, args)
}

/// An `aws` invocation against R2 at `endpoint`, authenticated as exactly the
/// pair `access_key`/`secret_key` rather than the deploy's credential. An
/// inherited profile and session token are dropped, since a session token
/// beside a long-lived pair authenticates as nobody and a profile may carry
/// settings meant for another identity, and the secret half is redacted from
/// every rendering of the command.
pub fn r2(endpoint: &str, access_key: &str, secret_key: &str) -> ChildProcess {
	ChildProcess::new("aws")
		.without_env("AWS_PROFILE")
		.without_env("AWS_SESSION_TOKEN")
		.with_secret(secret_key)
		.with_envs([
			("AWS_ACCESS_KEY_ID", access_key),
			("AWS_SECRET_ACCESS_KEY", secret_key),
		])
		.with_args([
			"--endpoint-url",
			endpoint,
			"--region",
			CloudflareAccount::R2_REGION,
		])
}
