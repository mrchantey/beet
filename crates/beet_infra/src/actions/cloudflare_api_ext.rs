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
