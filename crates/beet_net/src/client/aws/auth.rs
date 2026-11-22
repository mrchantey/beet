use crate::prelude::*;
use beet_core::prelude::*;

/// Signature Version 4 as a [`RequestAuth`]: a credential source's current
/// credential signing for one service in one region.
///
/// Recovery reads two refusals. A lapsed credential (`ExpiredToken`,
/// `ExpiredTokenException`, `TokenRefreshRequired`) drops the source's cache
/// and retries when the source can answer another. A clock skew
/// (`RequestTimeTooSkewed`, `RequestExpired`, `InvalidSignatureException`
/// naming the time) corrects the process's [`AwsClock`] by the response
/// `Date` and retries, so every later signature carries the server's time.
#[derive(Clone)]
pub struct AwsAuth {
	source: AwsCredentialSource,
	signer: SigV4,
}

impl core::fmt::Debug for AwsAuth {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("AwsAuth")
			.field("source", &self.source)
			.field("signer", &self.signer)
			.finish()
	}
}

impl AwsAuth {
	/// The codes a lapsed credential is refused with.
	pub const EXPIRED: &'static [&'static str] = &[
		"ExpiredToken",
		"ExpiredTokenException",
		"TokenRefreshRequired",
	];
	/// The codes a skewed clock is refused with.
	pub const SKEWED: &'static [&'static str] = &[
		"RequestTimeTooSkewed",
		"RequestExpired",
		"RequestInTheFuture",
	];

	/// `source`'s credential signing as `signer`.
	pub fn new(source: AwsCredentialSource, signer: SigV4) -> Self {
		Self { source, signer }
	}

	/// Whether `failure` is a clock skew refusal.
	fn is_skew(failure: &AuthFailure) -> bool {
		Self::SKEWED.contains(&failure.code.as_str())
			|| (failure.code == "InvalidSignatureException"
				&& (failure.message.contains("Signature expired")
					|| failure.message.contains("Signature not yet current")))
	}
}

impl RequestAuth for AwsAuth {
	fn authorize(&self, request: Request) -> SendBoxedFuture<Result<Request>> {
		let this = self.clone();
		Box::pin(async move {
			let credentials = this.source.credentials().await?;
			this.signer
				.sign(request, &credentials, AwsClock::now())
				.await
		})
	}

	fn recover(&self, failure: &AuthFailure) -> SendBoxedFuture<Result<bool>> {
		let this = self.clone();
		let expired = Self::EXPIRED.contains(&failure.code.as_str());
		let server_time = Self::is_skew(failure)
			.then(|| {
				failure.header("date").and_then(Timestamp::parse_http_date)
			})
			.flatten();
		Box::pin(async move {
			if let Some(server_time) = server_time {
				warn!(
					"AWS refused this machine's clock, correcting it by {}ms to the server's",
					server_time.millis() - Timestamp::now().millis()
				);
				AwsClock::correct(server_time);
				return Ok(true);
			}
			match expired {
				true => this.source.invalidate().await.xok(),
				false => Ok(false),
			}
		})
	}
}

#[cfg(test)]
mod test {
	use super::*;

	fn failure(code: &str, message: &str, date: Option<&str>) -> AuthFailure {
		let mut headers = HeaderMap::new();
		if let Some(date) = date {
			headers.set_raw("date", date);
		}
		AuthFailure {
			status: StatusCode::FORBIDDEN,
			code: code.into(),
			message: message.into(),
			headers,
		}
	}

	fn auth() -> AwsAuth {
		AwsAuth::new(
			AwsCredentials::new("AKID", "secret").into(),
			SigV4::new("us-east-1", "sts"),
		)
	}

	/// A skew refusal corrects the clock to the response's date and retries;
	/// a skew-flavoured signature error counts, another signature error not.
	#[beet_core::test]
	async fn corrects_a_skewed_clock() {
		let date = Timestamp::now().millis() + 3_600_000;
		let http_date = Timestamp::from_millis(date).format_rfc2822();
		auth()
			.recover(&failure("RequestTimeTooSkewed", "", Some(&http_date)))
			.await
			.unwrap()
			.xpect_true();
		(AwsClock::now().millis() - date)
			.abs()
			.xpect_less_than(2_000);
		AwsAuth::is_skew(&failure(
			"InvalidSignatureException",
			"Signature expired: 20150830T123600Z is now earlier than ..",
			None,
		))
		.xpect_true();
		AwsAuth::is_skew(&failure(
			"InvalidSignatureException",
			"The request signature we calculated does not match",
			None,
		))
		.xpect_false();
		AwsClock::correct(Timestamp::now());
	}

	/// A fixed credential that lapsed cannot be refreshed, so it is not
	/// retried; an unrelated refusal never is.
	#[beet_core::test]
	async fn retries_only_what_recovers() {
		auth()
			.recover(&failure("ExpiredToken", "", None))
			.await
			.unwrap()
			.xpect_false();
		auth()
			.recover(&failure("AccessDenied", "", None))
			.await
			.unwrap()
			.xpect_false();
	}
}
