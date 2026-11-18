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
use crate::prelude::*;
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
///
/// Apex alias redirects are their own action, [`CloudflareApexRedirect`], so
/// a zone publishing none never asks the token for the redirect group.
#[action]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
// the cache-phase entrypoint ruleset, then the `ssl` and `always_use_https`
// settings
#[require(CloudflareAccess = CloudflareAccess::new::<Self>(&[
	TokenPermission::CACHE_SETTINGS_WRITE,
	TokenPermission::ZONE_SETTINGS_WRITE,
]))]
pub async fn CloudflareZoneSetup(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let access = CloudflareAccess::resolve(&cx.caller).await?;
	let (zone, token) = access.zone_auth(&cx.caller).await?;
	let zone_id = &zone.id;

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

/// Converges the zone's `http_request_dynamic_redirect` entrypoint ruleset to
/// exactly the declared aliases, ie `<CloudflareApexRedirect aliases=["www"]/>`:
/// each alias answers a `301` to the same path on the zone apex, query kept,
/// so a page has one address. The PUT replaces the whole ruleset, so an empty
/// list clears every redirect a previous deploy published.
///
/// A redirect rule fires only on proxied traffic, so every alias needs a
/// proxied DNS record of its own, ie the `AAAA 100::` placeholder once no
/// origin backs it.
#[action]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
#[require(CloudflareAccess = CloudflareAccess::new::<Self>(&[
	TokenPermission::DYNAMIC_URL_REDIRECTS_WRITE,
]))]
pub async fn CloudflareApexRedirect(
	/// Subdomain labels redirected to the zone apex, ie `www` for
	/// `www.beet.org`.
	#[field]
	aliases: Vec<SmolStr>,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let access = CloudflareAccess::resolve(&cx.caller).await?;
	let (zone, token) = access.zone_auth(&cx.caller).await?;
	// an entrypoint PUT creates or replaces, idempotent
	cloudflare_api_ext::send(
		Request::put(format!(
			"{API_BASE}/zones/{}/rulesets/phases/http_request_dynamic_redirect/entrypoint",
			zone.id
		))
		.with_auth_bearer(&token)
		.with_json_body(&redirect_rules(&zone, &aliases))?,
		"publishing the apex alias redirects",
	)
	.await?;
	match aliases.is_empty() {
		true => info!("cleared the apex alias redirects"),
		false => {
			info!("redirecting {} to {}", aliases.join(", "), zone.domain)
		}
	}
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

/// The entrypoint redirect-ruleset body: one rule answering every alias of
/// the zone apex with a `301` to the same path on the apex, query kept, or no
/// rules at all when there are no aliases.
fn redirect_rules(
	zone: &CloudflareZone,
	aliases: &[SmolStr],
) -> serde_json::Value {
	if aliases.is_empty() {
		return serde_json::json!({ "rules": [] });
	}
	let hosts = aliases
		.iter()
		.map(|alias| format!("\"{alias}.{}\"", zone.domain))
		.collect::<Vec<_>>()
		.join(" ");
	serde_json::json!({
		"rules": [{
			"action": "redirect",
			"expression": format!("http.host in {{{hosts}}}"),
			"description": format!("apex aliases to {}", zone.domain),
			"action_parameters": {
				"from_value": {
					"status_code": 301,
					"target_url": {
						"expression": format!(
							"concat(\"https://{}\", http.request.uri.path)",
							zone.domain
						),
					},
					"preserve_query_string": true,
				},
			},
		}],
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
/// call authenticates with the deploy token, whose `Cache Purge` group this
/// action declares.
#[action]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
// `POST zones/{zone}/purge_cache`
#[require(CloudflareAccess = CloudflareAccess::new::<Self>(&[
	TokenPermission::CACHE_PURGE,
]))]
pub async fn CloudflarePurgeCache(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let access = CloudflareAccess::resolve(&cx.caller).await?;
	let (zone, token) = access.zone_auth(&cx.caller).await?;
	let zone_id = &zone.id;
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

#[cfg(test)]
mod test {
	use super::*;

	/// Each alias answers a `301` to the same path on the apex, query kept.
	#[beet_core::test]
	fn redirects_aliases_to_the_apex() {
		redirect_rules(&CloudflareZone::new("beet.org", "zone"), &[
			"www".into(),
			"old".into(),
		])
		.xpect_eq(serde_json::json!({
			"rules": [{
				"action": "redirect",
				"expression": r#"http.host in {"www.beet.org" "old.beet.org"}"#,
				"description": "apex aliases to beet.org",
				"action_parameters": {
					"from_value": {
						"status_code": 301,
						"target_url": {
							"expression": r#"concat("https://beet.org", http.request.uri.path)"#,
						},
						"preserve_query_string": true,
					},
				},
			}],
		}));
	}

	/// A site authors its aliases as a markup list.
	#[beet_core::test]
	fn authors_aliases_in_markup() {
		let mut world = InfraPlugin.into_world();
		world.init_resource::<PackageConfig>();
		let entity = BsxTemplate::parse_entry(
			&world,
			r#"<CloudflareApexRedirect aliases=["www"]/>"#,
		)
		.unwrap()
		.spawn(&mut world)
		.unwrap();
		world
			.entity(entity)
			.get::<CloudflareApexRedirect>()
			.unwrap()
			.aliases
			.xpect_eq(vec![SmolStr::from("www")]);
	}

	/// No aliases converges to no rules, clearing any a deploy published.
	#[beet_core::test]
	fn no_aliases_clears_the_redirects() {
		redirect_rules(&CloudflareZone::new("beet.org", "zone"), &[])
			.xpect_eq(serde_json::json!({ "rules": [] }));
	}
}
