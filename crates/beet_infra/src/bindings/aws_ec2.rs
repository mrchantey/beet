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
pub struct AwsEipAssociationDetails {
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub allocation_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub allow_reassociation: Option<bool>,
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
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub instance_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub network_interface_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub private_ip_address: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub provider: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub public_ip: Option<SmolStr>,
	/// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub region: Option<SmolStr>,
}
impl terra::ToJson for AwsEipAssociationDetails {
	fn to_json(&self) -> serde_json::Value {
		serde_json::to_value(self).expect("serialization should not fail")
	}
}
impl terra::Resource for AwsEipAssociationDetails {
	fn resource_type(&self) -> &'static str { "aws_eip_association" }
	fn provider(&self) -> &'static terra::Provider { &terra::Provider::AWS }
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct AwsEipDetails {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub address: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub allocation_id: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub arn: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub associate_with_private_ip: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub association_id: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub carrier_ip: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub count: Option<i64>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub customer_owned_ip: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub customer_owned_ipv4_pool: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub depends_on: Option<Vec<SmolStr>>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub domain: Option<SmolStr>,
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
	pub instance: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ipam_pool_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub network_border_group: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub network_interface: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub private_dns: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub private_ip: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub provider: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ptr_record: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub public_dns: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub public_ip: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub public_ipv4_pool: Option<SmolStr>,
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
	#[serde(skip_serializing_if = "Option::is_none")]
	pub timeouts: Option<Vec<AwsEipResourceBlockTypeTimeouts>>,
}
impl terra::ToJson for AwsEipDetails {
	fn to_json(&self) -> serde_json::Value {
		serde_json::to_value(self).expect("serialization should not fail")
	}
}
impl terra::Resource for AwsEipDetails {
	fn resource_type(&self) -> &'static str { "aws_eip" }
	fn provider(&self) -> &'static terra::Provider { &terra::Provider::AWS }
	fn validate_definition(
		&self,
	) -> Result<(), terra::ResourceValidationError> {
		if self.allocation_id.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "allocation_id",
				},
			);
		}
		if self.arn.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "arn",
				},
			);
		}
		if self.association_id.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "association_id",
				},
			);
		}
		if self.carrier_ip.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "carrier_ip",
				},
			);
		}
		if self.customer_owned_ip.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "customer_owned_ip",
				},
			);
		}
		if self.private_dns.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "private_dns",
				},
			);
		}
		if self.private_ip.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "private_ip",
				},
			);
		}
		if self.ptr_record.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "ptr_record",
				},
			);
		}
		if self.public_dns.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "public_dns",
				},
			);
		}
		if self.public_ip.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "public_ip",
				},
			);
		}
		Ok(())
	}
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct AwsIamInstanceProfileDetails {
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub arn: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub count: Option<i64>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub create_date: Option<SmolStr>,
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
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub name: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub name_prefix: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub path: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub provider: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub role: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tags: Option<Map<SmolStr, SmolStr>>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tags_all: Option<Map<SmolStr, SmolStr>>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub unique_id: Option<SmolStr>,
}
impl terra::ToJson for AwsIamInstanceProfileDetails {
	fn to_json(&self) -> serde_json::Value {
		serde_json::to_value(self).expect("serialization should not fail")
	}
}
impl terra::Resource for AwsIamInstanceProfileDetails {
	fn resource_type(&self) -> &'static str { "aws_iam_instance_profile" }
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
		if self.create_date.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "create_date",
				},
			);
		}
		if self.unique_id.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "unique_id",
				},
			);
		}
		Ok(())
	}
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct AwsInstanceDetails {
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ami: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub arn: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub associate_public_ip_address: Option<bool>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub availability_zone: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub count: Option<i64>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub depends_on: Option<Vec<SmolStr>>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub disable_api_stop: Option<bool>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub disable_api_termination: Option<bool>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ebs_optimized: Option<bool>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub enable_primary_ipv6: Option<bool>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub for_each: Option<Vec<SmolStr>>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub force_destroy: Option<bool>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub get_password_data: Option<bool>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub hibernation: Option<bool>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub host_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub host_resource_group_arn: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub iam_instance_profile: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub instance_initiated_shutdown_behavior: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub instance_lifecycle: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub instance_state: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub instance_type: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ipv6_address_count: Option<i64>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ipv6_addresses: Option<Vec<SmolStr>>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub key_name: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub monitoring: Option<bool>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub outpost_arn: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub password_data: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub placement_group: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub placement_group_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub placement_partition_number: Option<i64>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub primary_network_interface_id: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub private_dns: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub private_ip: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub provider: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub public_dns: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub public_ip: Option<SmolStr>,
	/// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub region: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub secondary_private_ips: Option<Vec<SmolStr>>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub security_groups: Option<Vec<SmolStr>>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub source_dest_check: Option<bool>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub spot_instance_request_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub subnet_id: Option<SmolStr>,
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
	pub tenancy: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub user_data: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub user_data_base64: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub user_data_replace_on_change: Option<bool>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub volume_tags: Option<Map<SmolStr, SmolStr>>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub vpc_security_group_ids: Option<Vec<SmolStr>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub capacity_reservation_specification: Option<
		Vec<AwsInstanceResourceBlockTypeCapacityReservationSpecification>,
	>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub cpu_options: Option<Vec<AwsInstanceResourceBlockTypeCpuOptions>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub credit_specification:
		Option<Vec<AwsInstanceResourceBlockTypeCreditSpecification>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ebs_block_device:
		Option<Vec<AwsInstanceResourceBlockTypeEbsBlockDevice>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub enclave_options:
		Option<Vec<AwsInstanceResourceBlockTypeEnclaveOptions>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ephemeral_block_device:
		Option<Vec<AwsInstanceResourceBlockTypeEphemeralBlockDevice>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub instance_market_options:
		Option<Vec<AwsInstanceResourceBlockTypeInstanceMarketOptions>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub launch_template:
		Option<Vec<AwsInstanceResourceBlockTypeLaunchTemplate>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub maintenance_options:
		Option<Vec<AwsInstanceResourceBlockTypeMaintenanceOptions>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub metadata_options:
		Option<Vec<AwsInstanceResourceBlockTypeMetadataOptions>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub network_interface:
		Option<Vec<AwsInstanceResourceBlockTypeNetworkInterface>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub primary_network_interface:
		Option<Vec<AwsInstanceResourceBlockTypePrimaryNetworkInterface>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub private_dns_name_options:
		Option<Vec<AwsInstanceResourceBlockTypePrivateDnsNameOptions>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub root_block_device:
		Option<Vec<AwsInstanceResourceBlockTypeRootBlockDevice>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub secondary_network_interface:
		Option<Vec<AwsInstanceResourceBlockTypeSecondaryNetworkInterface>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub timeouts: Option<Vec<AwsInstanceResourceBlockTypeTimeouts>>,
}
impl terra::ToJson for AwsInstanceDetails {
	fn to_json(&self) -> serde_json::Value {
		serde_json::to_value(self).expect("serialization should not fail")
	}
}
impl terra::Resource for AwsInstanceDetails {
	fn resource_type(&self) -> &'static str { "aws_instance" }
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
		if self.instance_lifecycle.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "instance_lifecycle",
				},
			);
		}
		if self.instance_state.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "instance_state",
				},
			);
		}
		if self.outpost_arn.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "outpost_arn",
				},
			);
		}
		if self.password_data.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "password_data",
				},
			);
		}
		if self.primary_network_interface_id.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "primary_network_interface_id",
				},
			);
		}
		if self.private_dns.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "private_dns",
				},
			);
		}
		if self.public_dns.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "public_dns",
				},
			);
		}
		if self.public_ip.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "public_ip",
				},
			);
		}
		if self.spot_instance_request_id.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "spot_instance_request_id",
				},
			);
		}
		Ok(())
	}
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct AwsKeyPairDetails {
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
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub fingerprint: Option<SmolStr>,
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
	pub key_name: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub key_name_prefix: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub key_pair_id: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub key_type: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub provider: Option<SmolStr>,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub public_key: SmolStr,
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
impl terra::ToJson for AwsKeyPairDetails {
	fn to_json(&self) -> serde_json::Value {
		serde_json::to_value(self).expect("serialization should not fail")
	}
}
impl terra::Resource for AwsKeyPairDetails {
	fn resource_type(&self) -> &'static str { "aws_key_pair" }
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
		if self.fingerprint.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "fingerprint",
				},
			);
		}
		if self.key_pair_id.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "key_pair_id",
				},
			);
		}
		if self.key_type.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "key_type",
				},
			);
		}
		if self.public_key.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "public_key",
			});
		}
		Ok(())
	}
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "timeouts")]
pub struct AwsEipResourceBlockTypeTimeouts {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub delete: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub read: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub update: Option<SmolStr>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "capacity_reservation_specification")]
pub struct AwsInstanceResourceBlockTypeCapacityReservationSpecification {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity_reservation_preference: Option<SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity_reservation_target: Option<
        Vec<CapacityReservationSpecificationResourceBlockTypeCapacityReservationTarget>,
    >,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "cpu_options")]
pub struct AwsInstanceResourceBlockTypeCpuOptions {
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub amd_sev_snp: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub core_count: Option<i64>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub nested_virtualization: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub threads_per_core: Option<i64>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "credit_specification")]
pub struct AwsInstanceResourceBlockTypeCreditSpecification {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub cpu_credits: Option<SmolStr>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "ebs_block_device")]
pub struct AwsInstanceResourceBlockTypeEbsBlockDevice {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub delete_on_termination: Option<bool>,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub device_name: SmolStr,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub encrypted: Option<bool>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub iops: Option<i64>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub kms_key_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub snapshot_id: Option<SmolStr>,
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
	pub throughput: Option<i64>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub volume_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub volume_size: Option<i64>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub volume_type: Option<SmolStr>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "enclave_options")]
pub struct AwsInstanceResourceBlockTypeEnclaveOptions {
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub enabled: Option<bool>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "ephemeral_block_device")]
pub struct AwsInstanceResourceBlockTypeEphemeralBlockDevice {
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub device_name: SmolStr,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub no_device: Option<bool>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub virtual_name: Option<SmolStr>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "instance_market_options")]
pub struct AwsInstanceResourceBlockTypeInstanceMarketOptions {
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub market_type: Option<SmolStr>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub spot_options:
		Option<Vec<InstanceMarketOptionsResourceBlockTypeSpotOptions>>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "launch_template")]
pub struct AwsInstanceResourceBlockTypeLaunchTemplate {
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub name: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub version: Option<SmolStr>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "maintenance_options")]
pub struct AwsInstanceResourceBlockTypeMaintenanceOptions {
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub auto_recovery: Option<SmolStr>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "metadata_options")]
pub struct AwsInstanceResourceBlockTypeMetadataOptions {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub http_endpoint: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub http_protocol_ipv6: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub http_put_response_hop_limit: Option<i64>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub http_tokens: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub instance_metadata_tags: Option<SmolStr>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "network_interface")]
pub struct AwsInstanceResourceBlockTypeNetworkInterface {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub delete_on_termination: Option<bool>,
	/// ## Attribute
	/// `required`
	pub device_index: i64,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub network_card_index: Option<i64>,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub network_interface_id: SmolStr,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "primary_network_interface")]
pub struct AwsInstanceResourceBlockTypePrimaryNetworkInterface {
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub delete_on_termination: Option<bool>,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub network_interface_id: SmolStr,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "private_dns_name_options")]
pub struct AwsInstanceResourceBlockTypePrivateDnsNameOptions {
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub enable_resource_name_dns_a_record: Option<bool>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub enable_resource_name_dns_aaaa_record: Option<bool>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub hostname_type: Option<SmolStr>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "root_block_device")]
pub struct AwsInstanceResourceBlockTypeRootBlockDevice {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub delete_on_termination: Option<bool>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub device_name: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub encrypted: Option<bool>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub iops: Option<i64>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub kms_key_id: Option<SmolStr>,
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
	pub throughput: Option<i64>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub volume_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub volume_size: Option<i64>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub volume_type: Option<SmolStr>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "secondary_network_interface")]
pub struct AwsInstanceResourceBlockTypeSecondaryNetworkInterface {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub delete_on_termination: Option<bool>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub device_index: Option<i64>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub interface_type: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub mac_address: Option<SmolStr>,
	/// ## Attribute
	/// `required`
	pub network_card_index: i64,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub private_ip_address_count: Option<i64>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub private_ip_addresses: Option<Vec<SmolStr>>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub secondary_interface_id: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub secondary_network_id: Option<SmolStr>,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub secondary_subnet_id: SmolStr,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub source_dest_check: Option<bool>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub status: Option<SmolStr>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "timeouts")]
pub struct AwsInstanceResourceBlockTypeTimeouts {
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
	pub read: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub update: Option<SmolStr>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "capacity_reservation_target")]
pub struct CapacityReservationSpecificationResourceBlockTypeCapacityReservationTarget
{
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub capacity_reservation_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub capacity_reservation_resource_group_arn: Option<SmolStr>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "spot_options")]
pub struct InstanceMarketOptionsResourceBlockTypeSpotOptions {
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub instance_interruption_behavior: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub max_price: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub spot_instance_type: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub valid_until: Option<SmolStr>,
}
