//! Terraform provider bindings for AWS Route53 DNS.

#![allow(unused_imports, non_snake_case, non_camel_case_types, non_upper_case_globals)]
use std::collections::BTreeMap as Map;
use serde::{Serialize, Deserialize};
use serde_json;
#[allow(unused)]
use beet_core::prelude::*;
#[allow(unused)]
use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
pub struct AwsRoute53RecordDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_overwrite: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub for_each: Option<Vec<SmolStr>>,
    /// The FQDN built from the name and zone.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fqdn: Option<SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_check_id: Option<SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multivalue_answer_routing_policy: Option<bool>,
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub name: SmolStr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records: Option<Vec<SmolStr>>,
    /// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub set_identifier: Option<SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Map<SmolStr, SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags_all: Option<Map<SmolStr, SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<i64>,
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub r#type: SmolStr,
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub zone_id: SmolStr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<Vec<AwsRoute53RecordResourceBlockTypeAlias>>,
}
impl AwsRoute53RecordDetails {
    pub fn new(name: SmolStr, r#type: SmolStr, zone_id: SmolStr) -> Self {
        Self {
            allow_overwrite: None,
            count: None,
            depends_on: None,
            for_each: None,
            fqdn: None,
            health_check_id: None,
            id: None,
            multivalue_answer_routing_policy: None,
            name,
            provider: None,
            records: None,
            region: None,
            set_identifier: None,
            tags: None,
            tags_all: None,
            ttl: None,
            r#type,
            zone_id,
            alias: None,
        }
    }
}
impl terra::ToJson for AwsRoute53RecordDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl terra::Resource for AwsRoute53RecordDetails {
    fn resource_type(&self) -> &'static str {
        "aws_route53_record"
    }
    fn provider(&self) -> &'static terra::Provider {
        &terra::Provider::AWS
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "alias")]
pub struct AwsRoute53RecordResourceBlockTypeAlias {
    pub evaluate_target_health: bool,
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub name: SmolStr,
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub zone_id: SmolStr,
}
impl AwsRoute53RecordResourceBlockTypeAlias {
    pub fn new(
        evaluate_target_health: bool,
        name: SmolStr,
        zone_id: SmolStr,
    ) -> Self {
        Self {
            evaluate_target_health,
            name,
            zone_id,
        }
    }
}
