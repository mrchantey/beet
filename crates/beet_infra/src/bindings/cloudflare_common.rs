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
	pub data: Option<Map<SmolStr, SmolStr>>,
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
	pub settings: Option<Map<SmolStr, SmolStr>>,
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
pub struct CloudflareLoadBalancerDetails {
	/// Controls features that modify the routing of requests to pools and origins in response to dynamic conditions, such as during the interval between active health monitoring requests. For example, zero-downtime failover occurs immediately when an origin becomes unavailable due to HTTP 521, 522, or 523 response codes. If there is another healthy origin in the same pool, the request is retried once against this alternate origin.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub adaptive_routing: Option<Map<SmolStr, SmolStr>>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub count: Option<i64>,
	/// A mapping of country codes to a list of pool IDs (ordered by their failover priority) for the given country. Any country not explicitly defined will fall back to using the corresponding region_pool mapping if it exists else to default_pools.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub country_pools: Option<Map<SmolStr, SmolStr>>,
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
	pub location_strategy: Option<Map<SmolStr, SmolStr>>,
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
	pub pop_pools: Option<Map<SmolStr, SmolStr>>,
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
	pub random_steering: Option<Map<SmolStr, SmolStr>>,
	/// A mapping of region codes to a list of pool IDs (ordered by their failover priority) for the given region. Any regions not explicitly defined will fall back to using default_pools.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub region_pools: Option<Map<SmolStr, SmolStr>>,
	/// BETA Field Not General Access: A list of rules for this load balancer to execute.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub rules: Option<Map<SmolStr, SmolStr>>,
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
	pub session_affinity_attributes: Option<Map<SmolStr, SmolStr>>,
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
	pub header: Option<Map<SmolStr, SmolStr>>,
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
	/// The port number to connect to for the health check. Required for TCP, UDP, and SMTP checks. HTTP and HTTPS checks should only define the port when using a non-standard port (HTTP: default 80, HTTPS: default 443).
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
	pub load_shedding: Option<Map<SmolStr, SmolStr>>,
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
	pub notification_filter: Option<Map<SmolStr, SmolStr>>,
	/// Configures origin steering for the pool. Controls how origins are selected for new sessions and traffic without session affinity.
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub origin_steering: Option<Map<SmolStr, SmolStr>>,
	/// The list of origins within this pool. Traffic directed at this pool is balanced across all currently healthy origins, provided the pool itself is healthy.
	/// ## Attribute
	/// `required`
	pub origins: Map<SmolStr, SmolStr>,
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
