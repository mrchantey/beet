//! Auto-generated Terraform provider bindings — do not edit!
//! Auto-generated Terraform provider bindings — do not edit!
//! Auto-generated Terraform provider bindings — do not edit!

#![allow(unused_imports, non_snake_case, non_camel_case_types, non_upper_case_globals)]
use std::collections::BTreeMap as Map;
use serde::{Serialize, Deserialize};
use serde_json;
#[allow(unused)]
use beet_core::prelude::*;
#[allow(unused)]
use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
pub struct AwsCloudwatchLogGroupDetails {
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arn: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    /// ## Attribute
    /// `optional`, `computed`
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
    pub id: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kms_key_id: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_group_class: Option<SmolStr>,
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
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_in_days: Option<i64>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_destroy: Option<bool>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Map<SmolStr, SmolStr>>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags_all: Option<Map<SmolStr, SmolStr>>,
}
impl terra::ToJson for AwsCloudwatchLogGroupDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl terra::Resource for AwsCloudwatchLogGroupDetails {
    fn resource_type(&self) -> &'static str {
        "aws_cloudwatch_log_group"
    }
    fn provider(&self) -> &'static terra::Provider {
        &terra::Provider::AWS
    }
    fn validate_definition(&self) -> Result<(), terra::ResourceValidationError> {
        if self.arn.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "arn",
            });
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
pub struct AwsEcrRepositoryDetails {
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
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_delete: Option<bool>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_tag_mutability: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub name: SmolStr,
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
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry_id: Option<SmolStr>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository_url: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Map<SmolStr, SmolStr>>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags_all: Option<Map<SmolStr, SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption_configuration: Option<
        Vec<AwsEcrRepositoryResourceBlockTypeEncryptionConfiguration>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_scanning_configuration: Option<
        Vec<AwsEcrRepositoryResourceBlockTypeImageScanningConfiguration>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_tag_mutability_exclusion_filter: Option<
        Vec<AwsEcrRepositoryResourceBlockTypeImageTagMutabilityExclusionFilter>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeouts: Option<Vec<AwsEcrRepositoryResourceBlockTypeTimeouts>>,
}
impl terra::ToJson for AwsEcrRepositoryDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl terra::Resource for AwsEcrRepositoryDetails {
    fn resource_type(&self) -> &'static str {
        "aws_ecr_repository"
    }
    fn provider(&self) -> &'static terra::Provider {
        &terra::Provider::AWS
    }
    fn validate_definition(&self) -> Result<(), terra::ResourceValidationError> {
        if self.arn.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "arn",
            });
        }
        if self.name.is_empty() {
            return Err(terra::ResourceValidationError::MissingRequiredField {
                resource_type: self.resource_type(),
                field_name: "name",
            });
        }
        if self.registry_id.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "registry_id",
            });
        }
        if self.repository_url.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "repository_url",
            });
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
pub struct AwsEcsClusterDetails {
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
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub name: SmolStr,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_connect_defaults: Option<
        Vec<AwsEcsClusterResourceBlockTypeServiceConnectDefaults>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setting: Option<Vec<AwsEcsClusterResourceBlockTypeSetting>>,
}
impl terra::ToJson for AwsEcsClusterDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl terra::Resource for AwsEcsClusterDetails {
    fn resource_type(&self) -> &'static str {
        "aws_ecs_cluster"
    }
    fn provider(&self) -> &'static terra::Provider {
        &terra::Provider::AWS
    }
    fn validate_definition(&self) -> Result<(), terra::ResourceValidationError> {
        if self.arn.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "arn",
            });
        }
        if self.name.is_empty() {
            return Err(terra::ResourceValidationError::MissingRequiredField {
                resource_type: self.resource_type(),
                field_name: "name",
            });
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
pub struct AwsEcsServiceDetails {
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arn: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub availability_zone_rebalancing: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cluster: Option<SmolStr>,
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
    pub deployment_maximum_percent: Option<i64>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_minimum_healthy_percent: Option<i64>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desired_count: Option<i64>,
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
    pub for_each: Option<Vec<SmolStr>>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_delete: Option<bool>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_new_deployment: Option<bool>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_check_grace_period_seconds: Option<i64>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub launch_type: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub name: SmolStr,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform_version: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub propagate_tags: Option<SmolStr>,
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
    pub scheduling_strategy: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sigint_rollback: Option<bool>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Map<SmolStr, SmolStr>>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags_all: Option<Map<SmolStr, SmolStr>>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_definition: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triggers: Option<Map<SmolStr, SmolStr>>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wait_for_steady_state: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alarms: Option<Vec<AwsEcsServiceResourceBlockTypeAlarms>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity_provider_strategy: Option<
        Vec<AwsEcsServiceResourceBlockTypeCapacityProviderStrategy>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_circuit_breaker: Option<
        Vec<AwsEcsServiceResourceBlockTypeDeploymentCircuitBreaker>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_configuration: Option<
        Vec<AwsEcsServiceResourceBlockTypeDeploymentConfiguration>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_controller: Option<
        Vec<AwsEcsServiceResourceBlockTypeDeploymentController>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_balancer: Option<Vec<AwsEcsServiceResourceBlockTypeLoadBalancer>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_configuration: Option<
        Vec<AwsEcsServiceResourceBlockTypeNetworkConfiguration>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ordered_placement_strategy: Option<
        Vec<AwsEcsServiceResourceBlockTypeOrderedPlacementStrategy>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placement_constraints: Option<
        Vec<AwsEcsServiceResourceBlockTypePlacementConstraints>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_connect_configuration: Option<
        Vec<AwsEcsServiceResourceBlockTypeServiceConnectConfiguration>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_registries: Option<Vec<AwsEcsServiceResourceBlockTypeServiceRegistries>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeouts: Option<Vec<AwsEcsServiceResourceBlockTypeTimeouts>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_configuration: Option<
        Vec<AwsEcsServiceResourceBlockTypeVolumeConfiguration>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vpc_lattice_configurations: Option<
        Vec<AwsEcsServiceResourceBlockTypeVpcLatticeConfigurations>,
    >,
}
impl terra::ToJson for AwsEcsServiceDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl terra::Resource for AwsEcsServiceDetails {
    fn resource_type(&self) -> &'static str {
        "aws_ecs_service"
    }
    fn provider(&self) -> &'static terra::Provider {
        &terra::Provider::AWS
    }
    fn validate_definition(&self) -> Result<(), terra::ResourceValidationError> {
        if self.arn.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "arn",
            });
        }
        if self.name.is_empty() {
            return Err(terra::ResourceValidationError::MissingRequiredField {
                resource_type: self.resource_type(),
                field_name: "name",
            });
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
pub struct AwsEcsTaskDefinitionDetails {
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arn: Option<SmolStr>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arn_without_revision: Option<SmolStr>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub container_definitions: SmolStr,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<SmolStr>>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_fault_injection: Option<bool>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_role_arn: Option<SmolStr>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub family: SmolStr,
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
    pub ipc_mode: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_mode: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid_mode: Option<SmolStr>,
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
    pub requires_compatibilities: Option<Vec<SmolStr>>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<i64>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_destroy: Option<bool>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Map<SmolStr, SmolStr>>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags_all: Option<Map<SmolStr, SmolStr>>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_role_arn: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_latest: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_storage: Option<
        Vec<AwsEcsTaskDefinitionResourceBlockTypeEphemeralStorage>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placement_constraints: Option<
        Vec<AwsEcsTaskDefinitionResourceBlockTypePlacementConstraints>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_configuration: Option<
        Vec<AwsEcsTaskDefinitionResourceBlockTypeProxyConfiguration>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_platform: Option<
        Vec<AwsEcsTaskDefinitionResourceBlockTypeRuntimePlatform>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<Vec<AwsEcsTaskDefinitionResourceBlockTypeVolume>>,
}
impl terra::ToJson for AwsEcsTaskDefinitionDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl terra::Resource for AwsEcsTaskDefinitionDetails {
    fn resource_type(&self) -> &'static str {
        "aws_ecs_task_definition"
    }
    fn provider(&self) -> &'static terra::Provider {
        &terra::Provider::AWS
    }
    fn validate_definition(&self) -> Result<(), terra::ResourceValidationError> {
        if self.arn.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "arn",
            });
        }
        if self.arn_without_revision.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "arn_without_revision",
            });
        }
        if self.container_definitions.is_empty() {
            return Err(terra::ResourceValidationError::MissingRequiredField {
                resource_type: self.resource_type(),
                field_name: "container_definitions",
            });
        }
        if self.family.is_empty() {
            return Err(terra::ResourceValidationError::MissingRequiredField {
                resource_type: self.resource_type(),
                field_name: "family",
            });
        }
        if self.revision.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "revision",
            });
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
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
    fn resource_type(&self) -> &'static str {
        "aws_internet_gateway"
    }
    fn provider(&self) -> &'static terra::Provider {
        &terra::Provider::AWS
    }
    fn validate_definition(&self) -> Result<(), terra::ResourceValidationError> {
        if self.arn.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "arn",
            });
        }
        if self.owner_id.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "owner_id",
            });
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
pub struct AwsLbDetails {
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arn: Option<SmolStr>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arn_suffix: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_keep_alive: Option<i64>,
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
    pub desync_mitigation_mode: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns_name: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns_record_client_routing_policy: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drop_invalid_header_fields: Option<bool>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_cross_zone_load_balancing: Option<bool>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_deletion_protection: Option<bool>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_http2: Option<bool>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_tls_version_and_cipher_suite_headers: Option<bool>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_waf_fail_open: Option<bool>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_xff_client_port: Option<bool>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_zonal_shift: Option<bool>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enforce_security_group_inbound_rules_on_private_link_traffic: Option<SmolStr>,
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
    pub idle_timeout: Option<i64>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal: Option<bool>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address_type: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_balancer_type: Option<SmolStr>,
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
    pub preserve_host_header: Option<bool>,
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
    pub secondary_ips_auto_assigned_per_subnet: Option<i64>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_groups: Option<Vec<SmolStr>>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subnets: Option<Vec<SmolStr>>,
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
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xff_header_processing_mode: Option<SmolStr>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zone_id: Option<SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_logs: Option<Vec<AwsLbResourceBlockTypeAccessLogs>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_logs: Option<Vec<AwsLbResourceBlockTypeConnectionLogs>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_check_logs: Option<Vec<AwsLbResourceBlockTypeHealthCheckLogs>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipam_pools: Option<Vec<AwsLbResourceBlockTypeIpamPools>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_load_balancer_capacity: Option<
        Vec<AwsLbResourceBlockTypeMinimumLoadBalancerCapacity>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subnet_mapping: Option<Vec<AwsLbResourceBlockTypeSubnetMapping>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeouts: Option<Vec<AwsLbResourceBlockTypeTimeouts>>,
}
impl terra::ToJson for AwsLbDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl terra::Resource for AwsLbDetails {
    fn resource_type(&self) -> &'static str {
        "aws_lb"
    }
    fn provider(&self) -> &'static terra::Provider {
        &terra::Provider::AWS
    }
    fn validate_definition(&self) -> Result<(), terra::ResourceValidationError> {
        if self.arn.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "arn",
            });
        }
        if self.arn_suffix.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "arn_suffix",
            });
        }
        if self.dns_name.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "dns_name",
            });
        }
        if self.vpc_id.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "vpc_id",
            });
        }
        if self.zone_id.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "zone_id",
            });
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
pub struct AwsLbListenerDetails {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alpn_policy: Option<SmolStr>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arn: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub certificate_arn: Option<SmolStr>,
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
    pub load_balancer_arn: SmolStr,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<i64>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<SmolStr>,
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
    pub routing_http_request_x_amzn_mtls_clientcert_header_name: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing_http_request_x_amzn_mtls_clientcert_issuer_header_name: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing_http_request_x_amzn_mtls_clientcert_leaf_header_name: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing_http_request_x_amzn_mtls_clientcert_serial_number_header_name: Option<
        SmolStr,
    >,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing_http_request_x_amzn_mtls_clientcert_subject_header_name: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing_http_request_x_amzn_mtls_clientcert_validity_header_name: Option<
        SmolStr,
    >,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing_http_request_x_amzn_tls_cipher_suite_header_name: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing_http_request_x_amzn_tls_version_header_name: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing_http_response_access_control_allow_credentials_header_value: Option<
        SmolStr,
    >,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing_http_response_access_control_allow_headers_header_value: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing_http_response_access_control_allow_methods_header_value: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing_http_response_access_control_allow_origin_header_value: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing_http_response_access_control_expose_headers_header_value: Option<
        SmolStr,
    >,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing_http_response_access_control_max_age_header_value: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing_http_response_content_security_policy_header_value: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing_http_response_server_enabled: Option<bool>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing_http_response_strict_transport_security_header_value: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing_http_response_x_content_type_options_header_value: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing_http_response_x_frame_options_header_value: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_policy: Option<SmolStr>,
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
    pub tcp_idle_timeout_seconds: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_action: Option<Vec<AwsLbListenerResourceBlockTypeDefaultAction>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutual_authentication: Option<
        Vec<AwsLbListenerResourceBlockTypeMutualAuthentication>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeouts: Option<Vec<AwsLbListenerResourceBlockTypeTimeouts>>,
}
impl terra::ToJson for AwsLbListenerDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl terra::Resource for AwsLbListenerDetails {
    fn resource_type(&self) -> &'static str {
        "aws_lb_listener"
    }
    fn provider(&self) -> &'static terra::Provider {
        &terra::Provider::AWS
    }
    fn validate_definition(&self) -> Result<(), terra::ResourceValidationError> {
        if self.arn.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "arn",
            });
        }
        if self.load_balancer_arn.is_empty() {
            return Err(terra::ResourceValidationError::MissingRequiredField {
                resource_type: self.resource_type(),
                field_name: "load_balancer_arn",
            });
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
pub struct AwsLbTargetGroupDetails {
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arn: Option<SmolStr>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arn_suffix: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_termination: Option<bool>,
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
    pub deregistration_delay: Option<SmolStr>,
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
    pub ip_address_type: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lambda_multi_value_headers_enabled: Option<bool>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_balancer_arns: Option<Vec<SmolStr>>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_balancing_algorithm_type: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_balancing_anomaly_mitigation: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_balancing_cross_zone_enabled: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_prefix: Option<SmolStr>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<i64>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preserve_client_ip: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol_version: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_protocol_v2: Option<bool>,
    /// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slow_start: Option<i64>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Map<SmolStr, SmolStr>>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags_all: Option<Map<SmolStr, SmolStr>>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_control_port: Option<i64>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_type: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vpc_id: Option<SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_check: Option<Vec<AwsLbTargetGroupResourceBlockTypeHealthCheck>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stickiness: Option<Vec<AwsLbTargetGroupResourceBlockTypeStickiness>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_failover: Option<Vec<AwsLbTargetGroupResourceBlockTypeTargetFailover>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_health_state: Option<
        Vec<AwsLbTargetGroupResourceBlockTypeTargetHealthState>,
    >,
}
impl terra::ToJson for AwsLbTargetGroupDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl terra::Resource for AwsLbTargetGroupDetails {
    fn resource_type(&self) -> &'static str {
        "aws_lb_target_group"
    }
    fn provider(&self) -> &'static terra::Provider {
        &terra::Provider::AWS
    }
    fn validate_definition(&self) -> Result<(), terra::ResourceValidationError> {
        if self.arn.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "arn",
            });
        }
        if self.arn_suffix.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "arn_suffix",
            });
        }
        if self.load_balancer_arns.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "load_balancer_arns",
            });
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
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
    fn resource_type(&self) -> &'static str {
        "aws_route"
    }
    fn provider(&self) -> &'static terra::Provider {
        &terra::Provider::AWS
    }
    fn validate_definition(&self) -> Result<(), terra::ResourceValidationError> {
        if self.instance_id.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "instance_id",
            });
        }
        if self.instance_owner_id.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "instance_owner_id",
            });
        }
        if self.origin.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "origin",
            });
        }
        if self.route_table_id.is_empty() {
            return Err(terra::ResourceValidationError::MissingRequiredField {
                resource_type: self.resource_type(),
                field_name: "route_table_id",
            });
        }
        if self.state.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "state",
            });
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
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
    /// `required`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subnet_id: Option<SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeouts: Option<Vec<AwsRouteTableAssociationResourceBlockTypeTimeouts>>,
}
impl terra::ToJson for AwsRouteTableAssociationDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl terra::Resource for AwsRouteTableAssociationDetails {
    fn resource_type(&self) -> &'static str {
        "aws_route_table_association"
    }
    fn provider(&self) -> &'static terra::Provider {
        &terra::Provider::AWS
    }
    fn validate_definition(&self) -> Result<(), terra::ResourceValidationError> {
        if self.route_table_id.is_empty() {
            return Err(terra::ResourceValidationError::MissingRequiredField {
                resource_type: self.resource_type(),
                field_name: "route_table_id",
            });
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
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
    pub route: Option<Vec<SmolStr>>,
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
    fn resource_type(&self) -> &'static str {
        "aws_route_table"
    }
    fn provider(&self) -> &'static terra::Provider {
        &terra::Provider::AWS
    }
    fn validate_definition(&self) -> Result<(), terra::ResourceValidationError> {
        if self.arn.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "arn",
            });
        }
        if self.owner_id.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "owner_id",
            });
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
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
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
    pub egress: Option<Vec<SmolStr>>,
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
    pub ingress: Option<Vec<SmolStr>>,
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
    fn resource_type(&self) -> &'static str {
        "aws_security_group"
    }
    fn provider(&self) -> &'static terra::Provider {
        &terra::Provider::AWS
    }
    fn validate_definition(&self) -> Result<(), terra::ResourceValidationError> {
        if self.arn.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "arn",
            });
        }
        if self.owner_id.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "owner_id",
            });
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
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
    /// `optional`, `computed`
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
    fn resource_type(&self) -> &'static str {
        "aws_security_group_rule"
    }
    fn provider(&self) -> &'static terra::Provider {
        &terra::Provider::AWS
    }
    fn validate_definition(&self) -> Result<(), terra::ResourceValidationError> {
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
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "security_group_rule_id",
            });
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
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
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
    /// `required`
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
    /// `optional`, `computed`
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
    fn resource_type(&self) -> &'static str {
        "aws_subnet"
    }
    fn provider(&self) -> &'static terra::Provider {
        &terra::Provider::AWS
    }
    fn validate_definition(&self) -> Result<(), terra::ResourceValidationError> {
        if self.arn.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "arn",
            });
        }
        if self.ipv6_cidr_block_association_id.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "ipv6_cidr_block_association_id",
            });
        }
        if self.owner_id.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "owner_id",
            });
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
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
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
    /// `required`
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
    fn resource_type(&self) -> &'static str {
        "aws_vpc"
    }
    fn provider(&self) -> &'static terra::Provider {
        &terra::Provider::AWS
    }
    fn validate_definition(&self) -> Result<(), terra::ResourceValidationError> {
        if self.arn.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "arn",
            });
        }
        if self.default_network_acl_id.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "default_network_acl_id",
            });
        }
        if self.default_route_table_id.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "default_route_table_id",
            });
        }
        if self.default_security_group_id.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "default_security_group_id",
            });
        }
        if self.dhcp_options_id.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "dhcp_options_id",
            });
        }
        if self.ipv6_association_id.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "ipv6_association_id",
            });
        }
        if self.main_route_table_id.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "main_route_table_id",
            });
        }
        if self.owner_id.is_some() {
            return Err(terra::ResourceValidationError::NonEmptyComputedField {
                resource_type: self.resource_type(),
                field_name: "owner_id",
            });
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "encryption_configuration")]
pub struct AwsEcrRepositoryResourceBlockTypeEncryptionConfiguration {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption_type: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kms_key: Option<SmolStr>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "image_scanning_configuration")]
pub struct AwsEcrRepositoryResourceBlockTypeImageScanningConfiguration {
    /// ## Attribute
    /// `required`
    pub scan_on_push: bool,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "image_tag_mutability_exclusion_filter")]
pub struct AwsEcrRepositoryResourceBlockTypeImageTagMutabilityExclusionFilter {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub filter: SmolStr,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub filter_type: SmolStr,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "timeouts")]
pub struct AwsEcrRepositoryResourceBlockTypeTimeouts {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete: Option<SmolStr>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "service_connect_defaults")]
pub struct AwsEcsClusterResourceBlockTypeServiceConnectDefaults {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub namespace: SmolStr,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "setting")]
pub struct AwsEcsClusterResourceBlockTypeSetting {
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub name: SmolStr,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub value: SmolStr,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "alarms")]
pub struct AwsEcsServiceResourceBlockTypeAlarms {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub alarm_names: Vec<SmolStr>,
    /// ## Attribute
    /// `required`
    pub enable: bool,
    /// ## Attribute
    /// `required`
    pub rollback: bool,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "capacity_provider_strategy")]
pub struct AwsEcsServiceResourceBlockTypeCapacityProviderStrategy {
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
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "deployment_circuit_breaker")]
pub struct AwsEcsServiceResourceBlockTypeDeploymentCircuitBreaker {
    /// ## Attribute
    /// `required`
    pub enable: bool,
    /// ## Attribute
    /// `required`
    pub rollback: bool,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "deployment_configuration")]
pub struct AwsEcsServiceResourceBlockTypeDeploymentConfiguration {
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bake_time_in_minutes: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canary_configuration: Option<
        Vec<DeploymentConfigurationResourceBlockTypeCanaryConfiguration>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lifecycle_hook: Option<
        Vec<DeploymentConfigurationResourceBlockTypeLifecycleHook>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linear_configuration: Option<
        Vec<DeploymentConfigurationResourceBlockTypeLinearConfiguration>,
    >,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "deployment_controller")]
pub struct AwsEcsServiceResourceBlockTypeDeploymentController {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<SmolStr>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "load_balancer")]
pub struct AwsEcsServiceResourceBlockTypeLoadBalancer {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub container_name: SmolStr,
    /// ## Attribute
    /// `required`
    pub container_port: i64,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elb_name: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_group_arn: Option<SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advanced_configuration: Option<
        Vec<LoadBalancerResourceBlockTypeAdvancedConfiguration>,
    >,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "network_configuration")]
pub struct AwsEcsServiceResourceBlockTypeNetworkConfiguration {
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
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "ordered_placement_strategy")]
pub struct AwsEcsServiceResourceBlockTypeOrderedPlacementStrategy {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<SmolStr>,
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub r#type: SmolStr,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "placement_constraints")]
pub struct AwsEcsServiceResourceBlockTypePlacementConstraints {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expression: Option<SmolStr>,
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub r#type: SmolStr,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "service_connect_configuration")]
pub struct AwsEcsServiceResourceBlockTypeServiceConnectConfiguration {
    /// ## Attribute
    /// `required`
    pub enabled: bool,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_log_configuration: Option<
        Vec<ServiceConnectConfigurationResourceBlockTypeAccessLogConfiguration>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_configuration: Option<
        Vec<ServiceConnectConfigurationResourceBlockTypeLogConfiguration>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<Vec<ServiceConnectConfigurationResourceBlockTypeService>>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "service_registries")]
pub struct AwsEcsServiceResourceBlockTypeServiceRegistries {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_name: Option<SmolStr>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_port: Option<i64>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<i64>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub registry_arn: SmolStr,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "timeouts")]
pub struct AwsEcsServiceResourceBlockTypeTimeouts {
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
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "volume_configuration")]
pub struct AwsEcsServiceResourceBlockTypeVolumeConfiguration {
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub name: SmolStr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub managed_ebs_volume: Option<
        Vec<VolumeConfigurationResourceBlockTypeManagedEbsVolume>,
    >,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "vpc_lattice_configurations")]
pub struct AwsEcsServiceResourceBlockTypeVpcLatticeConfigurations {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub port_name: SmolStr,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub role_arn: SmolStr,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub target_group_arn: SmolStr,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "ephemeral_storage")]
pub struct AwsEcsTaskDefinitionResourceBlockTypeEphemeralStorage {
    /// ## Attribute
    /// `required`
    pub size_in_gib: i64,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "placement_constraints")]
pub struct AwsEcsTaskDefinitionResourceBlockTypePlacementConstraints {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expression: Option<SmolStr>,
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub r#type: SmolStr,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "proxy_configuration")]
pub struct AwsEcsTaskDefinitionResourceBlockTypeProxyConfiguration {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub container_name: SmolStr,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<Map<SmolStr, SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<SmolStr>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "runtime_platform")]
pub struct AwsEcsTaskDefinitionResourceBlockTypeRuntimePlatform {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_architecture: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operating_system_family: Option<SmolStr>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "volume")]
pub struct AwsEcsTaskDefinitionResourceBlockTypeVolume {
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configure_at_launch: Option<bool>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_path: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub name: SmolStr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docker_volume_configuration: Option<
        Vec<VolumeResourceBlockTypeDockerVolumeConfiguration>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub efs_volume_configuration: Option<
        Vec<VolumeResourceBlockTypeEfsVolumeConfiguration>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fsx_windows_file_server_volume_configuration: Option<
        Vec<VolumeResourceBlockTypeFsxWindowsFileServerVolumeConfiguration>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s3files_volume_configuration: Option<
        Vec<VolumeResourceBlockTypeS3filesVolumeConfiguration>,
    >,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
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
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "default_action")]
pub struct AwsLbListenerResourceBlockTypeDefaultAction {
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<i64>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_group_arn: Option<SmolStr>,
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub r#type: SmolStr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authenticate_cognito: Option<
        Vec<DefaultActionResourceBlockTypeAuthenticateCognito>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authenticate_oidc: Option<Vec<DefaultActionResourceBlockTypeAuthenticateOidc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixed_response: Option<Vec<DefaultActionResourceBlockTypeFixedResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jwt_validation: Option<Vec<DefaultActionResourceBlockTypeJwtValidation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect: Option<Vec<DefaultActionResourceBlockTypeRedirect>>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "mutual_authentication")]
pub struct AwsLbListenerResourceBlockTypeMutualAuthentication {
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advertise_trust_store_ca_names: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_client_certificate_expiry: Option<bool>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub mode: SmolStr,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trust_store_arn: Option<SmolStr>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "timeouts")]
pub struct AwsLbListenerResourceBlockTypeTimeouts {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update: Option<SmolStr>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "access_logs")]
pub struct AwsLbResourceBlockTypeAccessLogs {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub bucket: SmolStr,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<SmolStr>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "connection_logs")]
pub struct AwsLbResourceBlockTypeConnectionLogs {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub bucket: SmolStr,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<SmolStr>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "health_check_logs")]
pub struct AwsLbResourceBlockTypeHealthCheckLogs {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub bucket: SmolStr,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<SmolStr>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "ipam_pools")]
pub struct AwsLbResourceBlockTypeIpamPools {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub ipv4_ipam_pool_id: SmolStr,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "minimum_load_balancer_capacity")]
pub struct AwsLbResourceBlockTypeMinimumLoadBalancerCapacity {
    /// ## Attribute
    /// `required`
    pub capacity_units: i64,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "subnet_mapping")]
pub struct AwsLbResourceBlockTypeSubnetMapping {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allocation_id: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipv6_address: Option<SmolStr>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outpost_id: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_ipv4_address: Option<SmolStr>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub subnet_id: SmolStr,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "timeouts")]
pub struct AwsLbResourceBlockTypeTimeouts {
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
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "health_check")]
pub struct AwsLbTargetGroupResourceBlockTypeHealthCheck {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub healthy_threshold: Option<i64>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<i64>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matcher: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<SmolStr>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<i64>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unhealthy_threshold: Option<i64>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "stickiness")]
pub struct AwsLbTargetGroupResourceBlockTypeStickiness {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cookie_duration: Option<i64>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cookie_name: Option<SmolStr>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub r#type: SmolStr,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "target_failover")]
pub struct AwsLbTargetGroupResourceBlockTypeTargetFailover {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub on_deregistration: SmolStr,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub on_unhealthy: SmolStr,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "target_health_state")]
pub struct AwsLbTargetGroupResourceBlockTypeTargetHealthState {
    /// ## Attribute
    /// `required`
    pub enable_unhealthy_connection_termination: bool,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unhealthy_draining_interval: Option<i64>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
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
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
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
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
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
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
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
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "timeouts")]
pub struct AwsSecurityGroupRuleResourceBlockTypeTimeouts {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create: Option<SmolStr>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
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
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "authenticate_cognito")]
pub struct DefaultActionResourceBlockTypeAuthenticateCognito {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication_request_extra_params: Option<Map<SmolStr, SmolStr>>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_unauthenticated_request: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_cookie_name: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_timeout: Option<i64>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub user_pool_arn: SmolStr,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub user_pool_client_id: SmolStr,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub user_pool_domain: SmolStr,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "authenticate_oidc")]
pub struct DefaultActionResourceBlockTypeAuthenticateOidc {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication_request_extra_params: Option<Map<SmolStr, SmolStr>>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub authorization_endpoint: SmolStr,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub client_id: SmolStr,
    /// ## Attribute
    /// `required`, `sensitive`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub client_secret: SmolStr,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub issuer: SmolStr,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_unauthenticated_request: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_cookie_name: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_timeout: Option<i64>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub token_endpoint: SmolStr,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub user_info_endpoint: SmolStr,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "fixed_response")]
pub struct DefaultActionResourceBlockTypeFixedResponse {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub content_type: SmolStr,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_body: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<SmolStr>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "jwt_validation")]
pub struct DefaultActionResourceBlockTypeJwtValidation {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub issuer: SmolStr,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub jwks_endpoint: SmolStr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_claim: Option<Vec<JwtValidationResourceBlockTypeAdditionalClaim>>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "redirect")]
pub struct DefaultActionResourceBlockTypeRedirect {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<SmolStr>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub status_code: SmolStr,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "canary_configuration")]
pub struct DeploymentConfigurationResourceBlockTypeCanaryConfiguration {
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canary_bake_time_in_minutes: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canary_percent: Option<i64>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "lifecycle_hook")]
pub struct DeploymentConfigurationResourceBlockTypeLifecycleHook {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hook_details: Option<SmolStr>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub hook_target_arn: SmolStr,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub lifecycle_stages: Vec<SmolStr>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub role_arn: SmolStr,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "linear_configuration")]
pub struct DeploymentConfigurationResourceBlockTypeLinearConfiguration {
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_bake_time_in_minutes: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_percent: Option<i64>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "authorization_config")]
pub struct EfsVolumeConfigurationResourceBlockTypeAuthorizationConfig {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_point_id: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iam: Option<SmolStr>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "authorization_config")]
pub struct FsxWindowsFileServerVolumeConfigurationResourceBlockTypeAuthorizationConfig {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub credentials_parameter: SmolStr,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub domain: SmolStr,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "additional_claim")]
pub struct JwtValidationResourceBlockTypeAdditionalClaim {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub format: SmolStr,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub name: SmolStr,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<SmolStr>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "advanced_configuration")]
pub struct LoadBalancerResourceBlockTypeAdvancedConfiguration {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub alternate_target_group_arn: SmolStr,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub production_listener_rule: SmolStr,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub role_arn: SmolStr,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_listener_rule: Option<SmolStr>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "secret_option")]
pub struct LogConfigurationResourceBlockTypeSecretOption {
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub name: SmolStr,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub value_from: SmolStr,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "tag_specifications")]
pub struct ManagedEbsVolumeResourceBlockTypeTagSpecifications {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub propagate_tags: Option<SmolStr>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub resource_type: SmolStr,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Map<SmolStr, SmolStr>>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "access_log_configuration")]
pub struct ServiceConnectConfigurationResourceBlockTypeAccessLogConfiguration {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub format: SmolStr,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_query_parameters: Option<SmolStr>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "log_configuration")]
pub struct ServiceConnectConfigurationResourceBlockTypeLogConfiguration {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub log_driver: SmolStr,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Map<SmolStr, SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_option: Option<Vec<LogConfigurationResourceBlockTypeSecretOption>>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "service")]
pub struct ServiceConnectConfigurationResourceBlockTypeService {
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discovery_name: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ingress_port_override: Option<i64>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub port_name: SmolStr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_alias: Option<Vec<ServiceResourceBlockTypeClientAlias>>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<Vec<ServiceResourceBlockTypeTimeout>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<Vec<ServiceResourceBlockTypeTls>>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "client_alias")]
pub struct ServiceResourceBlockTypeClientAlias {
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns_name: Option<SmolStr>,
    /// ## Attribute
    /// `required`
    pub port: i64,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "timeout")]
pub struct ServiceResourceBlockTypeTimeout {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_timeout_seconds: Option<i64>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_request_timeout_seconds: Option<i64>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "tls")]
pub struct ServiceResourceBlockTypeTls {
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kms_key: Option<SmolStr>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_arn: Option<SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuer_cert_authority: Option<Vec<TlsResourceBlockTypeIssuerCertAuthority>>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "issuer_cert_authority")]
pub struct TlsResourceBlockTypeIssuerCertAuthority {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub aws_pca_authority_arn: SmolStr,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "managed_ebs_volume")]
pub struct VolumeConfigurationResourceBlockTypeManagedEbsVolume {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted: Option<bool>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_system_type: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iops: Option<i64>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kms_key_id: Option<SmolStr>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub role_arn: SmolStr,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_in_gb: Option<i64>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_id: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub throughput: Option<i64>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_initialization_rate: Option<i64>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_type: Option<SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag_specifications: Option<
        Vec<ManagedEbsVolumeResourceBlockTypeTagSpecifications>,
    >,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "docker_volume_configuration")]
pub struct VolumeResourceBlockTypeDockerVolumeConfiguration {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autoprovision: Option<bool>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver_opts: Option<Map<SmolStr, SmolStr>>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Map<SmolStr, SmolStr>>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<SmolStr>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "efs_volume_configuration")]
pub struct VolumeResourceBlockTypeEfsVolumeConfiguration {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub file_system_id: SmolStr,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_directory: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transit_encryption: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transit_encryption_port: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization_config: Option<
        Vec<EfsVolumeConfigurationResourceBlockTypeAuthorizationConfig>,
    >,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "fsx_windows_file_server_volume_configuration")]
pub struct VolumeResourceBlockTypeFsxWindowsFileServerVolumeConfiguration {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub file_system_id: SmolStr,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub root_directory: SmolStr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization_config: Option<
        Vec<FsxWindowsFileServerVolumeConfigurationResourceBlockTypeAuthorizationConfig>,
    >,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "s3files_volume_configuration")]
pub struct VolumeResourceBlockTypeS3filesVolumeConfiguration {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_point_arn: Option<SmolStr>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub file_system_arn: SmolStr,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_directory: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transit_encryption_port: Option<i64>,
}
