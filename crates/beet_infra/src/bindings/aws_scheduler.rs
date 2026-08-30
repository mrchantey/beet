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
pub struct AwsSchedulerScheduleDetails {
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub action_after_completion: Option<SmolStr>,
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
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub end_date: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub for_each: Option<Vec<SmolStr>>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub group_name: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub kms_key_arn: Option<SmolStr>,
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
	pub provider: Option<SmolStr>,
	/// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub region: Option<SmolStr>,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub schedule_expression: SmolStr,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub schedule_expression_timezone: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub start_date: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub state: Option<SmolStr>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub flexible_time_window:
		Option<Vec<AwsSchedulerScheduleResourceBlockTypeFlexibleTimeWindow>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub target: Option<Vec<AwsSchedulerScheduleResourceBlockTypeTarget>>,
}
impl terra::ToJson for AwsSchedulerScheduleDetails {
	fn to_json(&self) -> serde_json::Value {
		serde_json::to_value(self).expect("serialization should not fail")
	}
}
impl terra::Resource for AwsSchedulerScheduleDetails {
	fn resource_type(&self) -> &'static str { "aws_scheduler_schedule" }
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
		if self.schedule_expression.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "schedule_expression",
			});
		}
		Ok(())
	}
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "flexible_time_window")]
pub struct AwsSchedulerScheduleResourceBlockTypeFlexibleTimeWindow {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub maximum_window_in_minutes: Option<i64>,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub mode: SmolStr,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "target")]
pub struct AwsSchedulerScheduleResourceBlockTypeTarget {
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub arn: SmolStr,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub input: Option<SmolStr>,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub role_arn: SmolStr,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub dead_letter_config:
		Option<Vec<TargetResourceBlockTypeDeadLetterConfig>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ecs_parameters: Option<Vec<TargetResourceBlockTypeEcsParameters>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub eventbridge_parameters:
		Option<Vec<TargetResourceBlockTypeEventbridgeParameters>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub kinesis_parameters:
		Option<Vec<TargetResourceBlockTypeKinesisParameters>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub retry_policy: Option<Vec<TargetResourceBlockTypeRetryPolicy>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub sqs_parameters: Option<Vec<TargetResourceBlockTypeSqsParameters>>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "capacity_provider_strategy")]
pub struct EcsParametersResourceBlockTypeCapacityProviderStrategy {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub base: Option<i64>,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub capacity_provider: SmolStr,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub weight: Option<i64>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "network_configuration")]
pub struct EcsParametersResourceBlockTypeNetworkConfiguration {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub assign_public_ip: Option<bool>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub security_groups: Option<Vec<SmolStr>>,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub subnets: Vec<SmolStr>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "placement_constraints")]
pub struct EcsParametersResourceBlockTypePlacementConstraints {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub expression: Option<SmolStr>,
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub r#type: SmolStr,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "placement_strategy")]
pub struct EcsParametersResourceBlockTypePlacementStrategy {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub field: Option<SmolStr>,
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub r#type: SmolStr,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "dead_letter_config")]
pub struct TargetResourceBlockTypeDeadLetterConfig {
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub arn: SmolStr,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "ecs_parameters")]
pub struct TargetResourceBlockTypeEcsParameters {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub enable_ecs_managed_tags: Option<bool>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub enable_execute_command: Option<bool>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub group: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub launch_type: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub platform_version: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub propagate_tags: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub reference_id: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tags: Option<Map<SmolStr, SmolStr>>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub task_count: Option<i64>,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub task_definition_arn: SmolStr,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub capacity_provider_strategy:
		Option<Vec<EcsParametersResourceBlockTypeCapacityProviderStrategy>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub network_configuration:
		Option<Vec<EcsParametersResourceBlockTypeNetworkConfiguration>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub placement_constraints:
		Option<Vec<EcsParametersResourceBlockTypePlacementConstraints>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub placement_strategy:
		Option<Vec<EcsParametersResourceBlockTypePlacementStrategy>>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "eventbridge_parameters")]
pub struct TargetResourceBlockTypeEventbridgeParameters {
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub detail_type: SmolStr,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub source: SmolStr,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "kinesis_parameters")]
pub struct TargetResourceBlockTypeKinesisParameters {
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub partition_key: SmolStr,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "retry_policy")]
pub struct TargetResourceBlockTypeRetryPolicy {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub maximum_event_age_in_seconds: Option<i64>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub maximum_retry_attempts: Option<i64>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "sqs_parameters")]
pub struct TargetResourceBlockTypeSqsParameters {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub message_group_id: Option<SmolStr>,
}
