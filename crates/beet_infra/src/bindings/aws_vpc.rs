//! Auto-generated Terraform provider bindings — do not edit!
//! Auto-generated Terraform provider bindings — do not edit!
//! Auto-generated Terraform provider bindings — do not edit!
//! Generated from the hashicorp/aws v6.62.0 schema.

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
pub struct AwsInternetGatewayDetails {
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub arn: Option<SmolStr>,
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
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub owner_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub provider: Option<SmolStr>,
	/// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub region: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tags: Option<Map<SmolStr, SmolStr>>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tags_all: Option<Map<SmolStr, SmolStr>>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub vpc_id: Option<SmolStr>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub timeouts: Option<Vec<AwsInternetGatewayResourceBlockTypeTimeouts>>,
}
impl terra::ToJson for AwsInternetGatewayDetails {
	fn to_json(&self) -> serde_json::Value {
		serde_json::to_value(self).expect("serialization should not fail")
	}
}
impl terra::Resource for AwsInternetGatewayDetails {
	fn resource_type(&self) -> &'static str { "aws_internet_gateway" }
	fn provider(&self) -> &'static terra::Provider { &terra::Provider::AWS }
	fn validate_definition(
		&self,
	) -> Result<(), terra::ResourceValidationError> {
		if self.arn.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "arn",
				},
			);
		}
		if self.owner_id.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "owner_id",
				},
			);
		}
		Ok(())
	}
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct AwsRouteDetails {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub carrier_gateway_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub core_network_arn: Option<SmolStr>,
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
	pub destination_cidr_block: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub destination_ipv6_cidr_block: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub destination_prefix_list_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub egress_only_gateway_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub for_each: Option<Vec<SmolStr>>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub gateway_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub instance_id: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub instance_owner_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub local_gateway_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub nat_gateway_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub network_interface_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub odb_network_arn: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub origin: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub provider: Option<SmolStr>,
	/// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub region: Option<SmolStr>,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub route_table_id: SmolStr,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub state: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub transit_gateway_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub vpc_endpoint_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub vpc_peering_connection_id: Option<SmolStr>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub timeouts: Option<Vec<AwsRouteResourceBlockTypeTimeouts>>,
}
impl terra::ToJson for AwsRouteDetails {
	fn to_json(&self) -> serde_json::Value {
		serde_json::to_value(self).expect("serialization should not fail")
	}
}
impl terra::Resource for AwsRouteDetails {
	fn resource_type(&self) -> &'static str { "aws_route" }
	fn provider(&self) -> &'static terra::Provider { &terra::Provider::AWS }
	fn validate_definition(
		&self,
	) -> Result<(), terra::ResourceValidationError> {
		if self.instance_id.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "instance_id",
				},
			);
		}
		if self.instance_owner_id.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "instance_owner_id",
				},
			);
		}
		if self.origin.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "origin",
				},
			);
		}
		if self.route_table_id.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "route_table_id",
			});
		}
		if self.state.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "state",
				},
			);
		}
		Ok(())
	}
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct AwsRouteTableAssociationDetails {
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
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub gateway_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub provider: Option<SmolStr>,
	/// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub region: Option<SmolStr>,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub route_table_id: SmolStr,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub subnet_id: Option<SmolStr>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub timeouts:
		Option<Vec<AwsRouteTableAssociationResourceBlockTypeTimeouts>>,
}
impl terra::ToJson for AwsRouteTableAssociationDetails {
	fn to_json(&self) -> serde_json::Value {
		serde_json::to_value(self).expect("serialization should not fail")
	}
}
impl terra::Resource for AwsRouteTableAssociationDetails {
	fn resource_type(&self) -> &'static str { "aws_route_table_association" }
	fn provider(&self) -> &'static terra::Provider { &terra::Provider::AWS }
	fn validate_definition(
		&self,
	) -> Result<(), terra::ResourceValidationError> {
		if self.route_table_id.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "route_table_id",
			});
		}
		Ok(())
	}
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct AwsRouteTableDetails {
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub arn: Option<SmolStr>,
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
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub owner_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub propagating_vgws: Option<Vec<SmolStr>>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub provider: Option<SmolStr>,
	/// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub region: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub route: Option<Vec<AwsRouteTableRoute>>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tags: Option<Map<SmolStr, SmolStr>>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tags_all: Option<Map<SmolStr, SmolStr>>,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub vpc_id: SmolStr,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub timeouts: Option<Vec<AwsRouteTableResourceBlockTypeTimeouts>>,
}
impl terra::ToJson for AwsRouteTableDetails {
	fn to_json(&self) -> serde_json::Value {
		serde_json::to_value(self).expect("serialization should not fail")
	}
}
impl terra::Resource for AwsRouteTableDetails {
	fn resource_type(&self) -> &'static str { "aws_route_table" }
	fn provider(&self) -> &'static terra::Provider { &terra::Provider::AWS }
	fn validate_definition(
		&self,
	) -> Result<(), terra::ResourceValidationError> {
		if self.arn.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "arn",
				},
			);
		}
		if self.owner_id.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "owner_id",
				},
			);
		}
		if self.vpc_id.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "vpc_id",
			});
		}
		Ok(())
	}
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct AwsRouteTableRoute {
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub carrier_gateway_id: SmolStr,
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub cidr_block: SmolStr,
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub core_network_arn: SmolStr,
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub destination_prefix_list_id: SmolStr,
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub egress_only_gateway_id: SmolStr,
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub gateway_id: SmolStr,
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub ipv6_cidr_block: SmolStr,
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub local_gateway_id: SmolStr,
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub nat_gateway_id: SmolStr,
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub network_interface_id: SmolStr,
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub odb_network_arn: SmolStr,
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub transit_gateway_id: SmolStr,
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub vpc_endpoint_id: SmolStr,
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub vpc_peering_connection_id: SmolStr,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct AwsSecurityGroupDetails {
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub arn: Option<SmolStr>,
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
	pub description: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub egress: Option<Vec<AwsSecurityGroupEgress>>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub for_each: Option<Vec<SmolStr>>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ingress: Option<Vec<AwsSecurityGroupIngress>>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub name: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub name_prefix: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub owner_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub provider: Option<SmolStr>,
	/// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub region: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub revoke_rules_on_delete: Option<bool>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tags: Option<Map<SmolStr, SmolStr>>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tags_all: Option<Map<SmolStr, SmolStr>>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub vpc_id: Option<SmolStr>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub timeouts: Option<Vec<AwsSecurityGroupResourceBlockTypeTimeouts>>,
}
impl terra::ToJson for AwsSecurityGroupDetails {
	fn to_json(&self) -> serde_json::Value {
		serde_json::to_value(self).expect("serialization should not fail")
	}
}
impl terra::Resource for AwsSecurityGroupDetails {
	fn resource_type(&self) -> &'static str { "aws_security_group" }
	fn provider(&self) -> &'static terra::Provider { &terra::Provider::AWS }
	fn validate_definition(
		&self,
	) -> Result<(), terra::ResourceValidationError> {
		if self.arn.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "arn",
				},
			);
		}
		if self.owner_id.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "owner_id",
				},
			);
		}
		Ok(())
	}
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct AwsSecurityGroupEgress {
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub cidr_blocks: Vec<SmolStr>,
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub description: SmolStr,
	pub from_port: i64,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub ipv6_cidr_blocks: Vec<SmolStr>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub prefix_list_ids: Vec<SmolStr>,
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub protocol: SmolStr,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub security_groups: Vec<SmolStr>,
	pub self_ref: bool,
	pub to_port: i64,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct AwsSecurityGroupIngress {
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub cidr_blocks: Vec<SmolStr>,
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub description: SmolStr,
	pub from_port: i64,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub ipv6_cidr_blocks: Vec<SmolStr>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub prefix_list_ids: Vec<SmolStr>,
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub protocol: SmolStr,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub security_groups: Vec<SmolStr>,
	pub self_ref: bool,
	pub to_port: i64,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct AwsSecurityGroupRuleDetails {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub cidr_blocks: Option<Vec<SmolStr>>,
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
	pub description: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub for_each: Option<Vec<SmolStr>>,
	/// ## Attribute
	/// `required`
	pub from_port: i64,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ipv6_cidr_blocks: Option<Vec<SmolStr>>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub prefix_list_ids: Option<Vec<SmolStr>>,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub protocol: SmolStr,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub provider: Option<SmolStr>,
	/// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub region: Option<SmolStr>,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub security_group_id: SmolStr,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub security_group_rule_id: Option<SmolStr>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub self_ref: Option<bool>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub source_security_group_id: Option<SmolStr>,
	/// ## Attribute
	/// `required`
	pub to_port: i64,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub r#type: SmolStr,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub timeouts: Option<Vec<AwsSecurityGroupRuleResourceBlockTypeTimeouts>>,
}
impl terra::ToJson for AwsSecurityGroupRuleDetails {
	fn to_json(&self) -> serde_json::Value {
		serde_json::to_value(self).expect("serialization should not fail")
	}
}
impl terra::Resource for AwsSecurityGroupRuleDetails {
	fn resource_type(&self) -> &'static str { "aws_security_group_rule" }
	fn provider(&self) -> &'static terra::Provider { &terra::Provider::AWS }
	fn validate_definition(
		&self,
	) -> Result<(), terra::ResourceValidationError> {
		if self.protocol.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "protocol",
			});
		}
		if self.security_group_id.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "security_group_id",
			});
		}
		if self.security_group_rule_id.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "security_group_rule_id",
				},
			);
		}
		if self.r#type.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "type",
			});
		}
		Ok(())
	}
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct AwsSubnetDetails {
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub arn: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub assign_ipv6_address_on_creation: Option<bool>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub availability_zone: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub availability_zone_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub cidr_block: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub count: Option<i64>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub customer_owned_ipv4_pool: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub depends_on: Option<Vec<SmolStr>>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub enable_dns64: Option<bool>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub enable_lni_at_device_index: Option<i64>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub enable_resource_name_dns_a_record_on_launch: Option<bool>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub enable_resource_name_dns_aaaa_record_on_launch: Option<bool>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub for_each: Option<Vec<SmolStr>>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ipv4_ipam_pool_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ipv4_netmask_length: Option<i64>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ipv6_cidr_block: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ipv6_cidr_block_association_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ipv6_ipam_pool_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ipv6_native: Option<bool>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ipv6_netmask_length: Option<i64>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub map_customer_owned_ip_on_launch: Option<bool>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub map_public_ip_on_launch: Option<bool>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub outpost_arn: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub owner_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub private_dns_hostname_type_on_launch: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub provider: Option<SmolStr>,
	/// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub region: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tags: Option<Map<SmolStr, SmolStr>>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tags_all: Option<Map<SmolStr, SmolStr>>,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub vpc_id: SmolStr,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub timeouts: Option<Vec<AwsSubnetResourceBlockTypeTimeouts>>,
}
impl terra::ToJson for AwsSubnetDetails {
	fn to_json(&self) -> serde_json::Value {
		serde_json::to_value(self).expect("serialization should not fail")
	}
}
impl terra::Resource for AwsSubnetDetails {
	fn resource_type(&self) -> &'static str { "aws_subnet" }
	fn provider(&self) -> &'static terra::Provider { &terra::Provider::AWS }
	fn validate_definition(
		&self,
	) -> Result<(), terra::ResourceValidationError> {
		if self.arn.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "arn",
				},
			);
		}
		if self.ipv6_cidr_block_association_id.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "ipv6_cidr_block_association_id",
				},
			);
		}
		if self.owner_id.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "owner_id",
				},
			);
		}
		if self.vpc_id.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "vpc_id",
			});
		}
		Ok(())
	}
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct AwsVpcDetails {
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub arn: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub assign_generated_ipv6_cidr_block: Option<bool>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub cidr_block: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub count: Option<i64>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub default_network_acl_id: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub default_route_table_id: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub default_security_group_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub depends_on: Option<Vec<SmolStr>>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub dhcp_options_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub enable_dns_hostnames: Option<bool>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub enable_dns_support: Option<bool>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub enable_network_address_usage_metrics: Option<bool>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub for_each: Option<Vec<SmolStr>>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub instance_tenancy: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ipv4_ipam_pool_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ipv4_netmask_length: Option<i64>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ipv6_association_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ipv6_cidr_block: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ipv6_cidr_block_network_border_group: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ipv6_ipam_pool_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ipv6_netmask_length: Option<i64>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub main_route_table_id: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub owner_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub provider: Option<SmolStr>,
	/// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub region: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tags: Option<Map<SmolStr, SmolStr>>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tags_all: Option<Map<SmolStr, SmolStr>>,
}
impl terra::ToJson for AwsVpcDetails {
	fn to_json(&self) -> serde_json::Value {
		serde_json::to_value(self).expect("serialization should not fail")
	}
}
impl terra::Resource for AwsVpcDetails {
	fn resource_type(&self) -> &'static str { "aws_vpc" }
	fn provider(&self) -> &'static terra::Provider { &terra::Provider::AWS }
	fn validate_definition(
		&self,
	) -> Result<(), terra::ResourceValidationError> {
		if self.arn.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "arn",
				},
			);
		}
		if self.default_network_acl_id.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "default_network_acl_id",
				},
			);
		}
		if self.default_route_table_id.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "default_route_table_id",
				},
			);
		}
		if self.default_security_group_id.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "default_security_group_id",
				},
			);
		}
		if self.dhcp_options_id.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "dhcp_options_id",
				},
			);
		}
		if self.ipv6_association_id.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "ipv6_association_id",
				},
			);
		}
		if self.main_route_table_id.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "main_route_table_id",
				},
			);
		}
		if self.owner_id.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "owner_id",
				},
			);
		}
		Ok(())
	}
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "timeouts")]
pub struct AwsInternetGatewayResourceBlockTypeTimeouts {
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
#[serde(rename = "timeouts")]
pub struct AwsRouteResourceBlockTypeTimeouts {
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
#[serde(rename = "timeouts")]
pub struct AwsRouteTableAssociationResourceBlockTypeTimeouts {
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
#[serde(rename = "timeouts")]
pub struct AwsRouteTableResourceBlockTypeTimeouts {
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
#[serde(rename = "timeouts")]
pub struct AwsSecurityGroupResourceBlockTypeTimeouts {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub create: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub delete: Option<SmolStr>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "timeouts")]
pub struct AwsSecurityGroupRuleResourceBlockTypeTimeouts {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub create: Option<SmolStr>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "timeouts")]
pub struct AwsSubnetResourceBlockTypeTimeouts {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub create: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub delete: Option<SmolStr>,
}
