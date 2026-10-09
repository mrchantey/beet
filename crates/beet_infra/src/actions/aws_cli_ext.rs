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

/// An `aws <args..>` invocation against R2 at `endpoint`, authenticated as
/// exactly the pair `access_key`/`secret_key` rather than the deploy's
/// credential. An inherited profile and session token are dropped, since a
/// session token beside a long-lived pair authenticates as nobody and a
/// profile may carry settings meant for another identity, and the secret half
/// is redacted from every rendering of the command.
///
/// The args are taken here rather than set after, like every constructor in
/// this module: [`ChildProcess::with_args`] replaces, so args set on the
/// result would drop the endpoint and send the call to AWS S3 instead.
pub fn r2<'a>(
	endpoint: &str,
	access_key: &str,
	secret_key: &str,
	args: impl IntoIterator<Item = &'a str>,
) -> ChildProcess {
	ChildProcess::new("aws")
		.without_env("AWS_PROFILE")
		.without_env("AWS_SESSION_TOKEN")
		.with_secret(secret_key)
		.with_envs([
			("AWS_ACCESS_KEY_ID", access_key),
			("AWS_SECRET_ACCESS_KEY", secret_key),
		])
		.with_args(args.into_iter().map(SmolStr::from).chain([
			SmolStr::from("--endpoint-url"),
			SmolStr::from(endpoint),
			SmolStr::from("--region"),
			SmolStr::from(CloudflareAccount::R2_REGION),
		]))
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// An R2 call keeps its endpoint and its region whatever it runs, and
	/// never prints the secret half.
	///
	/// REGRESSION: the endpoint was set with `with_args` and every caller then
	/// set its own command the same way, which replaced the endpoint: each R2
	/// call (the cold copy's list, push and probe, an example's bucket empty)
	/// went to AWS S3 with an R2 pair.
	#[beet_core::test]
	fn an_r2_call_keeps_its_endpoint() {
		aws_cli_ext::r2(
			"https://acct.r2.cloudflarestorage.com",
			"id",
			"hunter2",
			["s3", "ls"],
		)
		.to_string()
		.xpect_starts_with("aws s3 ls")
		.xpect_contains("--endpoint-url https://acct.r2.cloudflarestorage.com")
		.xpect_contains("--region auto")
		.xnot()
		.xpect_contains("hunter2");
	}
}
