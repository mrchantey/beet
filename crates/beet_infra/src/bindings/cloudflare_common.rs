//! Auto-generated Terraform provider bindings — do not edit by hand.

#![allow(unused_imports, non_snake_case, non_camel_case_types, non_upper_case_globals)]
use std::collections::BTreeMap as Map;
use serde::{Serialize, Deserialize};
use serde_json;
#[allow(unused)]
use beet_core::prelude::*;
#[allow(unused)]
use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
pub struct CloudflareDnsRecordDetails {
    /// Comments or notes about the DNS record. This field has no effect on DNS responses.
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<SmolStr>,
    /// When the record comment was last modified. Omitted if there is no comment.
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment_modified_on: Option<SmolStr>,
    /// A valid IPv4 address.
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    /// When the record was created.
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_on: Option<SmolStr>,
    /// Components of a CAA record.
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Map<SmolStr, SmolStr>>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<SmolStr>>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub for_each: Option<Vec<SmolStr>>,
    /// Identifier.
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<SmolStr>,
    /// Extra Cloudflare-specific information about the record.
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<SmolStr>,
    /// When the record was last modified.
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_on: Option<SmolStr>,
    /// DNS record name (or @ for the zone apex) in Punycode.
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub name: SmolStr,
    /// Required for MX, SRV and URI records; unused by other record types. Records with lower priorities are preferred.
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i64>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<SmolStr>,
    /// Whether the record can be proxied by Cloudflare or not.
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxiable: Option<bool>,
    /// Whether the record is receiving the performance and security benefits of Cloudflare.
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxied: Option<bool>,
    /// Settings for the DNS record.
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<Map<SmolStr, SmolStr>>,
    /// Custom tags for the DNS record. This field has no effect on DNS responses.
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<SmolStr>>,
    /// When the record tags were last modified. Omitted if there are no tags.
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags_modified_on: Option<SmolStr>,
    /// Time To Live (TTL) of the DNS record in seconds. Setting to 1 means 'automatic'. Value must be between 60 and 86400, with the minimum reduced to 30 for Enterprise zones.
    /// ## Attribute
    /// `required`
    pub ttl: i64,
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub r#type: SmolStr,
    /// Identifier.
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub zone_id: SmolStr,
}
impl terra::ToJson for CloudflareDnsRecordDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl terra::Resource for CloudflareDnsRecordDetails {
    fn resource_type(&self) -> &'static str {
        "cloudflare_dns_record"
    }
    fn provider(&self) -> &'static terra::Provider {
        &terra::Provider::CLOUDFLARE
    }
    fn validate_definition(&self) -> Result {
        if self.comment_modified_on.is_some() {
            bevybail!(
                "{}: computed-only field `comment_modified_on` should not be set", self
                .resource_type()
            );
        }
        if self.created_on.is_some() {
            bevybail!(
                "{}: computed-only field `created_on` should not be set", self
                .resource_type()
            );
        }
        if self.id.is_some() {
            bevybail!(
                "{}: computed-only field `id` should not be set", self.resource_type()
            );
        }
        if self.meta.is_some() {
            bevybail!(
                "{}: computed-only field `meta` should not be set", self.resource_type()
            );
        }
        if self.modified_on.is_some() {
            bevybail!(
                "{}: computed-only field `modified_on` should not be set", self
                .resource_type()
            );
        }
        if self.name.is_empty() {
            bevybail!("{}: required field `name` is empty", self.resource_type());
        }
        if self.proxiable.is_some() {
            bevybail!(
                "{}: computed-only field `proxiable` should not be set", self
                .resource_type()
            );
        }
        if self.tags_modified_on.is_some() {
            bevybail!(
                "{}: computed-only field `tags_modified_on` should not be set", self
                .resource_type()
            );
        }
        if self.r#type.is_empty() {
            bevybail!("{}: required field `type` is empty", self.resource_type());
        }
        if self.zone_id.is_empty() {
            bevybail!("{}: required field `zone_id` is empty", self.resource_type());
        }
        Ok(())
    }
}
