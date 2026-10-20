//! What every AWS SDK client in beet starts from, and the account its
//! credentials belong to.
//!
//! Identity rather than storage: who the launch is, which is what lets a name
//! be DERIVED from the account instead of declared per project. See
//! `S3Backend`'s default state bucket in `beet_infra`.
use beet_core::prelude::*;

/// How long one attempt may make no progress before the SDK gives up on it:
/// its connect (resolving the name, opening the socket and the tls handshake,
/// which the SDK times as one step) and a stalled stream alike.
///
/// The SDK's own defaults, a 3.1s connect and a 5s stall grace, are outlasted
/// by a resolver stalled on an unreachable nameserver (a router advertising
/// ipv6 nameservers over a link that drops ipv6 stalls a lookup for ~5s), and
/// by a fresh box at zero burst capacity verifying its first handshake.
/// Either then fails all three attempts at the same step, so the call fails as
/// a bare `dispatch failure` ~11s in: the deploy of 2026-10-16 at its first
/// call (`sts:GetCallerIdentity`), and a fresh site box's first request to its
/// repo store.
pub const STALL_TIMEOUT: Duration = Duration::from_secs(10);

/// The config loader every beet SDK client starts from: the environment's
/// (credentials, region, endpoint), patient for [`STALL_TIMEOUT`].
pub fn loader() -> aws_config::ConfigLoader {
	aws_config::from_env()
		.timeout_config(
			aws_config::timeout::TimeoutConfig::builder()
				.connect_timeout(STALL_TIMEOUT)
				.build(),
		)
		.stalled_stream_protection(
			aws_config::stalled_stream_protection::StalledStreamProtectionConfig::enabled()
				.grace_period(STALL_TIMEOUT)
				.build(),
		)
}

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
				let mut loader = loader();
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

#[cfg(test)]
mod test {
	use beet_core::prelude::*;

	/// Every client waits out a stalled lookup (~5s) at both of the steps the
	/// SDK times it at, where its defaults (3.1s, 5s) failed every attempt as
	/// a bare `dispatch failure`.
	#[beet_core::test]
	async fn outlasts_a_stalled_lookup() {
		let config = async_ext::on_tokio(async {
			super::loader()
				// named, so the load never probes for one
				.region(aws_config::Region::new("us-east-1"))
				.load()
				.await
				.xok()
		})
		.await
		.unwrap();
		config
			.timeout_config()
			.unwrap()
			.connect_timeout()
			.unwrap()
			.xpect_greater_than(Duration::from_secs(5));
		config
			.stalled_stream_protection()
			.unwrap()
			.grace_period()
			.xpect_greater_than(Duration::from_secs(5));
	}
}
