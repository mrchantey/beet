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
pub struct AwsAppautoscalingPolicyDetails {
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alarm_arns: Option<Vec<SmolStr>>,
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
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub name: SmolStr,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_type: Option<SmolStr>,
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
    pub resource_id: SmolStr,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub scalable_dimension: SmolStr,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub service_namespace: SmolStr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub predictive_scaling_policy_configuration: Option<
        Vec<AwsAppautoscalingPolicyResourceBlockTypePredictiveScalingPolicyConfiguration>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_scaling_policy_configuration: Option<
        Vec<AwsAppautoscalingPolicyResourceBlockTypeStepScalingPolicyConfiguration>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_tracking_scaling_policy_configuration: Option<
        Vec<
            AwsAppautoscalingPolicyResourceBlockTypeTargetTrackingScalingPolicyConfiguration,
        >,
    >,
}
impl terra::ToJson for AwsAppautoscalingPolicyDetails {
	fn to_json(&self) -> serde_json::Value {
		serde_json::to_value(self).expect("serialization should not fail")
	}
}
impl terra::Resource for AwsAppautoscalingPolicyDetails {
	fn resource_type(&self) -> &'static str { "aws_appautoscaling_policy" }
	fn provider(&self) -> &'static terra::Provider { &terra::Provider::AWS }
	fn validate_definition(
		&self,
	) -> Result<(), terra::ResourceValidationError> {
		if self.alarm_arns.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "alarm_arns",
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
		if self.name.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "name",
			});
		}
		if self.resource_id.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "resource_id",
			});
		}
		if self.scalable_dimension.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "scalable_dimension",
			});
		}
		if self.service_namespace.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "service_namespace",
			});
		}
		Ok(())
	}
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct AwsAppautoscalingTargetDetails {
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
	/// `required`
	pub max_capacity: i64,
	/// ## Attribute
	/// `required`
	pub min_capacity: i64,
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
	pub resource_id: SmolStr,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub role_arn: Option<SmolStr>,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub scalable_dimension: SmolStr,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub service_namespace: SmolStr,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tags: Option<Map<SmolStr, SmolStr>>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tags_all: Option<Map<SmolStr, SmolStr>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub suspended_state:
		Option<Vec<AwsAppautoscalingTargetResourceBlockTypeSuspendedState>>,
}
impl terra::ToJson for AwsAppautoscalingTargetDetails {
	fn to_json(&self) -> serde_json::Value {
		serde_json::to_value(self).expect("serialization should not fail")
	}
}
impl terra::Resource for AwsAppautoscalingTargetDetails {
	fn resource_type(&self) -> &'static str { "aws_appautoscaling_target" }
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
		if self.resource_id.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "resource_id",
			});
		}
		if self.scalable_dimension.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "scalable_dimension",
			});
		}
		if self.service_namespace.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "service_namespace",
			});
		}
		Ok(())
	}
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "predictive_scaling_policy_configuration")]
pub struct AwsAppautoscalingPolicyResourceBlockTypePredictiveScalingPolicyConfiguration {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_capacity_breach_behavior: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_capacity_buffer: Option<i64>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduling_buffer_time: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metric_specification: Option<
        Vec<PredictiveScalingPolicyConfigurationResourceBlockTypeMetricSpecification>,
    >,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "step_scaling_policy_configuration")]
pub struct AwsAppautoscalingPolicyResourceBlockTypeStepScalingPolicyConfiguration
{
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub adjustment_type: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub cooldown: Option<i64>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub metric_aggregation_type: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub min_adjustment_magnitude: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub step_adjustment: Option<
		Vec<StepScalingPolicyConfigurationResourceBlockTypeStepAdjustment>,
	>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "target_tracking_scaling_policy_configuration")]
pub struct AwsAppautoscalingPolicyResourceBlockTypeTargetTrackingScalingPolicyConfiguration {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_scale_in: Option<bool>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale_in_cooldown: Option<i64>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale_out_cooldown: Option<i64>,
    /// ## Attribute
    /// `required`
    pub target_value: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customized_metric_specification: Option<
        Vec<
            TargetTrackingScalingPolicyConfigurationResourceBlockTypeCustomizedMetricSpecification,
        >,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub predefined_metric_specification: Option<
        Vec<
            TargetTrackingScalingPolicyConfigurationResourceBlockTypePredefinedMetricSpecification,
        >,
    >,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "suspended_state")]
pub struct AwsAppautoscalingTargetResourceBlockTypeSuspendedState {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub dynamic_scaling_in_suspended: Option<bool>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub dynamic_scaling_out_suspended: Option<bool>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub scheduled_scaling_suspended: Option<bool>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "dimensions")]
pub struct CustomizedMetricSpecificationResourceBlockTypeDimensions {
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub name: SmolStr,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub value: SmolStr,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "metrics")]
pub struct CustomizedMetricSpecificationResourceBlockTypeMetrics {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub expression: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub id: SmolStr,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub label: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub return_data: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub metric_stat: Option<Vec<MetricsResourceBlockTypeMetricStat>>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "dimensions")]
pub struct MetricResourceBlockTypeDimensions {
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub name: SmolStr,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub value: SmolStr,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "predefined_load_metric_specification")]
pub struct MetricSpecificationResourceBlockTypePredefinedLoadMetricSpecification
{
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub predefined_metric_type: SmolStr,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub resource_label: Option<SmolStr>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "predefined_metric_pair_specification")]
pub struct MetricSpecificationResourceBlockTypePredefinedMetricPairSpecification
{
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub predefined_metric_type: SmolStr,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub resource_label: Option<SmolStr>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "predefined_scaling_metric_specification")]
pub struct MetricSpecificationResourceBlockTypePredefinedScalingMetricSpecification
{
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub predefined_metric_type: SmolStr,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub resource_label: Option<SmolStr>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "metric")]
pub struct MetricStatResourceBlockTypeMetric {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub metric_name: SmolStr,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub namespace: SmolStr,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub dimensions: Option<Vec<MetricResourceBlockTypeDimensions>>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "metric_stat")]
pub struct MetricsResourceBlockTypeMetricStat {
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub stat: SmolStr,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub unit: Option<SmolStr>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub metric: Option<Vec<MetricStatResourceBlockTypeMetric>>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "metric_specification")]
pub struct PredictiveScalingPolicyConfigurationResourceBlockTypeMetricSpecification {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub target_value: SmolStr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub predefined_load_metric_specification: Option<
        Vec<MetricSpecificationResourceBlockTypePredefinedLoadMetricSpecification>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub predefined_metric_pair_specification: Option<
        Vec<MetricSpecificationResourceBlockTypePredefinedMetricPairSpecification>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub predefined_scaling_metric_specification: Option<
        Vec<MetricSpecificationResourceBlockTypePredefinedScalingMetricSpecification>,
    >,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "step_adjustment")]
pub struct StepScalingPolicyConfigurationResourceBlockTypeStepAdjustment {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub metric_interval_lower_bound: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub metric_interval_upper_bound: Option<SmolStr>,
	/// ## Attribute
	/// `required`
	pub scaling_adjustment: i64,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "customized_metric_specification")]
pub struct TargetTrackingScalingPolicyConfigurationResourceBlockTypeCustomizedMetricSpecification
{
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub metric_name: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub namespace: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub statistic: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub unit: Option<SmolStr>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub dimensions:
		Option<Vec<CustomizedMetricSpecificationResourceBlockTypeDimensions>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub metrics:
		Option<Vec<CustomizedMetricSpecificationResourceBlockTypeMetrics>>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "predefined_metric_specification")]
pub struct TargetTrackingScalingPolicyConfigurationResourceBlockTypePredefinedMetricSpecification
{
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub predefined_metric_type: SmolStr,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub resource_label: Option<SmolStr>,
}
