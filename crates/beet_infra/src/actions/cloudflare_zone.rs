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

/// Purges the edge cache of the hostnames the stack serves: the invalidation
/// step after a deploy or content sync. The edge may hold responses for their
/// full `s-maxage` (see the router's `CacheHeaders`), so anything that changes
/// served content must purge, or the old pages keep serving until the TTL
/// runs out.
///
/// The hostnames are read off the config the stack's blocks emit, every
/// proxied `cloudflare_dns_record` in the stack's zone (a record that is not
/// proxied is never cached), so the purge and the apply cannot disagree about
/// what the stack serves, and a stack sharing its zone with another (the
/// example beside the site, a dev stage beside prod) never cools the other's
/// cache. A stack serving no proxied hostname purges nothing.
///
/// A REST call (`POST zones/{zone}/purge_cache` with `hosts`), not tofu: a
/// purge is an event, not a resource. The zone is the stack's
/// [`CloudflareZone`] and the call authenticates with the deploy token, whose
/// `Cache Purge` group this action declares.
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
	let zone_id = zone.id.clone();
	let hosts = cx
		.caller
		.with_world(move |world, entity| -> Result<_> {
			let (.., config) = RenderScope::render(world, entity)?.finish()?;
			CloudflarePurgeCache::hosts(&config, &zone_id).xok()
		})
		.await??;
	if hosts.is_empty() {
		info!(
			"the stack serves no proxied hostname in {}: nothing to purge",
			zone.domain
		);
		return Pass(cx.input).xok();
	}
	let start = Instant::now();
	cloudflare_api_ext::send(
		Request::post(format!("{API_BASE}/zones/{}/purge_cache", zone.id))
			.with_auth_bearer(&token)
			.with_json_body(&serde_json::json!({ "hosts": hosts }))?,
		"purging the edge cache",
	)
	.await?;
	info!(
		"purged the edge cache of {} in {}",
		hosts.join(", "),
		time_ext::pretty_print_duration(start.elapsed())
	);
	Pass(cx.input).xok()
}

impl CloudflarePurgeCache {
	/// Every proxied hostname `config` publishes in `zone_id`, sorted and
	/// deduplicated.
	fn hosts(config: &terra::Config, zone_id: &str) -> Vec<String> {
		let mut hosts = CloudflareDnsRecords::of(config, Some(zone_id))
			.into_iter()
			.filter(|record| record["proxied"].as_bool() == Some(true))
			.filter_map(|record| record["name"].as_str().map(str::to_string))
			// a name only known after apply is never a served hostname
			.filter(|name| !name.contains("${"))
			.collect::<Vec<_>>();
		hosts.sort();
		hosts.dedup();
		hosts
	}
}

/// The `cloudflare_dns_record` resources a rendered config emits, the one
/// reading of them the zone verbs share, so a purge and an audit see exactly
/// the records the apply writes.
pub(crate) struct CloudflareDnsRecords;

impl CloudflareDnsRecords {
	/// Every record `config` emits, as its json, only those in `zone_id`
	/// when named.
	pub(crate) fn of(
		config: &terra::Config,
		zone_id: Option<&str>,
	) -> Vec<serde_json::Value> {
		config.to_json().into_json()["resource"]["cloudflare_dns_record"]
			.as_object()
			.map(|records| {
				records
					.values()
					.filter(|record| match zone_id {
						Some(zone_id) => {
							record["zone_id"].as_str() == Some(zone_id)
						}
						None => true,
					})
					.cloned()
					.collect()
			})
			.unwrap_or_default()
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// A purge names the stack's proxied hostnames and nothing else: not the
	/// unproxied certificate validation records beside them, and nothing for a
	/// zone the stack does not publish in, so it never cools a neighbour's
	/// cache.
	#[cfg(feature = "lambda_block")]
	#[beet_core::test]
	fn purges_the_proxied_hostnames() {
		let (scope, _dir) =
			RenderScope::test_render_zoned("example.org", |parent| {
				parent.spawn(RepoStoreBlock::test_store());
				parent.spawn(
					LambdaBlock::default()
						.with_dns(
							DnsProvider::cloudflare("www.example.org")
								.with_proxied(true),
						)
						.with_dns(
							DnsProvider::cloudflare("example.org")
								.with_proxied(true),
						),
				);
			});
		let (.., config) = scope.finish().unwrap();
		let records = CloudflareDnsRecords::of(&config, None);
		// the validation records are there, and not proxied
		(records.len() > 2).xpect_true();
		let zone = records[0]["zone_id"].as_str().unwrap().to_string();
		CloudflarePurgeCache::hosts(&config, &zone).xpect_eq(vec![
			"example.org".to_string(),
			"www.example.org".to_string(),
		]);
		CloudflarePurgeCache::hosts(&config, "another-zone")
			.len()
			.xpect_eq(0);
	}

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
		let entity = SceneBody::spawn_bsx(
			&mut world,
			r#"<CloudflareApexRedirect aliases=["www"]/>"#,
		)
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
