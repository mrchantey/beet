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
pub struct AwsDynamodbTableDetails {
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub arn: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub billing_mode: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub count: Option<i64>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub deletion_protection_enabled: Option<bool>,
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
	pub hash_key: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<SmolStr>,
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
	pub range_key: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub read_capacity: Option<i64>,
	/// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub region: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub restore_backup_arn: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub restore_date_time: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub restore_source_name: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub restore_source_table_arn: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub restore_to_latest_time: Option<bool>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub stream_arn: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub stream_enabled: Option<bool>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub stream_label: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub stream_view_type: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub table_class: Option<SmolStr>,
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
	pub write_capacity: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub attribute: Option<Vec<AwsDynamodbTableResourceBlockTypeAttribute>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub global_secondary_index:
		Option<Vec<AwsDynamodbTableResourceBlockTypeGlobalSecondaryIndex>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub global_table_witness:
		Option<Vec<AwsDynamodbTableResourceBlockTypeGlobalTableWitness>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub import_table: Option<Vec<AwsDynamodbTableResourceBlockTypeImportTable>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub local_secondary_index:
		Option<Vec<AwsDynamodbTableResourceBlockTypeLocalSecondaryIndex>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub on_demand_throughput:
		Option<Vec<AwsDynamodbTableResourceBlockTypeOnDemandThroughput>>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub point_in_time_recovery:
		Option<Vec<AwsDynamodbTableResourceBlockTypePointInTimeRecovery>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub replica: Option<Vec<AwsDynamodbTableResourceBlockTypeReplica>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub server_side_encryption:
		Option<Vec<AwsDynamodbTableResourceBlockTypeServerSideEncryption>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub timeouts: Option<Vec<AwsDynamodbTableResourceBlockTypeTimeouts>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ttl: Option<Vec<AwsDynamodbTableResourceBlockTypeTtl>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub warm_throughput:
		Option<Vec<AwsDynamodbTableResourceBlockTypeWarmThroughput>>,
}
impl terra::ToJson for AwsDynamodbTableDetails {
	fn to_json(&self) -> serde_json::Value {
		serde_json::to_value(self).expect("serialization should not fail")
	}
}
impl terra::Resource for AwsDynamodbTableDetails {
	fn resource_type(&self) -> &'static str { "aws_dynamodb_table" }
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
		if self.name.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "name",
			});
		}
		if self.stream_arn.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "stream_arn",
				},
			);
		}
		if self.stream_label.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "stream_label",
				},
			);
		}
		Ok(())
	}
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "attribute")]
pub struct AwsDynamodbTableResourceBlockTypeAttribute {
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub name: SmolStr,
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub r#type: SmolStr,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "global_secondary_index")]
pub struct AwsDynamodbTableResourceBlockTypeGlobalSecondaryIndex {
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub hash_key: Option<SmolStr>,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub name: SmolStr,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub non_key_attributes: Option<Vec<SmolStr>>,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub projection_type: SmolStr,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub range_key: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub read_capacity: Option<i64>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub write_capacity: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub key_schema: Option<Vec<GlobalSecondaryIndexResourceBlockTypeKeySchema>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub on_demand_throughput:
		Option<Vec<GlobalSecondaryIndexResourceBlockTypeOnDemandThroughput>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub warm_throughput:
		Option<Vec<GlobalSecondaryIndexResourceBlockTypeWarmThroughput>>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "global_table_witness")]
pub struct AwsDynamodbTableResourceBlockTypeGlobalTableWitness {
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub region_name: Option<SmolStr>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "import_table")]
pub struct AwsDynamodbTableResourceBlockTypeImportTable {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub input_compression_type: Option<SmolStr>,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub input_format: SmolStr,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub s3_bucket_source:
		Option<Vec<ImportTableResourceBlockTypeS3BucketSource>>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "local_secondary_index")]
pub struct AwsDynamodbTableResourceBlockTypeLocalSecondaryIndex {
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub name: SmolStr,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub non_key_attributes: Option<Vec<SmolStr>>,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub projection_type: SmolStr,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub range_key: SmolStr,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "on_demand_throughput")]
pub struct AwsDynamodbTableResourceBlockTypeOnDemandThroughput {
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub max_read_request_units: Option<i64>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub max_write_request_units: Option<i64>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "point_in_time_recovery")]
pub struct AwsDynamodbTableResourceBlockTypePointInTimeRecovery {
	/// ## Attribute
	/// `required`
	pub enabled: bool,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub recovery_period_in_days: Option<i64>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "replica")]
pub struct AwsDynamodbTableResourceBlockTypeReplica {
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub arn: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub consistency_mode: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub deletion_protection_enabled: Option<bool>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub kms_key_arn: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub point_in_time_recovery: Option<bool>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub propagate_tags: Option<bool>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub region_name: SmolStr,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub stream_arn: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub stream_label: Option<SmolStr>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "server_side_encryption")]
pub struct AwsDynamodbTableResourceBlockTypeServerSideEncryption {
	/// ## Attribute
	/// `required`
	pub enabled: bool,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub kms_key_arn: Option<SmolStr>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "timeouts")]
pub struct AwsDynamodbTableResourceBlockTypeTimeouts {
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
#[serde(rename = "ttl")]
pub struct AwsDynamodbTableResourceBlockTypeTtl {
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub attribute_name: Option<SmolStr>,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub enabled: Option<bool>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "warm_throughput")]
pub struct AwsDynamodbTableResourceBlockTypeWarmThroughput {
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub read_units_per_second: Option<i64>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub write_units_per_second: Option<i64>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "key_schema")]
pub struct GlobalSecondaryIndexResourceBlockTypeKeySchema {
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub attribute_name: SmolStr,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub key_type: SmolStr,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "on_demand_throughput")]
pub struct GlobalSecondaryIndexResourceBlockTypeOnDemandThroughput {
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub max_read_request_units: Option<i64>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub max_write_request_units: Option<i64>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "warm_throughput")]
pub struct GlobalSecondaryIndexResourceBlockTypeWarmThroughput {
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub read_units_per_second: Option<i64>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub write_units_per_second: Option<i64>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "s3_bucket_source")]
pub struct ImportTableResourceBlockTypeS3BucketSource {
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub bucket: SmolStr,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub bucket_owner: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub key_prefix: Option<SmolStr>,
}
