//! Auto-generated Terraform provider bindings — do not edit by hand.

#![allow(unused_imports, non_snake_case, non_camel_case_types, non_upper_case_globals)]
use std::collections::BTreeMap as Map;
use serde::{Serialize, Deserialize};
use serde_json;

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AwsLightsailInstanceDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arn: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub availability_zone: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub blueprint_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub bundle_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub for_each: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipv6_addresses: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_static_ip: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_pair_name: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_ip_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_ip_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ram_size: Option<i64>,
    /// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Map<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags_all: Option<Map<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add_on: Option<Vec<AwsLightsailInstanceResourceBlockTypeAddOn>>,
}
impl AwsLightsailInstanceDetails {
    pub fn new(
        availability_zone: String,
        blueprint_id: String,
        bundle_id: String,
        name: String,
    ) -> Self {
        Self {
            arn: None,
            availability_zone,
            blueprint_id,
            bundle_id,
            count: None,
            cpu_count: None,
            created_at: None,
            depends_on: None,
            for_each: None,
            id: None,
            ip_address_type: None,
            ipv6_addresses: None,
            is_static_ip: None,
            key_pair_name: None,
            name,
            private_ip_address: None,
            provider: None,
            public_ip_address: None,
            ram_size: None,
            region: None,
            tags: None,
            tags_all: None,
            user_data: None,
            username: None,
            add_on: None,
        }
    }
}
impl crate::prelude::TerraJson for AwsLightsailInstanceDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl crate::prelude::TerraResource for AwsLightsailInstanceDetails {
    fn resource_type(&self) -> &'static str {
        "aws_lightsail_instance"
    }
    fn provider(&self) -> &'static crate::prelude::TerraProvider {
        &crate::prelude::TerraProvider::AWS
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AwsLightsailInstancePublicPortsDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub for_each: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub instance_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port_info: Option<Vec<AwsLightsailInstancePublicPortsResourceBlockTypePortInfo>>,
}
impl AwsLightsailInstancePublicPortsDetails {
    pub fn new(instance_name: String) -> Self {
        Self {
            count: None,
            depends_on: None,
            for_each: None,
            id: None,
            instance_name,
            provider: None,
            region: None,
            port_info: None,
        }
    }
}
impl crate::prelude::TerraJson for AwsLightsailInstancePublicPortsDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl crate::prelude::TerraResource for AwsLightsailInstancePublicPortsDetails {
    fn resource_type(&self) -> &'static str {
        "aws_lightsail_instance_public_ports"
    }
    fn provider(&self) -> &'static crate::prelude::TerraProvider {
        &crate::prelude::TerraProvider::AWS
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
pub struct AwsLightsailKeyPairDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted_fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted_private_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub for_each: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pgp_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    /// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Map<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags_all: Option<Map<String, String>>,
}
impl crate::prelude::TerraJson for AwsLightsailKeyPairDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl crate::prelude::TerraResource for AwsLightsailKeyPairDetails {
    fn resource_type(&self) -> &'static str {
        "aws_lightsail_key_pair"
    }
    fn provider(&self) -> &'static crate::prelude::TerraProvider {
        &crate::prelude::TerraProvider::AWS
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AwsLightsailStaticIpAttachmentDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub for_each: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub instance_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub static_ip_name: String,
}
impl AwsLightsailStaticIpAttachmentDetails {
    pub fn new(instance_name: String, static_ip_name: String) -> Self {
        Self {
            count: None,
            depends_on: None,
            for_each: None,
            id: None,
            instance_name,
            ip_address: None,
            provider: None,
            region: None,
            static_ip_name,
        }
    }
}
impl crate::prelude::TerraJson for AwsLightsailStaticIpAttachmentDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl crate::prelude::TerraResource for AwsLightsailStaticIpAttachmentDetails {
    fn resource_type(&self) -> &'static str {
        "aws_lightsail_static_ip_attachment"
    }
    fn provider(&self) -> &'static crate::prelude::TerraProvider {
        &crate::prelude::TerraProvider::AWS
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AwsLightsailStaticIpDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub for_each: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub support_code: Option<String>,
}
impl AwsLightsailStaticIpDetails {
    pub fn new(name: String) -> Self {
        Self {
            arn: None,
            count: None,
            depends_on: None,
            for_each: None,
            id: None,
            ip_address: None,
            name,
            provider: None,
            region: None,
            support_code: None,
        }
    }
}
impl crate::prelude::TerraJson for AwsLightsailStaticIpDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl crate::prelude::TerraResource for AwsLightsailStaticIpDetails {
    fn resource_type(&self) -> &'static str {
        "aws_lightsail_static_ip"
    }
    fn provider(&self) -> &'static crate::prelude::TerraProvider {
        &crate::prelude::TerraProvider::AWS
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename = "port_info")]
pub struct AwsLightsailInstancePublicPortsResourceBlockTypePortInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cidr_list_aliases: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cidrs: Option<Vec<String>>,
    pub from_port: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipv6_cidrs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub protocol: String,
    pub to_port: i64,
}
impl AwsLightsailInstancePublicPortsResourceBlockTypePortInfo {
    pub fn new(from_port: i64, protocol: String, to_port: i64) -> Self {
        Self {
            cidr_list_aliases: None,
            cidrs: None,
            from_port,
            ipv6_cidrs: None,
            protocol,
            to_port,
        }
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename = "add_on")]
pub struct AwsLightsailInstanceResourceBlockTypeAddOn {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub snapshot_time: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub status: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub r#type: String,
}
impl AwsLightsailInstanceResourceBlockTypeAddOn {
    pub fn new(snapshot_time: String, status: String, r#type: String) -> Self {
        Self {
            snapshot_time,
            status,
            r#type,
        }
    }
}
