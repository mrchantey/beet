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
pub struct AwsAcmCertificateDetails {
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub arn: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub certificate_authority_arn: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub certificate_body: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub certificate_chain: Option<SmolStr>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub count: Option<i64>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub depends_on: Option<Vec<SmolStr>>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub domain_name: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub domain_validation_options: Option<Vec<SmolStr>>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub early_renewal_duration: Option<SmolStr>,
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
	pub key_algorithm: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub not_after: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub not_before: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub pending_renewal: Option<bool>,
	/// ## Attribute
	/// `optional`, `sensitive`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub private_key: Option<SmolStr>,
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
	pub renewal_eligibility: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub renewal_summary: Option<Vec<SmolStr>>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub status: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub subject_alternative_names: Option<Vec<SmolStr>>,
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tags: Option<Map<SmolStr, SmolStr>>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tags_all: Option<Map<SmolStr, SmolStr>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub r#type: Option<SmolStr>,
	/// ## Attribute
	/// `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub validation_emails: Option<Vec<SmolStr>>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub validation_method: Option<SmolStr>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub options: Option<Vec<AwsAcmCertificateResourceBlockTypeOptions>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub validation_option:
		Option<Vec<AwsAcmCertificateResourceBlockTypeValidationOption>>,
}
impl terra::ToJson for AwsAcmCertificateDetails {
	fn to_json(&self) -> serde_json::Value {
		serde_json::to_value(self).expect("serialization should not fail")
	}
}
impl terra::Resource for AwsAcmCertificateDetails {
	fn resource_type(&self) -> &'static str { "aws_acm_certificate" }
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
		if self.domain_validation_options.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "domain_validation_options",
				},
			);
		}
		if self.not_after.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "not_after",
				},
			);
		}
		if self.not_before.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "not_before",
				},
			);
		}
		if self.pending_renewal.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "pending_renewal",
				},
			);
		}
		if self.renewal_eligibility.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "renewal_eligibility",
				},
			);
		}
		if self.renewal_summary.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "renewal_summary",
				},
			);
		}
		if self.status.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "status",
				},
			);
		}
		if self.r#type.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "type",
				},
			);
		}
		if self.validation_emails.is_some() {
			return Err(
				terra::ResourceValidationError::NonEmptyComputedField {
					resource_type: self.resource_type(),
					field_name: "validation_emails",
				},
			);
		}
		Ok(())
	}
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
pub struct AwsAcmCertificateValidationDetails {
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub certificate_arn: SmolStr,
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
	pub validation_record_fqdns: Option<Vec<SmolStr>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub timeouts:
		Option<Vec<AwsAcmCertificateValidationResourceBlockTypeTimeouts>>,
}
impl terra::ToJson for AwsAcmCertificateValidationDetails {
	fn to_json(&self) -> serde_json::Value {
		serde_json::to_value(self).expect("serialization should not fail")
	}
}
impl terra::Resource for AwsAcmCertificateValidationDetails {
	fn resource_type(&self) -> &'static str { "aws_acm_certificate_validation" }
	fn provider(&self) -> &'static terra::Provider { &terra::Provider::AWS }
	fn validate_definition(
		&self,
	) -> Result<(), terra::ResourceValidationError> {
		if self.certificate_arn.is_empty() {
			return Err(terra::ResourceValidationError::MissingRequiredField {
				resource_type: self.resource_type(),
				field_name: "certificate_arn",
			});
		}
		Ok(())
	}
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "options")]
pub struct AwsAcmCertificateResourceBlockTypeOptions {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub certificate_transparency_logging_preference: Option<SmolStr>,
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub export: Option<SmolStr>,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "validation_option")]
pub struct AwsAcmCertificateResourceBlockTypeValidationOption {
	/// ## Attribute
	/// `optional`, `computed`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub domain_name: SmolStr,
	/// ## Attribute
	/// `required`
	#[serde(skip_serializing_if = "SmolStr::is_empty")]
	pub validation_domain: SmolStr,
}
#[derive(
	Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default,
)]
#[serde(rename = "timeouts")]
pub struct AwsAcmCertificateValidationResourceBlockTypeTimeouts {
	/// ## Attribute
	/// `optional`
	#[serde(skip_serializing_if = "Option::is_none")]
	pub create: Option<SmolStr>,
}
