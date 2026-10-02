//! Zone-level Cloudflare API actions for the deploy lifecycle.
//!
//! Deliberately REST, not terraform: entrypoint rulesets are zone singletons
//! that fight stack-scoped terraform state, and the zone APIs are idempotent
//! by design (entrypoint PUT, settings PATCH), so every stage's deploy
//! converges the shared zone safely. (Historically the same reasoning covered
//! Spectrum apps: the plan-polymorphic Spectrum API also rejects the terraform
//! provider's Enterprise-only fields.)
use crate::actions::cloudflare_api_ext;
use crate::actions::cloudflare_api_ext::API_BASE;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Publishes the zone-level edge config after an apply, converging the shared
/// zone from any stage's deploy:
/// - PUT the `http_request_cache_settings` entrypoint ruleset: everything is
///   cache-eligible with origin-controlled TTLs (the router's `CacheHeaders`
///   owns policy), except content-negotiated non-html requests, which bypass
///   the cache (edges key on the URL alone, so a markdown `Accept` must never
///   be answered by a cached html body).
/// - PATCH the `ssl` setting to `strict`: the origin presents a publicly
///   trusted ACM cert, so the edge-to-origin leg verifies it.
/// - PATCH `always_use_https` on: the `ssl` mode governs the HTTPS leg only, so
///   a plain http request is otherwise forwarded to the origin on port 80. An
///   https-only origin (an api gateway custom domain) answers nothing there, so
///   without this http://<site> is a 521. Redirecting at the edge also means the
///   request never reaches the origin at all.
#[action]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn CloudflareZoneSetup(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let (zone_id, token) = cloudflare_api_ext::zone_auth(&cx.caller).await?;

	// the cache ruleset: an entrypoint PUT creates or replaces, idempotent
	cloudflare_api_ext::send(
		Request::put(format!(
			"{API_BASE}/zones/{zone_id}/rulesets/phases/http_request_cache_settings/entrypoint"
		))
		.with_auth_bearer(&token)
		.with_json_body(&cache_rules())?,
		"publishing the edge cache ruleset",
	)
	.await?;
	info!("published the edge cache ruleset");

	// strict TLS on the edge-to-origin leg
	cloudflare_api_ext::send(
		Request::patch(format!("{API_BASE}/zones/{zone_id}/settings/ssl"))
			.with_auth_bearer(&token)
			.with_json_body(&serde_json::json!({ "value": "strict" }))?,
		"setting the zone ssl mode",
	)
	.await?;
	info!("zone ssl mode is strict");

	// http never reaches the origin: the edge redirects it to https first
	cloudflare_api_ext::send(
		Request::patch(format!(
			"{API_BASE}/zones/{zone_id}/settings/always_use_https"
		))
		.with_auth_bearer(&token)
		.with_json_body(&serde_json::json!({ "value": "on" }))?,
		"redirecting http to https at the edge",
	)
	.await?;
	info!("zone redirects http to https");

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
/// event, not a resource. The zone is the stack's [`CloudflareZone`] and the
/// call authenticates with `CLOUDFLARE_API_TOKEN` (needs the `Cache Purge`
/// permission).
#[action]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn CloudflarePurgeCache(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let (zone_id, token) = cloudflare_api_ext::zone_auth(&cx.caller).await?;
	let start = Instant::now();
	cloudflare_api_ext::send(
		Request::post(format!("{API_BASE}/zones/{zone_id}/purge_cache"))
			.with_auth_bearer(&token)
			.with_json_body(&serde_json::json!({ "purge_everything": true }))?,
		"purging the zone cache",
	)
	.await?;
	info!(
		"purged zone cache in {}",
		time_ext::pretty_print_duration(start.elapsed())
	);
	Pass(cx.input).xok()
}
