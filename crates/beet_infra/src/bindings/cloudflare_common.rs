//! Auto-generated Terraform provider bindings — do not edit by hand.

#![allow(unused_imports, non_snake_case, non_camel_case_types, non_upper_case_globals)]
use std::collections::BTreeMap as Map;
use serde::{Serialize, Deserialize};
use serde_json;
#[allow(unused)]
use beet_core::prelude::*;
#[allow(unused)]
use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct CloudflareDnsRecordDetails {
    /// Comments or notes about the DNS record. This field has no effect on DNS responses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<SmolStr>,
    /// When the record comment was last modified. Omitted if there is no comment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment_modified_on: Option<SmolStr>,
    /// A valid IPv4 address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    /// When the record was created.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_on: Option<SmolStr>,
    /// Components of a CAA record.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Map<SmolStr, SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub for_each: Option<Vec<SmolStr>>,
    /// Identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<SmolStr>,
    /// Extra Cloudflare-specific information about the record.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<SmolStr>,
    /// When the record was last modified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_on: Option<SmolStr>,
    /// DNS record name (or @ for the zone apex) in Punycode.
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub name: SmolStr,
    /// Required for MX, SRV and URI records; unused by other record types. Records with lower priorities are preferred.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<SmolStr>,
    /// Whether the record can be proxied by Cloudflare or not.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxiable: Option<bool>,
    /// Whether the record is receiving the performance and security benefits of Cloudflare.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxied: Option<bool>,
    /// Settings for the DNS record.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<Map<SmolStr, SmolStr>>,
    /// Custom tags for the DNS record. This field has no effect on DNS responses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<SmolStr>>,
    /// When the record tags were last modified. Omitted if there are no tags.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags_modified_on: Option<SmolStr>,
    /// Time To Live (TTL) of the DNS record in seconds. Setting to 1 means 'automatic'. Value must be between 60 and 86400, with the minimum reduced to 30 for Enterprise zones.
    pub ttl: i64,
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub r#type: SmolStr,
    /// Identifier.
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub zone_id: SmolStr,
}
impl CloudflareDnsRecordDetails {
    pub fn new(name: SmolStr, ttl: i64, r#type: SmolStr, zone_id: SmolStr) -> Self {
        Self {
            comment: None,
            comment_modified_on: None,
            content: None,
            count: None,
            created_on: None,
            data: None,
            depends_on: None,
            for_each: None,
            id: None,
            meta: None,
            modified_on: None,
            name,
            priority: None,
            provider: None,
            proxiable: None,
            proxied: None,
            settings: None,
            tags: None,
            tags_modified_on: None,
            ttl,
            r#type,
            zone_id,
        }
    }
}
impl TerraJson for CloudflareDnsRecordDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl TerraResource for CloudflareDnsRecordDetails {
    fn resource_type(&self) -> &'static str {
        "cloudflare_dns_record"
    }
    fn provider(&self) -> &'static TerraProvider {
        &TerraProvider::CLOUDFLARE
    }
}
