//! Auto-generated Terraform provider bindings — do not edit!
//! Auto-generated Terraform provider bindings — do not edit!
//! Auto-generated Terraform provider bindings — do not edit!
//! Generated from the hashicorp/aws v6.62.0 schema.

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
use std::collections::BTreeMap as Map;

#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct AwsSesv2ConfigurationSetDetails {
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub arn: Option<SmolStr>,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub configuration_set_name: SmolStr,
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
	pub delivery_options:
		Option<Vec<AwsSesv2ConfigurationSetResourceBlockTypeDeliveryOptions>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub reputation_options:
		Option<Vec<AwsSesv2ConfigurationSetResourceBlockTypeReputationOptions>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub sending_options:
		Option<Vec<AwsSesv2ConfigurationSetResourceBlockTypeSendingOptions>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub suppression_options: Option<
		Vec<AwsSesv2ConfigurationSetResourceBlockTypeSuppressionOptions>,
	>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tracking_options:
		Option<Vec<AwsSesv2ConfigurationSetResourceBlockTypeTrackingOptions>>,
}
impl terra::ToJson for AwsSesv2ConfigurationSetDetails {
	fn to_json(&self) -> Value {
		Value::from_serde(self).expect("serialization should not fail")
	}
}
impl terra::Resource for AwsSesv2ConfigurationSetDetails {
	fn resource_type(&self) -> &'static str { "aws_sesv2_configuration_set" }
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
		if self.configuration_set_name.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "configuration_set_name",
			});
		}
		Ok(())
	}
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct AwsSesv2ConfigurationSetEventDestinationDetails {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub configuration_set_name: SmolStr,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<SmolStr>>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub event_destination_name: SmolStr,
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
    pub provider: Option<SmolStr>,
    /// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_destination: Option<
        Vec<AwsSesv2ConfigurationSetEventDestinationResourceBlockTypeEventDestination>,
    >,
}
impl terra::ToJson for AwsSesv2ConfigurationSetEventDestinationDetails {
	fn to_json(&self) -> Value {
		Value::from_serde(self).expect("serialization should not fail")
	}
}
impl terra::Resource for AwsSesv2ConfigurationSetEventDestinationDetails {
	fn resource_type(&self) -> &'static str {
		"aws_sesv2_configuration_set_event_destination"
	}
	fn provider(&self) -> &'static terra::Provider { &terra::Provider::AWS }
	fn validate_definition(
		&self,
	) -> Result<(), terra::ResourceValidationError> {
		if self.configuration_set_name.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "configuration_set_name",
			});
		}
		if self.event_destination_name.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "event_destination_name",
			});
		}
		Ok(())
	}
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct AwsSesv2EmailIdentityDetails {
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub arn: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub configuration_set_name: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub count: Option<i64>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub depends_on: Option<Vec<SmolStr>>,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub email_identity: SmolStr,
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
	pub identity_type: Option<SmolStr>,
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
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub verification_status: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub verified_for_sending_status: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub dkim_signing_attributes: Option<
		Vec<AwsSesv2EmailIdentityResourceBlockTypeDkimSigningAttributes>,
	>,
}
impl terra::ToJson for AwsSesv2EmailIdentityDetails {
	fn to_json(&self) -> Value {
		Value::from_serde(self).expect("serialization should not fail")
	}
}
impl terra::Resource for AwsSesv2EmailIdentityDetails {
	fn resource_type(&self) -> &'static str { "aws_sesv2_email_identity" }
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
		if self.email_identity.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "email_identity",
			});
		}
		if self.identity_type.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "identity_type",
				},
			);
		}
		if self.verification_status.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "verification_status",
				},
			);
		}
		if self.verified_for_sending_status.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "verified_for_sending_status",
				},
			);
		}
		Ok(())
	}
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct AwsSesv2EmailIdentityMailFromAttributesDetails {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub behavior_on_mx_failure: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub count: Option<i64>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub depends_on: Option<Vec<SmolStr>>,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub email_identity: SmolStr,
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
	pub mail_from_domain: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub provider: Option<SmolStr>,
	/// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub region: Option<SmolStr>,
}
impl terra::ToJson for AwsSesv2EmailIdentityMailFromAttributesDetails {
	fn to_json(&self) -> Value {
		Value::from_serde(self).expect("serialization should not fail")
	}
}
impl terra::Resource for AwsSesv2EmailIdentityMailFromAttributesDetails {
	fn resource_type(&self) -> &'static str {
		"aws_sesv2_email_identity_mail_from_attributes"
	}
	fn provider(&self) -> &'static terra::Provider { &terra::Provider::AWS }
	fn validate_definition(
		&self,
	) -> Result<(), terra::ResourceValidationError> {
		if self.email_identity.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "email_identity",
			});
		}
		Ok(())
	}
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "event_destination")]
pub struct AwsSesv2ConfigurationSetEventDestinationResourceBlockTypeEventDestination
{
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub enabled: Option<bool>,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub matching_event_types: Vec<SmolStr>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub event_bridge_destination:
		Option<Vec<EventDestinationResourceBlockTypeEventBridgeDestination>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub kinesis_firehose_destination: Option<
		Vec<EventDestinationResourceBlockTypeKinesisFirehoseDestination>,
	>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub pinpoint_destination:
		Option<Vec<EventDestinationResourceBlockTypePinpointDestination>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub sns_destination:
		Option<Vec<EventDestinationResourceBlockTypeSnsDestination>>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "delivery_options")]
pub struct AwsSesv2ConfigurationSetResourceBlockTypeDeliveryOptions {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub max_delivery_seconds: Option<i64>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub sending_pool_name: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tls_policy: Option<SmolStr>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "reputation_options")]
pub struct AwsSesv2ConfigurationSetResourceBlockTypeReputationOptions {
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub last_fresh_start: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub reputation_metrics_enabled: Option<bool>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "sending_options")]
pub struct AwsSesv2ConfigurationSetResourceBlockTypeSendingOptions {
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub sending_enabled: Option<bool>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "suppression_options")]
pub struct AwsSesv2ConfigurationSetResourceBlockTypeSuppressionOptions {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub suppressed_reasons: Option<Vec<SmolStr>>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "tracking_options")]
pub struct AwsSesv2ConfigurationSetResourceBlockTypeTrackingOptions {
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub custom_redirect_domain: SmolStr,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub https_policy: Option<SmolStr>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "dkim_signing_attributes")]
pub struct AwsSesv2EmailIdentityResourceBlockTypeDkimSigningAttributes {
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub current_signing_key_length: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `sensitive`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub domain_signing_private_key: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub domain_signing_selector: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub last_key_generation_timestamp: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub next_signing_key_length: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub signing_attributes_origin: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub status: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tokens: Option<Vec<SmolStr>>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "event_bridge_destination")]
pub struct EventDestinationResourceBlockTypeEventBridgeDestination {
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub event_bus_arn: SmolStr,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "kinesis_firehose_destination")]
pub struct EventDestinationResourceBlockTypeKinesisFirehoseDestination {
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub delivery_stream_arn: SmolStr,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub iam_role_arn: SmolStr,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "pinpoint_destination")]
pub struct EventDestinationResourceBlockTypePinpointDestination {
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub application_arn: SmolStr,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "sns_destination")]
pub struct EventDestinationResourceBlockTypeSnsDestination {
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub topic_arn: SmolStr,
}
