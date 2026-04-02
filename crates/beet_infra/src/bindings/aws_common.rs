//! Auto-generated Terraform provider bindings — do not edit by hand.

#![allow(unused_imports, non_snake_case, non_camel_case_types, non_upper_case_globals)]
use std::collections::BTreeMap as Map;
use serde::{Serialize, Deserialize};
use serde_json;

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AwsIamRoleDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arn: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub assume_role_policy: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub for_each: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_detach_policies: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub managed_policy_arns: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_session_duration: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions_boundary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Map<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags_all: Option<Map<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline_policy: Option<Vec<AwsIamRoleResourceBlockTypeInlinePolicy>>,
}
impl AwsIamRoleDetails {
    pub fn new(assume_role_policy: String) -> Self {
        Self {
            arn: None,
            assume_role_policy,
            count: None,
            create_date: None,
            depends_on: None,
            description: None,
            for_each: None,
            force_detach_policies: None,
            id: None,
            managed_policy_arns: None,
            max_session_duration: None,
            name: None,
            name_prefix: None,
            path: None,
            permissions_boundary: None,
            provider: None,
            tags: None,
            tags_all: None,
            unique_id: None,
            inline_policy: None,
        }
    }
}
impl crate::prelude::TerraJson for AwsIamRoleDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl crate::prelude::TerraResource for AwsIamRoleDetails {
    fn resource_type(&self) -> &'static str {
        "aws_iam_role"
    }
    fn provider(&self) -> &'static crate::prelude::TerraProvider {
        &crate::prelude::TerraProvider::AWS
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AwsIamRolePolicyAttachmentDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub for_each: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub policy_arn: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub role: String,
}
impl AwsIamRolePolicyAttachmentDetails {
    pub fn new(policy_arn: String, role: String) -> Self {
        Self {
            count: None,
            depends_on: None,
            for_each: None,
            id: None,
            policy_arn,
            provider: None,
            role,
        }
    }
}
impl crate::prelude::TerraJson for AwsIamRolePolicyAttachmentDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl crate::prelude::TerraResource for AwsIamRolePolicyAttachmentDetails {
    fn resource_type(&self) -> &'static str {
        "aws_iam_role_policy_attachment"
    }
    fn provider(&self) -> &'static crate::prelude::TerraProvider {
        &crate::prelude::TerraProvider::AWS
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
pub struct AwsS3BucketDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acceleration_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acl: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket_domain_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket_namespace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket_prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket_region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket_regional_domain_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub for_each: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_destroy: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hosted_zone_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_lock_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_payer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Map<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags_all: Option<Map<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website_domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website_endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cors_rule: Option<Vec<AwsS3BucketResourceBlockTypeCorsRule>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grant: Option<Vec<AwsS3BucketResourceBlockTypeGrant>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lifecycle_rule: Option<Vec<AwsS3BucketResourceBlockTypeLifecycleRule>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<Vec<AwsS3BucketResourceBlockTypeLogging>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_lock_configuration: Option<
        Vec<AwsS3BucketResourceBlockTypeObjectLockConfiguration>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replication_configuration: Option<
        Vec<AwsS3BucketResourceBlockTypeReplicationConfiguration>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeouts: Option<Vec<AwsS3BucketResourceBlockTypeTimeouts>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub versioning: Option<Vec<AwsS3BucketResourceBlockTypeVersioning>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<Vec<AwsS3BucketResourceBlockTypeWebsite>>,
}
impl crate::prelude::TerraJson for AwsS3BucketDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl crate::prelude::TerraResource for AwsS3BucketDetails {
    fn resource_type(&self) -> &'static str {
        "aws_s3_bucket"
    }
    fn provider(&self) -> &'static crate::prelude::TerraProvider {
        &crate::prelude::TerraProvider::AWS
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "inline_policy")]
pub struct AwsIamRoleResourceBlockTypeInlinePolicy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<String>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename = "cors_rule")]
pub struct AwsS3BucketResourceBlockTypeCorsRule {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_headers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub allowed_methods: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub allowed_origins: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expose_headers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_age_seconds: Option<i64>,
}
impl AwsS3BucketResourceBlockTypeCorsRule {
    pub fn new(allowed_methods: Vec<String>, allowed_origins: Vec<String>) -> Self {
        Self {
            allowed_headers: None,
            allowed_methods,
            allowed_origins,
            expose_headers: None,
            max_age_seconds: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename = "grant")]
pub struct AwsS3BucketResourceBlockTypeGrant {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub permissions: Vec<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
}
impl AwsS3BucketResourceBlockTypeGrant {
    pub fn new(permissions: Vec<String>, r#type: String) -> Self {
        Self {
            id: None,
            permissions,
            r#type,
            uri: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename = "lifecycle_rule")]
pub struct AwsS3BucketResourceBlockTypeLifecycleRule {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abort_incomplete_multipart_upload_days: Option<i64>,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Map<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration: Option<Vec<LifecycleRuleResourceBlockTypeExpiration>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub noncurrent_version_expiration: Option<
        Vec<LifecycleRuleResourceBlockTypeNoncurrentVersionExpiration>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub noncurrent_version_transition: Option<
        Vec<LifecycleRuleResourceBlockTypeNoncurrentVersionTransition>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<Vec<LifecycleRuleResourceBlockTypeTransition>>,
}
impl AwsS3BucketResourceBlockTypeLifecycleRule {
    pub fn new(enabled: bool) -> Self {
        Self {
            abort_incomplete_multipart_upload_days: None,
            enabled,
            id: None,
            prefix: None,
            tags: None,
            expiration: None,
            noncurrent_version_expiration: None,
            noncurrent_version_transition: None,
            transition: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename = "logging")]
pub struct AwsS3BucketResourceBlockTypeLogging {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub target_bucket: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_prefix: Option<String>,
}
impl AwsS3BucketResourceBlockTypeLogging {
    pub fn new(target_bucket: String) -> Self {
        Self {
            target_bucket,
            target_prefix: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "object_lock_configuration")]
pub struct AwsS3BucketResourceBlockTypeObjectLockConfiguration {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_lock_enabled: Option<String>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename = "replication_configuration")]
pub struct AwsS3BucketResourceBlockTypeReplicationConfiguration {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<ReplicationConfigurationResourceBlockTypeRules>>,
}
impl AwsS3BucketResourceBlockTypeReplicationConfiguration {
    pub fn new(role: String) -> Self {
        Self { role, rules: None }
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "timeouts")]
pub struct AwsS3BucketResourceBlockTypeTimeouts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update: Option<String>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "versioning")]
pub struct AwsS3BucketResourceBlockTypeVersioning {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mfa_delete: Option<bool>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "website")]
pub struct AwsS3BucketResourceBlockTypeWebsite {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_document: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index_document: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_all_requests_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing_rules: Option<String>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename = "access_control_translation")]
pub struct DestinationResourceBlockTypeAccessControlTranslation {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub owner: String,
}
impl DestinationResourceBlockTypeAccessControlTranslation {
    pub fn new(owner: String) -> Self {
        Self { owner }
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "metrics")]
pub struct DestinationResourceBlockTypeMetrics {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minutes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "replication_time")]
pub struct DestinationResourceBlockTypeReplicationTime {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minutes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "expiration")]
pub struct LifecycleRuleResourceBlockTypeExpiration {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expired_object_delete_marker: Option<bool>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "noncurrent_version_expiration")]
pub struct LifecycleRuleResourceBlockTypeNoncurrentVersionExpiration {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days: Option<i64>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename = "noncurrent_version_transition")]
pub struct LifecycleRuleResourceBlockTypeNoncurrentVersionTransition {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days: Option<i64>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub storage_class: String,
}
impl LifecycleRuleResourceBlockTypeNoncurrentVersionTransition {
    pub fn new(storage_class: String) -> Self {
        Self { days: None, storage_class }
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename = "transition")]
pub struct LifecycleRuleResourceBlockTypeTransition {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days: Option<i64>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub storage_class: String,
}
impl LifecycleRuleResourceBlockTypeTransition {
    pub fn new(storage_class: String) -> Self {
        Self {
            date: None,
            days: None,
            storage_class,
        }
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename = "rules")]
pub struct ReplicationConfigurationResourceBlockTypeRules {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete_marker_replication_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i64>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<Vec<RulesResourceBlockTypeDestination>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<Vec<RulesResourceBlockTypeFilter>>,
}
impl ReplicationConfigurationResourceBlockTypeRules {
    pub fn new(status: String) -> Self {
        Self {
            delete_marker_replication_status: None,
            id: None,
            prefix: None,
            priority: None,
            status,
            destination: None,
            filter: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename = "destination")]
pub struct RulesResourceBlockTypeDestination {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub bucket: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replica_kms_key_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_control_translation: Option<
        Vec<DestinationResourceBlockTypeAccessControlTranslation>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<Vec<DestinationResourceBlockTypeMetrics>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replication_time: Option<Vec<DestinationResourceBlockTypeReplicationTime>>,
}
impl RulesResourceBlockTypeDestination {
    pub fn new(bucket: String) -> Self {
        Self {
            account_id: None,
            bucket,
            replica_kms_key_id: None,
            storage_class: None,
            access_control_translation: None,
            metrics: None,
            replication_time: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "filter")]
pub struct RulesResourceBlockTypeFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Map<String, String>>,
}
