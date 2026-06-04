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
pub struct AwsRoute53RecordDetails {
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub allow_overwrite: Option<bool>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub count: Option<i64>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub depends_on: Option<Vec<SmolStr>>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub for_each: Option<Vec<SmolStr>>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub fqdn: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub health_check_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub multivalue_answer_routing_policy: Option<bool>,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub name: SmolStr,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub provider: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub records: Option<Vec<SmolStr>>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub set_identifier: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ttl: Option<i64>,
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub r#type: SmolStr,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub zone_id: SmolStr,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub alias: Option<Vec<AwsRoute53RecordResourceBlockTypeAlias>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub cidr_routing_policy:
		Option<Vec<AwsRoute53RecordResourceBlockTypeCidrRoutingPolicy>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub failover_routing_policy:
		Option<Vec<AwsRoute53RecordResourceBlockTypeFailoverRoutingPolicy>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub geolocation_routing_policy:
		Option<Vec<AwsRoute53RecordResourceBlockTypeGeolocationRoutingPolicy>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub geoproximity_routing_policy:
		Option<Vec<AwsRoute53RecordResourceBlockTypeGeoproximityRoutingPolicy>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub latency_routing_policy:
		Option<Vec<AwsRoute53RecordResourceBlockTypeLatencyRoutingPolicy>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub timeouts: Option<Vec<AwsRoute53RecordResourceBlockTypeTimeouts>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub weighted_routing_policy:
		Option<Vec<AwsRoute53RecordResourceBlockTypeWeightedRoutingPolicy>>,
}
impl terra::ToJson for AwsRoute53RecordDetails {
	fn to_json(&self) -> serde_json::Value {
		serde_json::to_value(self).expect("serialization should not fail")
	}
}
impl terra::Resource for AwsRoute53RecordDetails {
	fn resource_type(&self) -> &'static str { "aws_route53_record" }
	fn provider(&self) -> &'static terra::Provider { &terra::Provider::AWS }
	fn validate_definition(
		&self,
	) -> Result<(), terra::ResourceValidationError> {
		if self.fqdn.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "fqdn",
				},
			);
		}
		if self.name.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "name",
			});
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
#[serde(rename = "alias")]
pub struct AwsRoute53RecordResourceBlockTypeAlias {
	/// ## Attribute
	/// `required`
	pub evaluate_target_health: bool,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub name: SmolStr,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub zone_id: SmolStr,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "cidr_routing_policy")]
pub struct AwsRoute53RecordResourceBlockTypeCidrRoutingPolicy {
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub collection_id: SmolStr,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub location_name: SmolStr,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "failover_routing_policy")]
pub struct AwsRoute53RecordResourceBlockTypeFailoverRoutingPolicy {
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub r#type: SmolStr,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "geolocation_routing_policy")]
pub struct AwsRoute53RecordResourceBlockTypeGeolocationRoutingPolicy {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub continent: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub country: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub subdivision: Option<SmolStr>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "geoproximity_routing_policy")]
pub struct AwsRoute53RecordResourceBlockTypeGeoproximityRoutingPolicy {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub aws_region: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub bias: Option<i64>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub local_zone_group: Option<SmolStr>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub coordinates:
		Option<Vec<GeoproximityRoutingPolicyResourceBlockTypeCoordinates>>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "latency_routing_policy")]
pub struct AwsRoute53RecordResourceBlockTypeLatencyRoutingPolicy {
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub region: SmolStr,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "timeouts")]
pub struct AwsRoute53RecordResourceBlockTypeTimeouts {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub create: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub delete: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub update: Option<SmolStr>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "weighted_routing_policy")]
pub struct AwsRoute53RecordResourceBlockTypeWeightedRoutingPolicy {
	/// ## Attribute
	/// `required`
	pub weight: i64,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "coordinates")]
pub struct GeoproximityRoutingPolicyResourceBlockTypeCoordinates {
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub latitude: SmolStr,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub longitude: SmolStr,
}
