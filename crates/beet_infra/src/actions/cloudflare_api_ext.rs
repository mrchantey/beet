//! The Cloudflare v4 api: its base, the credential every call carries, and the
//! envelope every answer is checked through.
//!
//! One place rather than one per action, because Cloudflare answers a failure
//! with `200 {"success": false}` as readily as with a status, so a call that
//! checks only one of the two reads a failure as a success.

use beet_core::prelude::*;
use beet_net::prelude::Request;
use beet_net::prelude::StatusCode;
use serde_json::Value;

/// The Cloudflare v4 api base.
pub const API_BASE: &str = "https://api.cloudflare.com/client/v4";

/// The api token from the environment. Never rendered into a config or passed
/// as an argument: the tofu subprocess inherits it, and a call carries it as a
/// bearer.
///
/// A deploy action reads it through its own
/// [`CloudflareAccess`](crate::prelude::CloudflareAccess), which is what
/// keeps every call it makes lowered into the token; the mint verb is the one
/// direct reader, taking a mint token passed here for one command (as CI
/// would) when it is not the sealed deploy token.
pub(crate) fn token() -> Result<SmolStr> {
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
		return Err(CloudflareApiError {
			what: what.into(),
			status,
			body,
		}
		.into());
	}
	Some(json).xok()
}

/// A Cloudflare call that answered a failing status or a `success: false`
/// envelope, typed so a caller can tell a refused credential from anything
/// else by [`BevyError::downcast_ref`] rather than by reading the message.
#[derive(Debug, thiserror::Error)]
#[error("{what} failed: {status} - {body}")]
pub struct CloudflareApiError {
	/// What was being attempted.
	pub what: String,
	/// The status it answered.
	pub status: StatusCode,
	/// The body it answered, an envelope naming its errors.
	pub body: String,
}

impl CloudflareApiError {
	/// Whether the credential itself was refused: malformed, unknown, or
	/// lacking the permission the call needs.
	pub fn refused_credential(&self) -> bool {
		matches!(self.status.as_u16(), 400 | 401 | 403)
	}

	/// The envelope's `errors[].message` list, joined, for a line that names
	/// what Cloudflare said without quoting its whole answer.
	pub fn messages(&self) -> String {
		error_messages(&serde_json::from_str(&self.body).unwrap_or_default())
	}
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
