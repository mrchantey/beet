//! The AWS account these credentials belong to.
//!
//! Identity rather than storage: who the launch is, which is what lets a name
//! be DERIVED from the account instead of declared per project. See
//! `S3Backend`'s default state bucket in `beet_infra`.
use beet_core::prelude::*;

/// The account id of whatever credentials this process resolves
/// (`sts:GetCallerIdentity`), read once per process.
///
/// A launch's account cannot change under it, so the first caller pays the
/// round trip and every later one reads the cache; a failure is not cached, so
/// a launch that gains credentials mid-flight resolves on the next attempt.
pub async fn account_id() -> Result<SmolStr> {
	static ACCOUNT_ID: async_lock::OnceCell<SmolStr> =
		async_lock::OnceCell::new();
	ACCOUNT_ID
		.get_or_try_init(|| {
			async_ext::pin_tokio(async move {
				let mut loader = aws_config::from_env();
				// STS answers for the whole partition from us-east-1, so a
				// launch that names no region gets one here rather than leaving
				// the sdk's region chain to probe instance metadata, a one
				// second timeout on every machine that is not an EC2 box
				if env_ext::var("AWS_REGION").is_err()
					&& env_ext::var("AWS_DEFAULT_REGION").is_err()
				{
					loader =
						loader.region(aws_config::Region::new("us-east-1"));
				}
				let config = loader.load().await;
				aws_sdk_sts::Client::new(&config)
					.get_caller_identity()
					.send()
					.await?
					.account
					.ok_or_else(|| {
						bevyhow!(
							"sts:GetCallerIdentity answered without an account"
						)
					})?
					.xmap(SmolStr::new)
					.xok()
			})
		})
		.await
		.cloned()
}
