//! The Cloudflare v4 api: its base, the credential every call carries, and the
//! envelope every answer is checked through.
//!
//! One place rather than one per action, because Cloudflare answers a failure
//! with `200 {"success": false}` as readily as with a status, so a call that
//! checks only one of the two reads a failure as a success.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::Request;
use serde_json::Value;

/// The Cloudflare v4 api base.
pub const API_BASE: &str = "https://api.cloudflare.com/client/v4";

/// The api token from the environment, which is the auth of every call here.
/// Never rendered into a config or passed as an argument: the tofu subprocess
/// and `wrangler` inherit it, and the calls below carry it as a bearer.
pub fn token() -> Result<SmolStr> {
	env_ext::var("CLOUDFLARE_API_TOKEN")
		.map_err(|_| bevyhow!("CLOUDFLARE_API_TOKEN is unset"))
}

/// The S3 secret access key an api token value derives to: its lowercase hex
/// SHA-256, which is what R2 expects of any token used over the S3 api and the
/// only half of the pair that is computed rather than read. Pinned by a test,
/// because a derivation that changed shape would park a credential that
/// authenticates nowhere and nothing would say so until a copy failed.
pub fn derive_secret_key(token_value: &str) -> String {
	digest_ext::hex::<sha2::Sha256>(token_value.as_bytes())
}

/// The S3 credentials for R2 derived from the api token in the environment,
/// in `account`: a token's id IS the access key id and the lowercase hex
/// SHA-256 of its value IS the secret access key
/// ([`derive_secret_key`]). So an R2 data-plane credential is a
/// DERIVATION of the one credential a Cloudflare command already carries,
/// never a second credential anybody holds, which is the whole reason this
/// exists: a hand-made pair with object write over every bucket in the account
/// is final in its damage (R2 keeps no versions), and a document that sealed
/// one would hold a credential the age key must not open.
///
/// An `R2BucketBlock` relies on the same two halves for a bucket's own parked
/// token, where the id is already known from the mint; here it is read back
/// with the token's own verify, the one call a token may always make about
/// itself. Account-owned, which is what the account's api-token pages mint
/// (the R2 page included) and the only kind that verify answers for.
pub async fn r2_credentials(account: &str) -> Result<(String, String)> {
	let token = token()?;
	let verified = send(
		Request::get(format!("{API_BASE}/accounts/{account}/tokens/verify"))
			.with_auth_bearer(&token),
		"verifying the api token to derive its R2 S3 pair (account-owned, \
		which a token made under My Profile is not)",
	)
	.await?;
	match verified["result"]["id"].as_str() {
		Some(id) => (id.to_string(), derive_secret_key(&token)).xok(),
		None => bevybail!(
			"the api token's verify answered no id, so no R2 S3 pair could be \
			derived"
		),
	}
}

/// The zone id `caller` resolves by ancestry
/// ([`ResolvedStack::cloudflare_zone`]) and the token beside it, the auth every
/// zone-level call needs.
pub async fn zone_auth(caller: &AsyncEntity) -> Result<(SmolStr, SmolStr)> {
	let zone_id = caller
		.with_state::<StackQuery, _>(|entity, stacks| {
			stacks
				.resolve(entity)
				.cloudflare_zone()
				.map(|zone| zone.id.clone())
		})
		.await??;
	Ok((zone_id, token()?))
}

/// Send `request`, failing on a non-2xx status or a `success: false` envelope
/// and naming `what` was being attempted, and answer the parsed body.
pub async fn send(request: Request, what: &str) -> Result<Value> {
	send_optional(request, what)
		.await?
		.ok_or_else(|| bevyhow!("{what} failed: the resource is not there"))
}

/// [`send`] answering [`None`] for a `404`, for a call that addresses something
/// a teardown may already have removed or a converge has not created yet. A
/// `204` answers [`Value::Null`], since an empty body is no envelope.
pub async fn send_optional(
	request: Request,
	what: &str,
) -> Result<Option<Value>> {
	let response = request.send().await?;
	let status = response.status();
	let body = response.text().await.unwrap_or_default();
	if status.as_u16() == 404 {
		return None.xok();
	}
	if status.is_ok() && body.trim().is_empty() {
		return Some(Value::Null).xok();
	}
	let json = serde_json::from_str::<Value>(&body).unwrap_or_default();
	if !status.is_ok() || json["success"] != true {
		bevybail!("{what} failed: {status} - {body}");
	}
	Some(json).xok()
}

/// The `errors[].message` list of a Cloudflare answer, joined, for a failure
/// whose body must not be quoted: a response that carries a minted secret is
/// reported through this rather than through [`send`].
pub fn error_messages(body: &Value) -> String {
	body["errors"]
		.as_array()
		.map(|errors| {
			errors
				.iter()
				.filter_map(|error| error["message"].as_str())
				.collect::<Vec<_>>()
				.join("; ")
		})
		.filter(|joined| !joined.is_empty())
		.unwrap_or_else(|| "no message".to_string())
}

#[cfg(test)]
mod tests {
	use super::*;

	/// The secret half of an R2 S3 pair is a derivation, so it is pinned:
	/// lowercase hex SHA-256 of the token value, the standard digest R2
	/// expects, checked against a published vector rather than against itself.
	#[beet_core::test]
	fn the_secret_key_is_the_hex_sha256_of_the_token() {
		derive_secret_key("test").xpect_eq(
			"9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
		);
	}
}
