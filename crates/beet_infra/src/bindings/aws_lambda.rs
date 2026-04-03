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
pub struct AwsApiGatewayRestApiDetails {
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_source: Option<SmolStr>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arn: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_media_types: Option<Vec<SmolStr>>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_date: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<SmolStr>>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_execute_api_endpoint: Option<bool>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_arn: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fail_on_warnings: Option<bool>,
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
    pub minimum_compression_size: Option<SmolStr>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub name: SmolStr,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Map<SmolStr, SmolStr>>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub put_rest_api_mode: Option<SmolStr>,
    /// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<SmolStr>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_resource_id: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Map<SmolStr, SmolStr>>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags_all: Option<Map<SmolStr, SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint_configuration: Option<
        Vec<AwsApiGatewayRestApiResourceBlockTypeEndpointConfiguration>,
    >,
}
impl terra::ToJson for AwsApiGatewayRestApiDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl terra::Resource for AwsApiGatewayRestApiDetails {
    fn resource_type(&self) -> &'static str {
        "aws_api_gateway_rest_api"
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
        if self.created_date.is_some() {
            bevybail!(
                "{}: computed-only field `created_date` should not be set", self
                .resource_type()
            );
        }
        if self.execution_arn.is_some() {
            bevybail!(
                "{}: computed-only field `execution_arn` should not be set", self
                .resource_type()
            );
        }
        if self.name.is_empty() {
            bevybail!("{}: required field `name` is empty", self.resource_type());
        }
        if self.root_resource_id.is_some() {
            bevybail!(
                "{}: computed-only field `root_resource_id` should not be set", self
                .resource_type()
            );
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
pub struct AwsApigatewayv2ApiDetails {
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_endpoint: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_selection_expression: Option<SmolStr>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arn: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials_arn: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<SmolStr>>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_execute_api_endpoint: Option<bool>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_arn: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fail_on_warnings: Option<bool>,
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
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub name: SmolStr,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub protocol_type: SmolStr,
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
    pub route_key: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_selection_expression: Option<SmolStr>,
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
    pub target: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cors_configuration: Option<
        Vec<AwsApigatewayv2ApiResourceBlockTypeCorsConfiguration>,
    >,
}
impl terra::ToJson for AwsApigatewayv2ApiDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl terra::Resource for AwsApigatewayv2ApiDetails {
    fn resource_type(&self) -> &'static str {
        "aws_apigatewayv2_api"
    }
    fn provider(&self) -> &'static terra::Provider {
        &terra::Provider::AWS
    }
    fn validate_definition(&self) -> Result {
        if self.api_endpoint.is_some() {
            bevybail!(
                "{}: computed-only field `api_endpoint` should not be set", self
                .resource_type()
            );
        }
        if self.arn.is_some() {
            bevybail!(
                "{}: computed-only field `arn` should not be set", self.resource_type()
            );
        }
        if self.execution_arn.is_some() {
            bevybail!(
                "{}: computed-only field `execution_arn` should not be set", self
                .resource_type()
            );
        }
        if self.name.is_empty() {
            bevybail!("{}: required field `name` is empty", self.resource_type());
        }
        if self.protocol_type.is_empty() {
            bevybail!(
                "{}: required field `protocol_type` is empty", self.resource_type()
            );
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
pub struct AwsApigatewayv2IntegrationDetails {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub api_id: SmolStr,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_type: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_handling_strategy: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials_arn: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<SmolStr>>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<SmolStr>,
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
    pub integration_method: Option<SmolStr>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration_response_selection_expression: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration_subtype: Option<SmolStr>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub integration_type: SmolStr,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration_uri: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passthrough_behavior: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_format_version: Option<SmolStr>,
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
    pub request_parameters: Option<Map<SmolStr, SmolStr>>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_templates: Option<Map<SmolStr, SmolStr>>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_selection_expression: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_milliseconds: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_parameters: Option<
        Vec<AwsApigatewayv2IntegrationResourceBlockTypeResponseParameters>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls_config: Option<Vec<AwsApigatewayv2IntegrationResourceBlockTypeTlsConfig>>,
}
impl terra::ToJson for AwsApigatewayv2IntegrationDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl terra::Resource for AwsApigatewayv2IntegrationDetails {
    fn resource_type(&self) -> &'static str {
        "aws_apigatewayv2_integration"
    }
    fn provider(&self) -> &'static terra::Provider {
        &terra::Provider::AWS
    }
    fn validate_definition(&self) -> Result {
        if self.api_id.is_empty() {
            bevybail!("{}: required field `api_id` is empty", self.resource_type());
        }
        if self.integration_response_selection_expression.is_some() {
            bevybail!(
                "{}: computed-only field `integration_response_selection_expression` should not be set",
                self.resource_type()
            );
        }
        if self.integration_type.is_empty() {
            bevybail!(
                "{}: required field `integration_type` is empty", self.resource_type()
            );
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
pub struct AwsApigatewayv2RouteDetails {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub api_id: SmolStr,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_required: Option<bool>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization_scopes: Option<Vec<SmolStr>>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization_type: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorizer_id: Option<SmolStr>,
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
    pub model_selection_expression: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_name: Option<SmolStr>,
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
    pub request_models: Option<Map<SmolStr, SmolStr>>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub route_key: SmolStr,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_response_selection_expression: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_parameter: Option<
        Vec<AwsApigatewayv2RouteResourceBlockTypeRequestParameter>,
    >,
}
impl terra::ToJson for AwsApigatewayv2RouteDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl terra::Resource for AwsApigatewayv2RouteDetails {
    fn resource_type(&self) -> &'static str {
        "aws_apigatewayv2_route"
    }
    fn provider(&self) -> &'static terra::Provider {
        &terra::Provider::AWS
    }
    fn validate_definition(&self) -> Result {
        if self.api_id.is_empty() {
            bevybail!("{}: required field `api_id` is empty", self.resource_type());
        }
        if self.route_key.is_empty() {
            bevybail!("{}: required field `route_key` is empty", self.resource_type());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
pub struct AwsApigatewayv2StageDetails {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub api_id: SmolStr,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arn: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_deploy: Option<bool>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_certificate_id: Option<SmolStr>,
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
    pub deployment_id: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<SmolStr>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_arn: Option<SmolStr>,
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
    pub invoke_url: Option<SmolStr>,
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
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage_variables: Option<Map<SmolStr, SmolStr>>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Map<SmolStr, SmolStr>>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags_all: Option<Map<SmolStr, SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_log_settings: Option<
        Vec<AwsApigatewayv2StageResourceBlockTypeAccessLogSettings>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_route_settings: Option<
        Vec<AwsApigatewayv2StageResourceBlockTypeDefaultRouteSettings>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_settings: Option<Vec<AwsApigatewayv2StageResourceBlockTypeRouteSettings>>,
}
impl terra::ToJson for AwsApigatewayv2StageDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl terra::Resource for AwsApigatewayv2StageDetails {
    fn resource_type(&self) -> &'static str {
        "aws_apigatewayv2_stage"
    }
    fn provider(&self) -> &'static terra::Provider {
        &terra::Provider::AWS
    }
    fn validate_definition(&self) -> Result {
        if self.api_id.is_empty() {
            bevybail!("{}: required field `api_id` is empty", self.resource_type());
        }
        if self.arn.is_some() {
            bevybail!(
                "{}: computed-only field `arn` should not be set", self.resource_type()
            );
        }
        if self.execution_arn.is_some() {
            bevybail!(
                "{}: computed-only field `execution_arn` should not be set", self
                .resource_type()
            );
        }
        if self.invoke_url.is_some() {
            bevybail!(
                "{}: computed-only field `invoke_url` should not be set", self
                .resource_type()
            );
        }
        if self.name.is_empty() {
            bevybail!("{}: required field `name` is empty", self.resource_type());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
pub struct AwsLambdaFunctionDetails {
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub architectures: Option<Vec<SmolStr>>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arn: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_sha256: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_signing_config_arn: Option<SmolStr>,
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
    pub description: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub for_each: Option<Vec<SmolStr>>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub function_name: SmolStr,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handler: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_uri: Option<SmolStr>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoke_arn: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kms_key_arn: Option<SmolStr>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layers: Option<Vec<SmolStr>>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_size: Option<i64>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_type: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publish: Option<bool>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publish_to: Option<SmolStr>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qualified_arn: Option<SmolStr>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qualified_invoke_arn: Option<SmolStr>,
    /// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replace_security_groups_on_destroy: Option<bool>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement_security_group_ids: Option<Vec<SmolStr>>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reserved_concurrent_executions: Option<i64>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_streaming_invoke_arn: Option<SmolStr>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub role: SmolStr,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s3_bucket: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s3_key: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s3_object_version: Option<SmolStr>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signing_job_arn: Option<SmolStr>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signing_profile_version_arn: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_destroy: Option<bool>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_code_hash: Option<SmolStr>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_code_size: Option<i64>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_kms_key_arn: Option<SmolStr>,
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
    pub timeout: Option<i64>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dead_letter_config: Option<
        Vec<AwsLambdaFunctionResourceBlockTypeDeadLetterConfig>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub durable_config: Option<Vec<AwsLambdaFunctionResourceBlockTypeDurableConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<Vec<AwsLambdaFunctionResourceBlockTypeEnvironment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_storage: Option<
        Vec<AwsLambdaFunctionResourceBlockTypeEphemeralStorage>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_system_config: Option<
        Vec<AwsLambdaFunctionResourceBlockTypeFileSystemConfig>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_config: Option<Vec<AwsLambdaFunctionResourceBlockTypeImageConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging_config: Option<Vec<AwsLambdaFunctionResourceBlockTypeLoggingConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snap_start: Option<Vec<AwsLambdaFunctionResourceBlockTypeSnapStart>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenancy_config: Option<Vec<AwsLambdaFunctionResourceBlockTypeTenancyConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeouts: Option<Vec<AwsLambdaFunctionResourceBlockTypeTimeouts>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracing_config: Option<Vec<AwsLambdaFunctionResourceBlockTypeTracingConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vpc_config: Option<Vec<AwsLambdaFunctionResourceBlockTypeVpcConfig>>,
}
impl terra::ToJson for AwsLambdaFunctionDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl terra::Resource for AwsLambdaFunctionDetails {
    fn resource_type(&self) -> &'static str {
        "aws_lambda_function"
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
        if self.function_name.is_empty() {
            bevybail!(
                "{}: required field `function_name` is empty", self.resource_type()
            );
        }
        if self.invoke_arn.is_some() {
            bevybail!(
                "{}: computed-only field `invoke_arn` should not be set", self
                .resource_type()
            );
        }
        if self.last_modified.is_some() {
            bevybail!(
                "{}: computed-only field `last_modified` should not be set", self
                .resource_type()
            );
        }
        if self.qualified_arn.is_some() {
            bevybail!(
                "{}: computed-only field `qualified_arn` should not be set", self
                .resource_type()
            );
        }
        if self.qualified_invoke_arn.is_some() {
            bevybail!(
                "{}: computed-only field `qualified_invoke_arn` should not be set", self
                .resource_type()
            );
        }
        if self.response_streaming_invoke_arn.is_some() {
            bevybail!(
                "{}: computed-only field `response_streaming_invoke_arn` should not be set",
                self.resource_type()
            );
        }
        if self.role.is_empty() {
            bevybail!("{}: required field `role` is empty", self.resource_type());
        }
        if self.signing_job_arn.is_some() {
            bevybail!(
                "{}: computed-only field `signing_job_arn` should not be set", self
                .resource_type()
            );
        }
        if self.signing_profile_version_arn.is_some() {
            bevybail!(
                "{}: computed-only field `signing_profile_version_arn` should not be set",
                self.resource_type()
            );
        }
        if self.source_code_size.is_some() {
            bevybail!(
                "{}: computed-only field `source_code_size` should not be set", self
                .resource_type()
            );
        }
        if self.version.is_some() {
            bevybail!(
                "{}: computed-only field `version` should not be set", self
                .resource_type()
            );
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
pub struct AwsLambdaFunctionUrlDetails {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub authorization_type: SmolStr,
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
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_arn: Option<SmolStr>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub function_name: SmolStr,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_url: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoke_mode: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qualifier: Option<SmolStr>,
    /// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<SmolStr>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_id: Option<SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cors: Option<Vec<AwsLambdaFunctionUrlResourceBlockTypeCors>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeouts: Option<Vec<AwsLambdaFunctionUrlResourceBlockTypeTimeouts>>,
}
impl terra::ToJson for AwsLambdaFunctionUrlDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl terra::Resource for AwsLambdaFunctionUrlDetails {
    fn resource_type(&self) -> &'static str {
        "aws_lambda_function_url"
    }
    fn provider(&self) -> &'static terra::Provider {
        &terra::Provider::AWS
    }
    fn validate_definition(&self) -> Result {
        if self.authorization_type.is_empty() {
            bevybail!(
                "{}: required field `authorization_type` is empty", self.resource_type()
            );
        }
        if self.function_arn.is_some() {
            bevybail!(
                "{}: computed-only field `function_arn` should not be set", self
                .resource_type()
            );
        }
        if self.function_name.is_empty() {
            bevybail!(
                "{}: required field `function_name` is empty", self.resource_type()
            );
        }
        if self.function_url.is_some() {
            bevybail!(
                "{}: computed-only field `function_url` should not be set", self
                .resource_type()
            );
        }
        if self.url_id.is_some() {
            bevybail!(
                "{}: computed-only field `url_id` should not be set", self
                .resource_type()
            );
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
pub struct AwsLambdaPermissionDetails {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub action: SmolStr,
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
    pub event_source_token: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub for_each: Option<Vec<SmolStr>>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub function_name: SmolStr,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_url_auth_type: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoked_via_function_url: Option<bool>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub principal: SmolStr,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub principal_org_id: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qualifier: Option<SmolStr>,
    /// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_account: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_arn: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statement_id: Option<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statement_id_prefix: Option<SmolStr>,
}
impl terra::ToJson for AwsLambdaPermissionDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl terra::Resource for AwsLambdaPermissionDetails {
    fn resource_type(&self) -> &'static str {
        "aws_lambda_permission"
    }
    fn provider(&self) -> &'static terra::Provider {
        &terra::Provider::AWS
    }
    fn validate_definition(&self) -> Result {
        if self.action.is_empty() {
            bevybail!("{}: required field `action` is empty", self.resource_type());
        }
        if self.function_name.is_empty() {
            bevybail!(
                "{}: required field `function_name` is empty", self.resource_type()
            );
        }
        if self.principal.is_empty() {
            bevybail!("{}: required field `principal` is empty", self.resource_type());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "endpoint_configuration")]
pub struct AwsApiGatewayRestApiResourceBlockTypeEndpointConfiguration {
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address_type: Option<SmolStr>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub types: Vec<SmolStr>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vpc_endpoint_ids: Option<Vec<SmolStr>>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "cors_configuration")]
pub struct AwsApigatewayv2ApiResourceBlockTypeCorsConfiguration {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_credentials: Option<bool>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_headers: Option<Vec<SmolStr>>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_methods: Option<Vec<SmolStr>>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_origins: Option<Vec<SmolStr>>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expose_headers: Option<Vec<SmolStr>>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_age: Option<i64>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "response_parameters")]
pub struct AwsApigatewayv2IntegrationResourceBlockTypeResponseParameters {
    /// ## Attribute
    /// `required`
    pub mappings: Map<SmolStr, SmolStr>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub status_code: SmolStr,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "tls_config")]
pub struct AwsApigatewayv2IntegrationResourceBlockTypeTlsConfig {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_name_to_verify: Option<SmolStr>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "request_parameter")]
pub struct AwsApigatewayv2RouteResourceBlockTypeRequestParameter {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub request_parameter_key: SmolStr,
    /// ## Attribute
    /// `required`
    pub required: bool,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "access_log_settings")]
pub struct AwsApigatewayv2StageResourceBlockTypeAccessLogSettings {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub destination_arn: SmolStr,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub format: SmolStr,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "default_route_settings")]
pub struct AwsApigatewayv2StageResourceBlockTypeDefaultRouteSettings {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_trace_enabled: Option<bool>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detailed_metrics_enabled: Option<bool>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging_level: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub throttling_burst_limit: Option<i64>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub throttling_rate_limit: Option<i64>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "route_settings")]
pub struct AwsApigatewayv2StageResourceBlockTypeRouteSettings {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_trace_enabled: Option<bool>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detailed_metrics_enabled: Option<bool>,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging_level: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub route_key: SmolStr,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub throttling_burst_limit: Option<i64>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub throttling_rate_limit: Option<i64>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "dead_letter_config")]
pub struct AwsLambdaFunctionResourceBlockTypeDeadLetterConfig {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub target_arn: SmolStr,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "durable_config")]
pub struct AwsLambdaFunctionResourceBlockTypeDurableConfig {
    /// ## Attribute
    /// `required`
    pub execution_timeout: i64,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_period: Option<i64>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "environment")]
pub struct AwsLambdaFunctionResourceBlockTypeEnvironment {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Map<SmolStr, SmolStr>>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "ephemeral_storage")]
pub struct AwsLambdaFunctionResourceBlockTypeEphemeralStorage {
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<i64>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "file_system_config")]
pub struct AwsLambdaFunctionResourceBlockTypeFileSystemConfig {
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub arn: SmolStr,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub local_mount_path: SmolStr,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "image_config")]
pub struct AwsLambdaFunctionResourceBlockTypeImageConfig {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<Vec<SmolStr>>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_point: Option<Vec<SmolStr>>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<SmolStr>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "logging_config")]
pub struct AwsLambdaFunctionResourceBlockTypeLoggingConfig {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub application_log_level: Option<SmolStr>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub log_format: SmolStr,
    /// ## Attribute
    /// `optional`, `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_group: Option<SmolStr>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_log_level: Option<SmolStr>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "snap_start")]
pub struct AwsLambdaFunctionResourceBlockTypeSnapStart {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub apply_on: SmolStr,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optimization_status: Option<SmolStr>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "tenancy_config")]
pub struct AwsLambdaFunctionResourceBlockTypeTenancyConfig {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub tenant_isolation_mode: SmolStr,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "timeouts")]
pub struct AwsLambdaFunctionResourceBlockTypeTimeouts {
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
#[serde(rename = "tracing_config")]
pub struct AwsLambdaFunctionResourceBlockTypeTracingConfig {
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub mode: SmolStr,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "vpc_config")]
pub struct AwsLambdaFunctionResourceBlockTypeVpcConfig {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipv6_allowed_for_dual_stack: Option<bool>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub security_group_ids: Vec<SmolStr>,
    /// ## Attribute
    /// `required`
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub subnet_ids: Vec<SmolStr>,
    /// ## Attribute
    /// `computed`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vpc_id: Option<SmolStr>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "cors")]
pub struct AwsLambdaFunctionUrlResourceBlockTypeCors {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_credentials: Option<bool>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_headers: Option<Vec<SmolStr>>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_methods: Option<Vec<SmolStr>>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_origins: Option<Vec<SmolStr>>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expose_headers: Option<Vec<SmolStr>>,
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_age: Option<i64>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "timeouts")]
pub struct AwsLambdaFunctionUrlResourceBlockTypeTimeouts {
    /// ## Attribute
    /// `optional`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create: Option<SmolStr>,
}
