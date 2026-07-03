//! Zone-level Cloudflare API actions for the deploy lifecycle.
//!
//! Deliberately REST, not terraform: entrypoint rulesets are zone singletons
//! that fight stack-scoped terraform state, and the zone APIs are idempotent
//! by design (entrypoint PUT, settings PATCH), so every stage's deploy
//! converges the shared zone safely. (Historically the same reasoning covered
//! Spectrum apps: the plan-polymorphic Spectrum API also rejects the terraform
//! provider's Enterprise-only fields.)
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// The Cloudflare v4 API base.
const API_BASE: &str = "https://api.cloudflare.com/client/v4";

/// The zone id + api token from the environment, the auth every zone call needs.
fn zone_env() -> Result<(String, String)> {
	let zone_id = env_ext::var("CLOUDFLARE_ZONE_ID")
		.map_err(|_| bevyhow!("CLOUDFLARE_ZONE_ID is unset"))?;
	let token = env_ext::var("CLOUDFLARE_API_TOKEN")
		.map_err(|_| bevyhow!("CLOUDFLARE_API_TOKEN is unset"))?;
	Ok((zone_id, token))
}

/// Send `request`, failing on a non-2xx or `success: false` envelope, and
/// return the parsed body.
async fn send_zone_request(request: Request) -> Result<serde_json::Value> {
	let response = request.send().await?;
	let status = response.status();
	let body = response.text().await.unwrap_or_default();
	let json: serde_json::Value =
		serde_json::from_str(&body).unwrap_or_default();
	if !status.is_ok() || json["success"] != true {
		bevybail!("cloudflare zone call failed: {status} - {body}");
	}
	Ok(json)
}

/// Publishes the zone-level edge config after an apply, converging the shared
/// zone from any stage's deploy:
/// - PUT the `http_request_cache_settings` entrypoint ruleset: everything is
///   cache-eligible with origin-controlled TTLs (the router's `CacheHeaders`
///   owns policy), except content-negotiated non-html requests, which bypass
///   the cache (edges key on the URL alone, so a markdown `Accept` must never
///   be answered by a cached html body).
/// - PATCH the `ssl` setting to `strict`: the origin presents a publicly
///   trusted ACM cert, so the edge-to-origin leg verifies it.
#[action(handler_only)]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn CloudflareZoneSetup(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let (zone_id, token) = zone_env()?;

	// the cache ruleset: an entrypoint PUT creates or replaces, idempotent
	send_zone_request(
		Request::put(format!(
			"{API_BASE}/zones/{zone_id}/rulesets/phases/http_request_cache_settings/entrypoint"
		))
		.with_auth_bearer(&token)
		.with_json_body(&cache_rules())?,
	)
	.await?;
	info!("published the edge cache ruleset");

	// strict TLS on the edge-to-origin leg
	send_zone_request(
		Request::patch(format!("{API_BASE}/zones/{zone_id}/settings/ssl"))
			.with_auth_bearer(&token)
			.with_json_body(&serde_json::json!({ "value": "strict" }))?,
	)
	.await?;
	info!("zone ssl mode is strict");

	Pass(cx.input).xok()
}

/// The entrypoint cache-ruleset body, in override order (later rules win per
/// setting): eligible-with-origin-TTLs first, then the non-html bypass.
fn cache_rules() -> serde_json::Value {
	// requests whose `Accept` names a non-html media type (a terminal asking
	// for markdown) skip the cache; an absent header or one carrying
	// `text/html` / `*/*` (browsers, curl defaults) stays eligible
	const NON_HTML_ACCEPT: &str = r#"any(http.request.headers["accept"][*] ne "") and not any(http.request.headers["accept"][*] contains "text/html") and not any(http.request.headers["accept"][*] contains "*/*")"#;
	serde_json::json!({
		"rules": [
			{
				"action": "set_cache_settings",
				"expression": "true",
				"description": "eligible for cache, the origin Cache-Control decides",
				"action_parameters": {
					"cache": true,
					"edge_ttl": { "mode": "respect_origin" },
					"browser_ttl": { "mode": "respect_origin" },
				},
			},
			{
				"action": "set_cache_settings",
				"expression": NON_HTML_ACCEPT,
				"description": "bypass for content-negotiated non-html requests",
				"action_parameters": { "cache": false },
			},
		],
	})
}

/// Purges the whole Cloudflare zone cache: the invalidation step after a
/// deploy or content sync. The edge may hold responses for their full
/// `s-maxage` (see the router's `CacheHeaders`), so anything that changes
/// served content must purge, or the old pages keep serving until the TTL
/// runs out.
///
/// Zone-wide by design: hostname-scoped purge is Enterprise-only and per-URL
/// purge needs the changed key list, so purge-everything is the simple
/// guarantee. Every stage shares the zone, so a dev sync also cools the prod
/// cache, which repopulates on the next hit.
///
/// A REST call (`POST zones/{zone}/purge_cache`), not tofu: a purge is an
/// event, not a resource. Reads `CLOUDFLARE_ZONE_ID` and authenticates with
/// `CLOUDFLARE_API_TOKEN` (needs the `Cache Purge` permission).
#[action(handler_only)]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn CloudflarePurgeCache(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let (zone_id, token) = zone_env()?;
	let start = Instant::now();
	send_zone_request(
		Request::post(format!("{API_BASE}/zones/{zone_id}/purge_cache"))
			.with_auth_bearer(&token)
			.with_json_body(&serde_json::json!({ "purge_everything": true }))?,
	)
	.await?;
	info!(
		"purged zone cache in {}",
		time_ext::pretty_print_duration(start.elapsed())
	);
	Pass(cx.input).xok()
}
