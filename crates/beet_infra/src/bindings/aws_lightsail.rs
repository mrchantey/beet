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
pub struct AwsLightsailInstanceDetails {
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arn: Option<SmolStr>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub availability_zone: SmolStr,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub blueprint_id: SmolStr,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub bundle_id: SmolStr,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_count: Option<i64>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<SmolStr>,
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
    pub ip_address_type: Option<SmolStr>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipv6_addresses: Option<Vec<SmolStr>>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_static_ip: Option<bool>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_pair_name: Option<SmolStr>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub name: SmolStr,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_ip_address: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<SmolStr>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_ip_address: Option<SmolStr>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ram_size: Option<i64>,
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
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_data: Option<SmolStr>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add_on: Option<Vec<AwsLightsailInstanceResourceBlockTypeAddOn>>,
}
impl terra::ToJson for AwsLightsailInstanceDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl terra::Resource for AwsLightsailInstanceDetails {
    fn resource_type(&self) -> &'static str {
        "aws_lightsail_instance"
    }
    fn provider(&self) -> &'static terra::Provider {
        &terra::Provider::AWS
    }
    fn validate_definition(&self) -> Result {
        if self.arn.is_some() {
            bevybail!(
                "{}: computed-only field `arn` should not be set", self.resource_type()
            );
        }
        if self.availability_zone.is_empty() {
            bevybail!(
                "{}: required field `availability_zone` is empty", self.resource_type()
            );
        }
        if self.blueprint_id.is_empty() {
            bevybail!(
                "{}: required field `blueprint_id` is empty", self.resource_type()
            );
        }
        if self.bundle_id.is_empty() {
            bevybail!("{}: required field `bundle_id` is empty", self.resource_type());
        }
        if self.cpu_count.is_some() {
            bevybail!(
                "{}: computed-only field `cpu_count` should not be set", self
                .resource_type()
            );
        }
        if self.created_at.is_some() {
            bevybail!(
                "{}: computed-only field `created_at` should not be set", self
                .resource_type()
            );
        }
        if self.ipv6_addresses.is_some() {
            bevybail!(
                "{}: computed-only field `ipv6_addresses` should not be set", self
                .resource_type()
            );
        }
        if self.is_static_ip.is_some() {
            bevybail!(
                "{}: computed-only field `is_static_ip` should not be set", self
                .resource_type()
            );
        }
        if self.name.is_empty() {
            bevybail!("{}: required field `name` is empty", self.resource_type());
        }
        if self.private_ip_address.is_some() {
            bevybail!(
                "{}: computed-only field `private_ip_address` should not be set", self
                .resource_type()
            );
        }
        if self.public_ip_address.is_some() {
            bevybail!(
                "{}: computed-only field `public_ip_address` should not be set", self
                .resource_type()
            );
        }
        if self.ram_size.is_some() {
            bevybail!(
                "{}: computed-only field `ram_size` should not be set", self
                .resource_type()
            );
        }
        if self.username.is_some() {
            bevybail!(
                "{}: computed-only field `username` should not be set", self
                .resource_type()
            );
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
pub struct AwsLightsailInstancePublicPortsDetails {
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
    pub instance_name: SmolStr,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<SmolStr>,
    /// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port_info: Option<Vec<AwsLightsailInstancePublicPortsResourceBlockTypePortInfo>>,
}
impl terra::ToJson for AwsLightsailInstancePublicPortsDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl terra::Resource for AwsLightsailInstancePublicPortsDetails {
    fn resource_type(&self) -> &'static str {
        "aws_lightsail_instance_public_ports"
    }
    fn provider(&self) -> &'static terra::Provider {
        &terra::Provider::AWS
    }
    fn validate_definition(&self) -> Result {
        if self.instance_name.is_empty() {
            bevybail!(
                "{}: required field `instance_name` is empty", self.resource_type()
            );
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
pub struct AwsLightsailKeyPairDetails {
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
    pub encrypted_fingerprint: Option<SmolStr>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted_private_key: Option<SmolStr>,
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
    /// `required`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_prefix: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pgp_key: Option<SmolStr>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_key: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<SmolStr>,
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
impl terra::ToJson for AwsLightsailKeyPairDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl terra::Resource for AwsLightsailKeyPairDetails {
    fn resource_type(&self) -> &'static str {
        "aws_lightsail_key_pair"
    }
    fn provider(&self) -> &'static terra::Provider {
        &terra::Provider::AWS
    }
    fn validate_definition(&self) -> Result {
        if self.arn.is_some() {
            bevybail!(
                "{}: computed-only field `arn` should not be set", self.resource_type()
            );
        }
        if self.encrypted_fingerprint.is_some() {
            bevybail!(
                "{}: computed-only field `encrypted_fingerprint` should not be set", self
                .resource_type()
            );
        }
        if self.encrypted_private_key.is_some() {
            bevybail!(
                "{}: computed-only field `encrypted_private_key` should not be set", self
                .resource_type()
            );
        }
        if self.fingerprint.is_some() {
            bevybail!(
                "{}: computed-only field `fingerprint` should not be set", self
                .resource_type()
            );
        }
        if self.private_key.is_some() {
            bevybail!(
                "{}: computed-only field `private_key` should not be set", self
                .resource_type()
            );
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
pub struct AwsLightsailStaticIpAttachmentDetails {
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
    pub instance_name: SmolStr,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<SmolStr>,
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
    pub static_ip_name: SmolStr,
}
impl terra::ToJson for AwsLightsailStaticIpAttachmentDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl terra::Resource for AwsLightsailStaticIpAttachmentDetails {
    fn resource_type(&self) -> &'static str {
        "aws_lightsail_static_ip_attachment"
    }
    fn provider(&self) -> &'static terra::Provider {
        &terra::Provider::AWS
    }
    fn validate_definition(&self) -> Result {
        if self.instance_name.is_empty() {
            bevybail!(
                "{}: required field `instance_name` is empty", self.resource_type()
            );
        }
        if self.ip_address.is_some() {
            bevybail!(
                "{}: computed-only field `ip_address` should not be set", self
                .resource_type()
            );
        }
        if self.static_ip_name.is_empty() {
            bevybail!(
                "{}: required field `static_ip_name` is empty", self.resource_type()
            );
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
pub struct AwsLightsailStaticIpDetails {
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
    pub ip_address: Option<SmolStr>,
    /// ## Attribute
    /// `required`
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
    pub support_code: Option<SmolStr>,
}
impl terra::ToJson for AwsLightsailStaticIpDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl terra::Resource for AwsLightsailStaticIpDetails {
    fn resource_type(&self) -> &'static str {
        "aws_lightsail_static_ip"
    }
    fn provider(&self) -> &'static terra::Provider {
        &terra::Provider::AWS
    }
    fn validate_definition(&self) -> Result {
        if self.arn.is_some() {
            bevybail!(
                "{}: computed-only field `arn` should not be set", self.resource_type()
            );
        }
        if self.ip_address.is_some() {
            bevybail!(
                "{}: computed-only field `ip_address` should not be set", self
                .resource_type()
            );
        }
        if self.name.is_empty() {
            bevybail!("{}: required field `name` is empty", self.resource_type());
        }
        if self.support_code.is_some() {
            bevybail!(
                "{}: computed-only field `support_code` should not be set", self
                .resource_type()
            );
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "port_info")]
pub struct AwsLightsailInstancePublicPortsResourceBlockTypePortInfo {
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cidr_list_aliases: Option<Vec<SmolStr>>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cidrs: Option<Vec<SmolStr>>,
    /// ## Attribute
    /// `required`
    pub from_port: i64,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipv6_cidrs: Option<Vec<SmolStr>>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub protocol: SmolStr,
    /// ## Attribute
    /// `required`
    pub to_port: i64,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "add_on")]
pub struct AwsLightsailInstanceResourceBlockTypeAddOn {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub snapshot_time: SmolStr,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub status: SmolStr,
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub r#type: SmolStr,
}
