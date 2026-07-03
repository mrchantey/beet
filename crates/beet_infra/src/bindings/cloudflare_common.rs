//! Auto-generated Terraform provider bindings — do not edit!
//! Auto-generated Terraform provider bindings — do not edit!
//! Auto-generated Terraform provider bindings — do not edit!

#![allow(
	unused_imports,
	non_snake_case,
	non_camel_case_types,
	non_upper_case_globals
)]
#[allow(unused)]
use crate::prelude::*;
#[allow(unused)]
use beet_core::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use serde_json;
use std::collections::BTreeMap as Map;

#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct CloudflareDnsRecordData {
	/// Algorithm.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub algorithm: Option<i64>,
	/// Altitude of location in meters.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub altitude: Option<i64>,
	/// Certificate.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub certificate: Option<SmolStr>,
	/// Digest.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub digest: Option<SmolStr>,
	/// Digest Type.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub digest_type: Option<i64>,
	/// Fingerprint.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub fingerprint: Option<SmolStr>,
	/// Flags for the CAA record.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub flags: Option<SmolStr>,
	/// Key Tag.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub key_tag: Option<i64>,
	/// Degrees of latitude.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub lat_degrees: Option<i64>,
	/// Latitude direction.
	/// Available values: "N", "S".
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub lat_direction: Option<SmolStr>,
	/// Minutes of latitude.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub lat_minutes: Option<i64>,
	/// Seconds of latitude.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub lat_seconds: Option<i64>,
	/// Degrees of longitude.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub long_degrees: Option<i64>,
	/// Longitude direction.
	/// Available values: "E", "W".
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub long_direction: Option<SmolStr>,
	/// Minutes of longitude.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub long_minutes: Option<i64>,
	/// Seconds of longitude.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub long_seconds: Option<i64>,
	/// Matching Type.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub matching_type: Option<i64>,
	/// Order.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub order: Option<i64>,
	/// The port of the service.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub port: Option<i64>,
	/// Horizontal precision of location.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub precision_horz: Option<i64>,
	/// Vertical precision of location.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub precision_vert: Option<i64>,
	/// Preference.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub preference: Option<i64>,
	/// Required for MX, SRV and URI records; unused by other record types. Records with lower priorities are preferred.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub priority: Option<i64>,
	/// Protocol.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub protocol: Option<i64>,
	/// Public Key.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub public_key: Option<SmolStr>,
	/// Regex.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub regex: Option<SmolStr>,
	/// Replacement.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub replacement: Option<SmolStr>,
	/// Selector.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub selector: Option<i64>,
	/// Service.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub service: Option<SmolStr>,
	/// Size of location in meters.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub size: Option<i64>,
	/// Name of the property controlled by this record (e.g.: issue, issuewild, iodef).
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tag: Option<SmolStr>,
	/// Target.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub target: Option<SmolStr>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub r#type: Option<i64>,
	/// Usage.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub usage: Option<i64>,
	/// Value of the record. This field's semantics depend on the chosen tag.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub value: Option<SmolStr>,
	/// The record weight.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub weight: Option<i64>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct CloudflareDnsRecordDetails {
	/// Comments or notes about the DNS record. This field has no effect on DNS responses.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub comment: Option<SmolStr>,
	/// When the record comment was last modified. Omitted if there is no comment.
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub comment_modified_on: Option<SmolStr>,
	/// A valid IPv4 address.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub content: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub count: Option<i64>,
	/// When the record was created.
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub created_on: Option<SmolStr>,
	/// Components of a CAA record.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub data: Option<CloudflareDnsRecordData>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub depends_on: Option<Vec<SmolStr>>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub for_each: Option<Vec<SmolStr>>,
	/// Identifier.
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<SmolStr>,
	/// Extra Cloudflare-specific information about the record.
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub meta: Option<SmolStr>,
	/// When the record was last modified.
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub modified_on: Option<SmolStr>,
	/// DNS record name (or @ for the zone apex) in Punycode.
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub name: SmolStr,
	/// Required for MX, SRV and URI records; unused by other record types. Records with lower priorities are preferred.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub priority: Option<i64>,
	/// Enables private network routing to the origin.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub private_routing: Option<bool>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub provider: Option<SmolStr>,
	/// Whether the record can be proxied by Cloudflare or not.
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub proxiable: Option<bool>,
	/// Whether the record is receiving the performance and security benefits of Cloudflare.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub proxied: Option<bool>,
	/// Settings for the DNS record.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub settings: Option<CloudflareDnsRecordSettings>,
	/// Custom tags for the DNS record. This field has no effect on DNS responses.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tags: Option<Vec<SmolStr>>,
	/// When the record tags were last modified. Omitted if there are no tags.
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tags_modified_on: Option<SmolStr>,
	/// Time To Live (TTL) of the DNS record in seconds. Setting to 1 means 'automatic'. Value must be between 60 and 86400, with the minimum reduced to 30 for Enterprise zones.
	/// ## Attribute
	/// `required`
	pub ttl: i64,
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub r#type: SmolStr,
	/// Identifier.
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub zone_id: SmolStr,
}
impl terra::ToJson for CloudflareDnsRecordDetails {
	fn to_json(&self) -> serde_json::Value {
		serde_json::to_value(self).expect("serialization should not fail")
	}
}
impl terra::Resource for CloudflareDnsRecordDetails {
	fn resource_type(&self) -> &'static str { "cloudflare_dns_record" }
	fn provider(&self) -> &'static terra::Provider {
		&terra::Provider::CLOUDFLARE
	}
	fn validate_definition(
		&self,
	) -> Result<(), terra::ResourceValidationError> {
		if self.comment_modified_on.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "comment_modified_on",
				},
			);
		}
		if self.created_on.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "created_on",
				},
			);
		}
		if self.id.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "id",
				},
			);
		}
		if self.meta.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "meta",
				},
			);
		}
		if self.modified_on.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "modified_on",
				},
			);
		}
		if self.name.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "name",
			});
		}
		if self.proxiable.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "proxiable",
				},
			);
		}
		if self.tags_modified_on.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "tags_modified_on",
				},
			);
		}
		if self.r#type.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "type",
			});
		}
		if self.zone_id.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "zone_id",
			});
		}
		Ok(())
	}
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct CloudflareDnsRecordSettings {
	/// If enabled, causes the CNAME record to be resolved externally and the resulting address records (e.g., A and AAAA) to be returned instead of the CNAME record itself. This setting is unavailable for proxied records, since they are always flattened.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub flatten_cname: Option<bool>,
	/// When enabled, only A records will be generated, and AAAA records will not be created. This setting is intended for exceptional cases. Note that this option only applies to proxied records and it has no effect on whether Cloudflare communicates with the origin using IPv4 or IPv6.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ipv4_only: Option<bool>,
	/// When enabled, only AAAA records will be generated, and A records will not be created. This setting is intended for exceptional cases. Note that this option only applies to proxied records and it has no effect on whether Cloudflare communicates with the origin using IPv4 or IPv6.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ipv6_only: Option<bool>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct CloudflareLoadBalancerAdaptiveRouting {
	/// Extends zero-downtime failover of requests to healthy origins from alternate pools, when no healthy alternate exists in the same pool, according to the failover order defined by traffic and origin steering. When set false (the default) zero-downtime failover will only occur between origins within the same pool. See `session_affinity_attributes` for control over when sessions are broken or reassigned.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub failover_across_pools: Option<bool>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct CloudflareLoadBalancerDetails {
	/// Controls features that modify the routing of requests to pools and origins in response to dynamic conditions, such as during the interval between active health monitoring requests. For example, zero-downtime failover occurs immediately when an origin becomes unavailable due to HTTP 521, 522, or 523 response codes. If there is another healthy origin in the same pool, the request is retried once against this alternate origin.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub adaptive_routing: Option<CloudflareLoadBalancerAdaptiveRouting>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub count: Option<i64>,
	/// A mapping of country codes to a list of pool IDs (ordered by their failover priority) for the given country. Any country not explicitly defined will fall back to using the corresponding region_pool mapping if it exists else to default_pools.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub country_pools: Option<Map<SmolStr, Vec<SmolStr>>>,
	/// When the record was created.
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub created_on: Option<SmolStr>,
	/// A list of pool IDs ordered by their failover priority. Pools defined here are used by default, or when region_pools are not configured for a given region.
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub default_pools: Vec<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub depends_on: Option<Vec<SmolStr>>,
	/// Object description.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<SmolStr>,
	/// Whether to enable (the default) this load balancer.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub enabled: Option<bool>,
	/// The pool ID to use when all other pools are detected as unhealthy.
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub fallback_pool: SmolStr,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub for_each: Option<Vec<SmolStr>>,
	/// Identifier.
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<SmolStr>,
	/// Controls location-based steering for non-proxied requests. See `steering_policy` to learn how steering is affected.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub location_strategy: Option<CloudflareLoadBalancerLocationStrategy>,
	/// When the record was last modified.
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub modified_on: Option<SmolStr>,
	/// DNS record name (or @ for the zone apex) in Punycode.
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub name: SmolStr,
	/// List of networks where Load Balancer or Pool is enabled.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub networks: Option<Vec<SmolStr>>,
	/// Enterprise only: A mapping of Cloudflare PoP identifiers to a list of pool IDs (ordered by their failover priority) for the PoP (datacenter). Any PoPs not explicitly defined will fall back to using the corresponding country_pool, then region_pool mapping if it exists else to default_pools.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub pop_pools: Option<Map<SmolStr, Vec<SmolStr>>>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub provider: Option<SmolStr>,
	/// Whether the record is receiving the performance and security benefits of Cloudflare.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub proxied: Option<bool>,
	/// Configures pool weights.
	/// - `steering_policy="random"`: A random pool is selected with probability proportional to pool weights.
	/// - `steering_policy="least_outstanding_requests"`: Use pool weights to scale each pool's outstanding requests.
	/// - `steering_policy="least_connections"`: Use pool weights to scale each pool's open connections.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub random_steering: Option<CloudflareLoadBalancerRandomSteering>,
	/// A mapping of region codes to a list of pool IDs (ordered by their failover priority) for the given region. Any regions not explicitly defined will fall back to using default_pools.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub region_pools: Option<Map<SmolStr, Vec<SmolStr>>>,
	/// BETA Field Not General Access: A list of rules for this load balancer to execute.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub rules: Option<Vec<CloudflareLoadBalancerRules>>,
	/// Specifies the type of session affinity the load balancer should use unless specified as `"none"`. The supported types are: - `"cookie"`: On the first request to a proxied load balancer, a cookie is generated, encoding information of which origin the request will be forwarded to. Subsequent requests, by the same client to the same load balancer, will be sent to the origin server the cookie encodes, for the duration of the cookie and as long as the origin server remains healthy. If the cookie has expired or the origin server is unhealthy, then a new origin server is calculated and used. - `"ip_cookie"`: Behaves the same as `"cookie"` except the initial origin selection is stable and based on the client's ip address. - `"header"`: On the first request to a proxied load balancer, a session key based on the configured HTTP headers (see `session_affinity_attributes.headers`) is generated, encoding the request headers used for storing in the load balancer session state which origin the request will be forwarded to. Subsequent requests to the load balancer with the same headers will be sent to the same origin server, for the duration of the session and as long as the origin server remains healthy. If the session has been idle for the duration of `session_affinity_ttl` seconds or the origin server is unhealthy, then a new origin server is calculated and used. See `headers` in `session_affinity_attributes` for additional required configuration.
	/// Available values: "none", "cookie", "ip_cookie", "header".
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub session_affinity: Option<SmolStr>,
	/// Configures attributes for session affinity.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub session_affinity_attributes:
		Option<CloudflareLoadBalancerSessionAffinityAttributes>,
	/// Time, in seconds, until a client's session expires after being created. Once the expiry time has been reached, subsequent requests may get sent to a different origin server. The accepted ranges per `session_affinity` policy are: - `"cookie"` / `"ip_cookie"`: The current default of 23 hours will be used unless explicitly set. The accepted range of values is between [1800, 604800]. - `"header"`: The current default of 1800 seconds will be used unless explicitly set. The accepted range of values is between [30, 3600]. Note: With session affinity by header, sessions only expire after they haven't been used for the number of seconds specified.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub session_affinity_ttl: Option<i64>,
	/// Steering Policy for this load balancer.
	/// - `"off"`: Use `default_pools`.
	/// - `"geo"`: Use `region_pools`/`country_pools`/`pop_pools`. For non-proxied requests, the country for `country_pools` is determined by `location_strategy`.
	/// - `"random"`: Select a pool randomly.
	/// - `"dynamic_latency"`: Use round trip time to select the closest pool in default_pools (requires pool health checks).
	/// - `"proximity"`: Use the pools' latitude and longitude to select the closest pool using the Cloudflare PoP location for proxied requests or the location determined by `location_strategy` for non-proxied requests.
	/// - `"least_outstanding_requests"`: Select a pool by taking into consideration `random_steering` weights, as well as each pool's number of outstanding requests. Pools with more pending requests are weighted proportionately less relative to others.
	/// - `"least_connections"`: Select a pool by taking into consideration `random_steering` weights, as well as each pool's number of open connections. Pools with more open connections are weighted proportionately less relative to others. Supported for HTTP/1 and HTTP/2 connections.
	/// - `""`: Will map to `"geo"` if you use `region_pools`/`country_pools`/`pop_pools` otherwise `"off"`.
	/// Available values: "off", "geo", "random", "dynamic_latency", "proximity", "least_outstanding_requests", "least_connections", "".
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub steering_policy: Option<SmolStr>,
	/// Time To Live (TTL) of the DNS record in seconds. Setting to 1 means 'automatic'. Value must be between 60 and 86400, with the minimum reduced to 30 for Enterprise zones.
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ttl: Option<i64>,
	/// Identifier.
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub zone_id: SmolStr,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub zone_name: Option<SmolStr>,
}
impl terra::ToJson for CloudflareLoadBalancerDetails {
	fn to_json(&self) -> serde_json::Value {
		serde_json::to_value(self).expect("serialization should not fail")
	}
}
impl terra::Resource for CloudflareLoadBalancerDetails {
	fn resource_type(&self) -> &'static str { "cloudflare_load_balancer" }
	fn provider(&self) -> &'static terra::Provider {
		&terra::Provider::CLOUDFLARE
	}
	fn validate_definition(
		&self,
	) -> Result<(), terra::ResourceValidationError> {
		if self.created_on.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "created_on",
				},
			);
		}
		if self.default_pools.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "default_pools",
			});
		}
		if self.fallback_pool.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "fallback_pool",
			});
		}
		if self.id.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "id",
				},
			);
		}
		if self.modified_on.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "modified_on",
				},
			);
		}
		if self.name.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "name",
			});
		}
		if self.zone_id.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "zone_id",
			});
		}
		if self.zone_name.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "zone_name",
				},
			);
		}
		Ok(())
	}
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct CloudflareLoadBalancerLocationStrategy {
	/// Determines the authoritative location when ECS is not preferred, does not exist in the request, or its GeoIP lookup is unsuccessful.
	/// - `"pop"`: Use the Cloudflare PoP location.
	/// - `"resolver_ip"`: Use the DNS resolver GeoIP location. If the GeoIP lookup is unsuccessful, use the Cloudflare PoP location.
	/// Available values: "pop", "resolver_ip".
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub mode: Option<SmolStr>,
	/// Whether the EDNS Client Subnet (ECS) GeoIP should be preferred as the authoritative location.
	/// - `"always"`: Always prefer ECS.
	/// - `"never"`: Never prefer ECS.
	/// - `"proximity"`: Prefer ECS only when `steering_policy="proximity"`.
	/// - `"geo"`: Prefer ECS only when `steering_policy="geo"`.
	/// Available values: "always", "never", "proximity", "geo".
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub prefer_ecs: Option<SmolStr>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct CloudflareLoadBalancerMonitorDetails {
	/// Identifier.
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub account_id: SmolStr,
	/// Do not validate the certificate when monitor use HTTPS. This parameter is currently only valid for HTTP and HTTPS monitors.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub allow_insecure: Option<bool>,
	/// To be marked unhealthy the monitored origin must fail this healthcheck N consecutive times.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub consecutive_down: Option<i64>,
	/// To be marked healthy the monitored origin must pass this healthcheck N consecutive times.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub consecutive_up: Option<i64>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub count: Option<i64>,
	/// When the record was created.
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub created_on: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub depends_on: Option<Vec<SmolStr>>,
	/// Object description.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<SmolStr>,
	/// A case-insensitive sub-string to look for in the response body. If this string is not found, the origin will be marked as unhealthy. This parameter is only valid for HTTP and HTTPS monitors.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub expected_body: Option<SmolStr>,
	/// The expected HTTP response code or code range of the health check. This parameter is only valid for HTTP and HTTPS monitors.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub expected_codes: Option<SmolStr>,
	/// Follow redirects if returned by the origin. This parameter is only valid for HTTP and HTTPS monitors.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub follow_redirects: Option<bool>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub for_each: Option<Vec<SmolStr>>,
	/// The HTTP request headers to send in the health check. It is recommended you set a Host header by default. The User-Agent header cannot be overridden. This parameter is only valid for HTTP and HTTPS monitors.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub header: Option<Map<SmolStr, Vec<SmolStr>>>,
	/// Identifier.
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<SmolStr>,
	/// The interval between each health check. Shorter intervals may improve failover time, but will increase load on the origins as we check from multiple locations.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub interval: Option<i64>,
	/// The method to use for the health check. This defaults to 'GET' for HTTP/HTTPS based checks and 'connection_established' for TCP based health checks.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub method: Option<SmolStr>,
	/// When the record was last modified.
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub modified_on: Option<SmolStr>,
	/// The endpoint path you want to conduct a health check against. This parameter is only valid for HTTP and HTTPS monitors.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub path: Option<SmolStr>,
	/// The port of the service.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub port: Option<i64>,
	/// Assign this monitor to emulate the specified zone while probing. This parameter is only valid for HTTP and HTTPS monitors.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub probe_zone: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub provider: Option<SmolStr>,
	/// The number of retries to attempt in case of a timeout before marking the origin as unhealthy. Retries are attempted immediately.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub retries: Option<i64>,
	/// The timeout (in seconds) before marking the health check as failed.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub timeout: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub r#type: Option<SmolStr>,
}
impl terra::ToJson for CloudflareLoadBalancerMonitorDetails {
	fn to_json(&self) -> serde_json::Value {
		serde_json::to_value(self).expect("serialization should not fail")
	}
}
impl terra::Resource for CloudflareLoadBalancerMonitorDetails {
	fn resource_type(&self) -> &'static str {
		"cloudflare_load_balancer_monitor"
	}
	fn provider(&self) -> &'static terra::Provider {
		&terra::Provider::CLOUDFLARE
	}
	fn validate_definition(
		&self,
	) -> Result<(), terra::ResourceValidationError> {
		if self.account_id.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "account_id",
			});
		}
		if self.created_on.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "created_on",
				},
			);
		}
		if self.id.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "id",
				},
			);
		}
		if self.modified_on.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "modified_on",
				},
			);
		}
		Ok(())
	}
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct CloudflareLoadBalancerPoolDetails {
	/// Identifier.
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub account_id: SmolStr,
	/// A list of regions from which to run health checks. Null means every Cloudflare data center.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub check_regions: Option<Vec<SmolStr>>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub count: Option<i64>,
	/// When the record was created.
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub created_on: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub depends_on: Option<Vec<SmolStr>>,
	/// Object description.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<SmolStr>,
	/// This field shows up only if the pool is disabled. This field is set with the time the pool was disabled at.
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub disabled_at: Option<SmolStr>,
	/// Whether to enable (the default) this load balancer.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub enabled: Option<bool>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub for_each: Option<Vec<SmolStr>>,
	/// Identifier.
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<SmolStr>,
	/// The latitude of the data center containing the origins used in this pool in decimal degrees. If this is set, longitude must also be set.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub latitude: Option<i64>,
	/// Configures load shedding policies and percentages for the pool.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub load_shedding: Option<CloudflareLoadBalancerPoolLoadShedding>,
	/// The longitude of the data center containing the origins used in this pool in decimal degrees. If this is set, latitude must also be set.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub longitude: Option<i64>,
	/// The minimum number of origins that must be healthy for this pool to serve traffic. If the number of healthy origins falls below this number, the pool will be marked unhealthy and will failover to the next available pool.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub minimum_origins: Option<i64>,
	/// When the record was last modified.
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub modified_on: Option<SmolStr>,
	/// The ID of the Monitor to use for checking the health of origins within this pool.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub monitor: Option<SmolStr>,
	/// The ID of the Monitor Group to use for checking the health of origins within this pool.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub monitor_group: Option<SmolStr>,
	/// DNS record name (or @ for the zone apex) in Punycode.
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub name: SmolStr,
	/// List of networks where Load Balancer or Pool is enabled.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub networks: Option<Vec<SmolStr>>,
	/// This field is now deprecated. It has been moved to Cloudflare's Centralized Notification service https://developers.cloudflare.com/fundamentals/notifications/. The email address to send health status notifications to. This can be an individual mailbox or a mailing list. Multiple emails can be supplied as a comma delimited list.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub notification_email: Option<SmolStr>,
	/// Filter pool and origin health notifications by resource type or health status. Use null to reset.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub notification_filter:
		Option<CloudflareLoadBalancerPoolNotificationFilter>,
	/// Configures origin steering for the pool. Controls how origins are selected for new sessions and traffic without session affinity.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub origin_steering: Option<CloudflareLoadBalancerPoolOriginSteering>,
	/// The list of origins within this pool. Traffic directed at this pool is balanced across all currently healthy origins, provided the pool itself is healthy.
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub origins: Vec<CloudflareLoadBalancerPoolOrigins>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub provider: Option<SmolStr>,
}
impl terra::ToJson for CloudflareLoadBalancerPoolDetails {
	fn to_json(&self) -> serde_json::Value {
		serde_json::to_value(self).expect("serialization should not fail")
	}
}
impl terra::Resource for CloudflareLoadBalancerPoolDetails {
	fn resource_type(&self) -> &'static str { "cloudflare_load_balancer_pool" }
	fn provider(&self) -> &'static terra::Provider {
		&terra::Provider::CLOUDFLARE
	}
	fn validate_definition(
		&self,
	) -> Result<(), terra::ResourceValidationError> {
		if self.account_id.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "account_id",
			});
		}
		if self.created_on.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "created_on",
				},
			);
		}
		if self.disabled_at.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "disabled_at",
				},
			);
		}
		if self.id.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "id",
				},
			);
		}
		if self.modified_on.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "modified_on",
				},
			);
		}
		if self.name.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "name",
			});
		}
		if self.networks.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "networks",
				},
			);
		}
		if self.origins.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "origins",
			});
		}
		Ok(())
	}
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct CloudflareLoadBalancerPoolLoadShedding {
	/// The percent of traffic to shed from the pool, according to the default policy. Applies to new sessions and traffic without session affinity.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub default_percent: Option<i64>,
	/// The default policy to use when load shedding. A random policy randomly sheds a given percent of requests. A hash policy computes a hash over the CF-Connecting-IP address and sheds all requests originating from a percent of IPs.
	/// Available values: "random", "hash".
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub default_policy: Option<SmolStr>,
	/// The percent of existing sessions to shed from the pool, according to the session policy.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub session_percent: Option<i64>,
	/// Only the hash policy is supported for existing sessions (to avoid exponential decay).
	/// Available values: "hash".
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub session_policy: Option<SmolStr>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct CloudflareLoadBalancerPoolNotificationFilter {
	/// Filter options for a particular resource type (pool or origin). Use null to reset.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub origin: Option<CloudflareLoadBalancerPoolNotificationFilterOrigin>,
	/// Filter options for a particular resource type (pool or origin). Use null to reset.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub pool: Option<CloudflareLoadBalancerPoolNotificationFilterPool>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct CloudflareLoadBalancerPoolNotificationFilterOrigin {
	/// If set true, disable notifications for this type of resource (pool or origin).
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub disable: Option<bool>,
	/// If present, send notifications only for this health status (e.g. false for only DOWN events). Use null to reset (all events).
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub healthy: Option<bool>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct CloudflareLoadBalancerPoolNotificationFilterPool {
	/// If set true, disable notifications for this type of resource (pool or origin).
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub disable: Option<bool>,
	/// If present, send notifications only for this health status (e.g. false for only DOWN events). Use null to reset (all events).
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub healthy: Option<bool>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct CloudflareLoadBalancerPoolOriginSteering {
	/// The type of origin steering policy to use.
	/// - `"random"`: Select an origin randomly.
	/// - `"hash"`: Select an origin by computing a hash over the CF-Connecting-IP address.
	/// - `"least_outstanding_requests"`: Select an origin by taking into consideration origin weights, as well as each origin's number of outstanding requests. Origins with more pending requests are weighted proportionately less relative to others.
	/// - `"least_connections"`: Select an origin by taking into consideration origin weights, as well as each origin's number of open connections. Origins with more open connections are weighted proportionately less relative to others. Supported for HTTP/1 and HTTP/2 connections.
	/// Available values: "random", "hash", "least_outstanding_requests", "least_connections".
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub policy: Option<SmolStr>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct CloudflareLoadBalancerPoolOrigins {
	/// The IP address (IPv4 or IPv6) of the origin, or its publicly addressable hostname. Hostnames entered here should resolve directly to the origin, and not be a hostname proxied by Cloudflare. To set an internal/reserved address, virtual_network_id must also be set.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub address: Option<SmolStr>,
	/// This field shows up only if the pool is disabled. This field is set with the time the pool was disabled at.
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub disabled_at: Option<SmolStr>,
	/// Whether to enable (the default) this load balancer.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub enabled: Option<bool>,
	/// If enabled, causes the CNAME record to be resolved externally and the resulting address records (e.g., A and AAAA) to be returned instead of the CNAME record itself. This setting is unavailable for proxied records, since they are always flattened.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub flatten_cname: Option<bool>,
	/// The HTTP request headers to send in the health check. It is recommended you set a Host header by default. The User-Agent header cannot be overridden. This parameter is only valid for HTTP and HTTPS monitors.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub header: Option<CloudflareLoadBalancerPoolOriginsHeader>,
	/// DNS record name (or @ for the zone apex) in Punycode.
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub name: Option<SmolStr>,
	/// The port of the service.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub port: Option<i64>,
	/// The virtual network subnet ID the origin belongs in. Virtual network must also belong to the account.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub virtual_network_id: Option<SmolStr>,
	/// The record weight.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub weight: Option<i64>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct CloudflareLoadBalancerPoolOriginsHeader {
	/// The 'Host' header allows to override the hostname set in the HTTP request. Current support is 1 'Host' header override per origin.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub host: Option<Vec<SmolStr>>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct CloudflareLoadBalancerRandomSteering {
	/// The default weight for pools in the load balancer that are not specified in the pool_weights map.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub default_weight: Option<i64>,
	/// A mapping of pool IDs to custom weights. The weight is relative to other pools in the load balancer.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub pool_weights: Option<Map<SmolStr, i64>>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct CloudflareLoadBalancerRules {
	/// The condition expressions to evaluate. If the condition evaluates to true, the overrides or fixed_response in this rule will be applied. An empty condition is always true. For more details on condition expressions, please see https://developers.cloudflare.com/load-balancing/understand-basics/load-balancing-rules/expressions.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub condition: Option<SmolStr>,
	/// Disable this specific rule. It will no longer be evaluated by this load balancer.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub disabled: Option<bool>,
	/// A collection of fields used to directly respond to the eyeball instead of routing to a pool. If a fixed_response is supplied the rule will be marked as terminates.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub fixed_response: Option<CloudflareLoadBalancerRulesFixedResponse>,
	/// DNS record name (or @ for the zone apex) in Punycode.
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub name: Option<SmolStr>,
	/// A collection of overrides to apply to the load balancer when this rule's condition is true. All fields are optional.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub overrides: Option<CloudflareLoadBalancerRulesOverrides>,
	/// Required for MX, SRV and URI records; unused by other record types. Records with lower priorities are preferred.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub priority: Option<i64>,
	/// If this rule's condition is true, this causes rule evaluation to stop after processing this rule.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub terminates: Option<bool>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct CloudflareLoadBalancerRulesFixedResponse {
	/// The http 'Content-Type' header to include in the response.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub content_type: Option<SmolStr>,
	/// The http 'Location' header to include in the response.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub location: Option<SmolStr>,
	/// Text to include as the http body.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub message_body: Option<SmolStr>,
	/// The http status code to respond with.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub status_code: Option<i64>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct CloudflareLoadBalancerRulesOverrides {
	/// Controls features that modify the routing of requests to pools and origins in response to dynamic conditions, such as during the interval between active health monitoring requests. For example, zero-downtime failover occurs immediately when an origin becomes unavailable due to HTTP 521, 522, or 523 response codes. If there is another healthy origin in the same pool, the request is retried once against this alternate origin.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub adaptive_routing:
		Option<CloudflareLoadBalancerRulesOverridesAdaptiveRouting>,
	/// A mapping of country codes to a list of pool IDs (ordered by their failover priority) for the given country. Any country not explicitly defined will fall back to using the corresponding region_pool mapping if it exists else to default_pools.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub country_pools: Option<Map<SmolStr, Vec<SmolStr>>>,
	/// A list of pool IDs ordered by their failover priority. Pools defined here are used by default, or when region_pools are not configured for a given region.
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub default_pools: Option<Vec<SmolStr>>,
	/// The pool ID to use when all other pools are detected as unhealthy.
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub fallback_pool: Option<SmolStr>,
	/// Controls location-based steering for non-proxied requests. See `steering_policy` to learn how steering is affected.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub location_strategy:
		Option<CloudflareLoadBalancerRulesOverridesLocationStrategy>,
	/// Enterprise only: A mapping of Cloudflare PoP identifiers to a list of pool IDs (ordered by their failover priority) for the PoP (datacenter). Any PoPs not explicitly defined will fall back to using the corresponding country_pool, then region_pool mapping if it exists else to default_pools.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub pop_pools: Option<Map<SmolStr, Vec<SmolStr>>>,
	/// Configures pool weights.
	/// - `steering_policy="random"`: A random pool is selected with probability proportional to pool weights.
	/// - `steering_policy="least_outstanding_requests"`: Use pool weights to scale each pool's outstanding requests.
	/// - `steering_policy="least_connections"`: Use pool weights to scale each pool's open connections.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub random_steering:
		Option<CloudflareLoadBalancerRulesOverridesRandomSteering>,
	/// A mapping of region codes to a list of pool IDs (ordered by their failover priority) for the given region. Any regions not explicitly defined will fall back to using default_pools.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub region_pools: Option<Map<SmolStr, Vec<SmolStr>>>,
	/// Specifies the type of session affinity the load balancer should use unless specified as `"none"`. The supported types are: - `"cookie"`: On the first request to a proxied load balancer, a cookie is generated, encoding information of which origin the request will be forwarded to. Subsequent requests, by the same client to the same load balancer, will be sent to the origin server the cookie encodes, for the duration of the cookie and as long as the origin server remains healthy. If the cookie has expired or the origin server is unhealthy, then a new origin server is calculated and used. - `"ip_cookie"`: Behaves the same as `"cookie"` except the initial origin selection is stable and based on the client's ip address. - `"header"`: On the first request to a proxied load balancer, a session key based on the configured HTTP headers (see `session_affinity_attributes.headers`) is generated, encoding the request headers used for storing in the load balancer session state which origin the request will be forwarded to. Subsequent requests to the load balancer with the same headers will be sent to the same origin server, for the duration of the session and as long as the origin server remains healthy. If the session has been idle for the duration of `session_affinity_ttl` seconds or the origin server is unhealthy, then a new origin server is calculated and used. See `headers` in `session_affinity_attributes` for additional required configuration.
	/// Available values: "none", "cookie", "ip_cookie", "header".
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub session_affinity: Option<SmolStr>,
	/// Configures attributes for session affinity.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub session_affinity_attributes:
		Option<CloudflareLoadBalancerRulesOverridesSessionAffinityAttributes>,
	/// Time, in seconds, until a client's session expires after being created. Once the expiry time has been reached, subsequent requests may get sent to a different origin server. The accepted ranges per `session_affinity` policy are: - `"cookie"` / `"ip_cookie"`: The current default of 23 hours will be used unless explicitly set. The accepted range of values is between [1800, 604800]. - `"header"`: The current default of 1800 seconds will be used unless explicitly set. The accepted range of values is between [30, 3600]. Note: With session affinity by header, sessions only expire after they haven't been used for the number of seconds specified.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub session_affinity_ttl: Option<i64>,
	/// Steering Policy for this load balancer.
	/// - `"off"`: Use `default_pools`.
	/// - `"geo"`: Use `region_pools`/`country_pools`/`pop_pools`. For non-proxied requests, the country for `country_pools` is determined by `location_strategy`.
	/// - `"random"`: Select a pool randomly.
	/// - `"dynamic_latency"`: Use round trip time to select the closest pool in default_pools (requires pool health checks).
	/// - `"proximity"`: Use the pools' latitude and longitude to select the closest pool using the Cloudflare PoP location for proxied requests or the location determined by `location_strategy` for non-proxied requests.
	/// - `"least_outstanding_requests"`: Select a pool by taking into consideration `random_steering` weights, as well as each pool's number of outstanding requests. Pools with more pending requests are weighted proportionately less relative to others.
	/// - `"least_connections"`: Select a pool by taking into consideration `random_steering` weights, as well as each pool's number of open connections. Pools with more open connections are weighted proportionately less relative to others. Supported for HTTP/1 and HTTP/2 connections.
	/// - `""`: Will map to `"geo"` if you use `region_pools`/`country_pools`/`pop_pools` otherwise `"off"`.
	/// Available values: "off", "geo", "random", "dynamic_latency", "proximity", "least_outstanding_requests", "least_connections", "".
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub steering_policy: Option<SmolStr>,
	/// Time To Live (TTL) of the DNS record in seconds. Setting to 1 means 'automatic'. Value must be between 60 and 86400, with the minimum reduced to 30 for Enterprise zones.
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ttl: Option<i64>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct CloudflareLoadBalancerRulesOverridesAdaptiveRouting {
	/// Extends zero-downtime failover of requests to healthy origins from alternate pools, when no healthy alternate exists in the same pool, according to the failover order defined by traffic and origin steering. When set false (the default) zero-downtime failover will only occur between origins within the same pool. See `session_affinity_attributes` for control over when sessions are broken or reassigned.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub failover_across_pools: Option<bool>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct CloudflareLoadBalancerRulesOverridesLocationStrategy {
	/// Determines the authoritative location when ECS is not preferred, does not exist in the request, or its GeoIP lookup is unsuccessful.
	/// - `"pop"`: Use the Cloudflare PoP location.
	/// - `"resolver_ip"`: Use the DNS resolver GeoIP location. If the GeoIP lookup is unsuccessful, use the Cloudflare PoP location.
	/// Available values: "pop", "resolver_ip".
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub mode: Option<SmolStr>,
	/// Whether the EDNS Client Subnet (ECS) GeoIP should be preferred as the authoritative location.
	/// - `"always"`: Always prefer ECS.
	/// - `"never"`: Never prefer ECS.
	/// - `"proximity"`: Prefer ECS only when `steering_policy="proximity"`.
	/// - `"geo"`: Prefer ECS only when `steering_policy="geo"`.
	/// Available values: "always", "never", "proximity", "geo".
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub prefer_ecs: Option<SmolStr>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct CloudflareLoadBalancerRulesOverridesRandomSteering {
	/// The default weight for pools in the load balancer that are not specified in the pool_weights map.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub default_weight: Option<i64>,
	/// A mapping of pool IDs to custom weights. The weight is relative to other pools in the load balancer.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub pool_weights: Option<Map<SmolStr, i64>>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct CloudflareLoadBalancerRulesOverridesSessionAffinityAttributes {
	/// Configures the drain duration in seconds. This field is only used when session affinity is enabled on the load balancer.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub drain_duration: Option<i64>,
	/// Configures the names of HTTP headers to base session affinity on when header `session_affinity` is enabled. At least one HTTP header name must be provided. To specify the exact cookies to be used, include an item in the following format: `"cookie:<cookie-name-1>,<cookie-name-2>"` (example) where everything after the colon is a comma-separated list of cookie names. Providing only `"cookie"` will result in all cookies being used. The default max number of HTTP header names that can be provided depends on your plan: 5 for Enterprise, 1 for all other plans.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub headers: Option<Vec<SmolStr>>,
	/// When header `session_affinity` is enabled, this option can be used to specify how HTTP headers on load balancing requests will be used. The supported values are: - `"true"`: Load balancing requests must contain *all* of the HTTP headers specified by the `headers` session affinity attribute, otherwise sessions aren't created. - `"false"`: Load balancing requests must contain *at least one* of the HTTP headers specified by the `headers` session affinity attribute, otherwise sessions aren't created.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub require_all_headers: Option<bool>,
	/// Configures the SameSite attribute on session affinity cookie. Value "Auto" will be translated to "Lax" or "None" depending if Always Use HTTPS is enabled. Note: when using value "None", the secure attribute can not be set to "Never".
	/// Available values: "Auto", "Lax", "None", "Strict".
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub samesite: Option<SmolStr>,
	/// Configures the Secure attribute on session affinity cookie. Value "Always" indicates the Secure attribute will be set in the Set-Cookie header, "Never" indicates the Secure attribute will not be set, and "Auto" will set the Secure attribute depending if Always Use HTTPS is enabled.
	/// Available values: "Auto", "Always", "Never".
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub secure: Option<SmolStr>,
	/// Configures the zero-downtime failover between origins within a pool when session affinity is enabled. This feature is currently incompatible with Argo, Tiered Cache, and Bandwidth Alliance. The supported values are: - `"none"`: No failover takes place for sessions pinned to the origin (default). - `"temporary"`: Traffic will be sent to another other healthy origin until the originally pinned origin is available; note that this can potentially result in heavy origin flapping. - `"sticky"`: The session affinity cookie is updated and subsequent requests are sent to the new origin. Note: Zero-downtime failover with sticky sessions is currently not supported for session affinity by header.
	/// Available values: "none", "temporary", "sticky".
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub zero_downtime_failover: Option<SmolStr>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct CloudflareLoadBalancerSessionAffinityAttributes {
	/// Configures the drain duration in seconds. This field is only used when session affinity is enabled on the load balancer.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub drain_duration: Option<i64>,
	/// Configures the names of HTTP headers to base session affinity on when header `session_affinity` is enabled. At least one HTTP header name must be provided. To specify the exact cookies to be used, include an item in the following format: `"cookie:<cookie-name-1>,<cookie-name-2>"` (example) where everything after the colon is a comma-separated list of cookie names. Providing only `"cookie"` will result in all cookies being used. The default max number of HTTP header names that can be provided depends on your plan: 5 for Enterprise, 1 for all other plans.
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub headers: Option<Vec<SmolStr>>,
	/// When header `session_affinity` is enabled, this option can be used to specify how HTTP headers on load balancing requests will be used. The supported values are: - `"true"`: Load balancing requests must contain *all* of the HTTP headers specified by the `headers` session affinity attribute, otherwise sessions aren't created. - `"false"`: Load balancing requests must contain *at least one* of the HTTP headers specified by the `headers` session affinity attribute, otherwise sessions aren't created.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub require_all_headers: Option<bool>,
	/// Configures the SameSite attribute on session affinity cookie. Value "Auto" will be translated to "Lax" or "None" depending if Always Use HTTPS is enabled. Note: when using value "None", the secure attribute can not be set to "Never".
	/// Available values: "Auto", "Lax", "None", "Strict".
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub samesite: Option<SmolStr>,
	/// Configures the Secure attribute on session affinity cookie. Value "Always" indicates the Secure attribute will be set in the Set-Cookie header, "Never" indicates the Secure attribute will not be set, and "Auto" will set the Secure attribute depending if Always Use HTTPS is enabled.
	/// Available values: "Auto", "Always", "Never".
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub secure: Option<SmolStr>,
	/// Configures the zero-downtime failover between origins within a pool when session affinity is enabled. This feature is currently incompatible with Argo, Tiered Cache, and Bandwidth Alliance. The supported values are: - `"none"`: No failover takes place for sessions pinned to the origin (default). - `"temporary"`: Traffic will be sent to another other healthy origin until the originally pinned origin is available; note that this can potentially result in heavy origin flapping. - `"sticky"`: The session affinity cookie is updated and subsequent requests are sent to the new origin. Note: Zero-downtime failover with sticky sessions is currently not supported for session affinity by header.
	/// Available values: "none", "temporary", "sticky".
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub zero_downtime_failover: Option<SmolStr>,
}
