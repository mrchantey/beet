use crate::bindings::*;
use crate::prelude::*;
use crate::terra::ResourceDef;
use beet_core::prelude::*;
use serde_json::json;

/// An HTTP failover in front of a primary origin (eg a Fargate NLB) and a
/// fallback origin (eg a Lambda serving the same site bucket), as a Cloudflare
/// Load Balancer: a health monitor, a primary pool and a fallback pool, and the
/// load balancer steering to the fallback when the primary pool is unhealthy.
///
/// ## Why this is a separate, opt-in block
///
/// A Cloudflare Load Balancer only governs **proxied** (orange-cloud) traffic
/// at the edge, so it applies only to hostnames published with
/// `DnsProvider::with_proxied(true)` (ssh on a proxied hostname rides a
/// Spectrum app, see `DnsProvider::with_ssh_spectrum`). SSH cannot fail over
/// (Lambda has no ssh), so the failover is HTTP-only, and it is kept out of
/// the core deploy behind a flag.
#[derive(Debug, Clone, Get, SetWith, Serialize, Deserialize, Component)]
#[component(immutable, on_add = ErasedBlock::on_add::<CloudflareFailoverBlock>)]
pub struct CloudflareFailoverBlock {
	/// Resource label prefix.
	label: SmolStr,
	/// The proxied hostname the load balancer answers, eg `beet.org`.
	hostname: SmolStr,
	/// The Cloudflare zone id (from `CLOUDFLARE_ZONE_ID`).
	zone_id: SmolStr,
	/// The Cloudflare account id the pools belong to (from `CLOUDFLARE_ACCOUNT_ID`).
	account_id: SmolStr,
	/// The primary origin address (a hostname/IP), eg the Fargate NLB `dns_name`.
	primary_origin: SmolStr,
	/// The fallback origin address (a hostname), eg a Lambda gateway host.
	fallback_origin: SmolStr,
	/// Health-check path on the origins.
	health_check_path: SmolStr,
}

impl CloudflareFailoverBlock {
	/// A failover steering `hostname` from `primary_origin` to `fallback_origin`.
	pub fn new(
		hostname: impl Into<SmolStr>,
		primary_origin: impl Into<SmolStr>,
		fallback_origin: impl Into<SmolStr>,
	) -> Self {
		Self {
			label: "failover".into(),
			hostname: hostname.into(),
			zone_id: env_ext::var("CLOUDFLARE_ZONE_ID")
				.unwrap_or_default()
				.into(),
			account_id: env_ext::var("CLOUDFLARE_ACCOUNT_ID")
				.unwrap_or_default()
				.into(),
			primary_origin: primary_origin.into(),
			fallback_origin: fallback_origin.into(),
			health_check_path: "/health".into(),
		}
	}

	fn build_label(&self, suffix: &str) -> String {
		format!("{}--{}", self.label, suffix)
	}

	/// A `cloudflare_load_balancer_pool` (untyped: the generated `origins` binding
	/// is a flat map, not the list-of-origin-blocks the schema needs). Returns the
	/// terraform resource address for the load balancer to reference.
	fn pool(
		&self,
		stack: &Stack,
		config: &mut terra::Config,
		suffix: &str,
		origin_address: &str,
		monitor_address: &str,
	) -> Result<String> {
		let ident = stack.resource_ident(self.build_label(suffix));
		let label = ident.label().to_string();
		config.add_untyped_resource(
			"cloudflare_load_balancer_pool",
			&label,
			&json!({
				"account_id": self.account_id,
				"name": ident.primary_identifier(),
				"monitor": monitor_address,
				"origins": [{
					"name": suffix,
					"address": origin_address,
					"enabled": true,
				}],
			}),
		)?;
		Ok(format!("${{cloudflare_load_balancer_pool.{label}.id}}"))
	}
}

impl Block for CloudflareFailoverBlock {
	fn apply_to_config(
		&self,
		_entity: &EntityRef,
		stack: &Stack,
		config: &mut terra::Config,
	) -> Result {
		ensure_cloudflare_provider(config)?;

		// the health monitor both pools share.
		let monitor = ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("monitor")),
			CloudflareLoadBalancerMonitorDetails {
				account_id: self.account_id.clone(),
				r#type: Some("http".into()),
				method: Some("GET".into()),
				path: Some(self.health_check_path.clone()),
				expected_codes: Some("200".into()),
				interval: Some(60),
				timeout: Some(5),
				..default()
			},
		);
		config.add_resource(&monitor)?;
		let monitor_ref = monitor.field_ref("id");

		let primary_pool = self.pool(
			stack,
			config,
			"primary",
			&self.primary_origin,
			&monitor_ref,
		)?;
		let fallback_pool = self.pool(
			stack,
			config,
			"fallback",
			&self.fallback_origin,
			&monitor_ref,
		)?;

		// the load balancer: serve from the primary pool, fall back when it is
		// unhealthy. Proxied, since a Cloudflare LB only governs edge traffic.
		let lb = ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("lb")),
			CloudflareLoadBalancerDetails {
				name: self.hostname.clone(),
				zone_id: self.zone_id.clone(),
				default_pools: vec![primary_pool.into()],
				fallback_pool: fallback_pool.into(),
				proxied: Some(true),
				enabled: Some(true),
				..default()
			},
		);
		config.add_resource(&lb)?;
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[beet_core::test]
	fn emits_load_balancer_with_primary_and_fallback_pools() {
		let (stack, _dir) = Stack::default_local();
		let mut config = stack.create_config();
		let mut world = World::new();
		CloudflareFailoverBlock::new(
			"site.example",
			"nlb.example",
			"lambda.example",
		)
		// explicit ids: the env-derived defaults are absent in a test run
		.with_zone_id("test-zone")
		.with_account_id("test-account")
		.apply_to_config(&world.spawn(()).as_readonly(), &stack, &mut config)
		.unwrap();
		config
			.to_json()
			.to_string()
			.as_str()
			.xpect_contains("cloudflare_load_balancer")
			.xpect_contains("cloudflare_load_balancer_pool")
			.xpect_contains("cloudflare_load_balancer_monitor")
			// primary + fallback origins, and the lb steers between two pools
			.xpect_contains("nlb.example")
			.xpect_contains("lambda.example")
			.xpect_contains("fallback_pool")
			.xpect_contains("\"proxied\":true");
	}
}
