//! Auto-generated Terraform provider bindings — do not edit by hand.

#![allow(unused_imports, non_snake_case, non_camel_case_types, non_upper_case_globals)]
use std::collections::BTreeMap as Map;
use serde::{Serialize, Deserialize};
use serde_json;

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AwsApiGatewayRestApiDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_source: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arn: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_media_types: Option<Vec<beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_date: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_execute_api_endpoint: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_arn: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fail_on_warnings: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub for_each: Option<Vec<beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_compression_size: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "beet_core::prelude::SmolStr::is_empty")]
    pub name: beet_core::prelude::SmolStr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<
        Map<beet_core::prelude::SmolStr, beet_core::prelude::SmolStr>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub put_rest_api_mode: Option<beet_core::prelude::SmolStr>,
    /// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_resource_id: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Map<beet_core::prelude::SmolStr, beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags_all: Option<Map<beet_core::prelude::SmolStr, beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint_configuration: Option<
        Vec<AwsApiGatewayRestApiResourceBlockTypeEndpointConfiguration>,
    >,
}
impl AwsApiGatewayRestApiDetails {
    pub fn new(name: beet_core::prelude::SmolStr) -> Self {
        Self {
            api_key_source: None,
            arn: None,
            binary_media_types: None,
            body: None,
            count: None,
            created_date: None,
            depends_on: None,
            description: None,
            disable_execute_api_endpoint: None,
            execution_arn: None,
            fail_on_warnings: None,
            for_each: None,
            id: None,
            minimum_compression_size: None,
            name,
            parameters: None,
            policy: None,
            provider: None,
            put_rest_api_mode: None,
            region: None,
            root_resource_id: None,
            tags: None,
            tags_all: None,
            endpoint_configuration: None,
        }
    }
}
impl crate::prelude::TerraJson for AwsApiGatewayRestApiDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl crate::prelude::TerraResource for AwsApiGatewayRestApiDetails {
    fn resource_type(&self) -> &'static str {
        "aws_api_gateway_rest_api"
    }
    fn provider(&self) -> &'static crate::prelude::TerraProvider {
        &crate::prelude::TerraProvider::AWS
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AwsApigatewayv2ApiDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_endpoint: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_selection_expression: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arn: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials_arn: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_execute_api_endpoint: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_arn: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fail_on_warnings: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub for_each: Option<Vec<beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address_type: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "beet_core::prelude::SmolStr::is_empty")]
    pub name: beet_core::prelude::SmolStr,
    #[serde(skip_serializing_if = "beet_core::prelude::SmolStr::is_empty")]
    pub protocol_type: beet_core::prelude::SmolStr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<beet_core::prelude::SmolStr>,
    /// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_key: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_selection_expression: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Map<beet_core::prelude::SmolStr, beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags_all: Option<Map<beet_core::prelude::SmolStr, beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cors_configuration: Option<
        Vec<AwsApigatewayv2ApiResourceBlockTypeCorsConfiguration>,
    >,
}
impl AwsApigatewayv2ApiDetails {
    pub fn new(
        name: beet_core::prelude::SmolStr,
        protocol_type: beet_core::prelude::SmolStr,
    ) -> Self {
        Self {
            api_endpoint: None,
            api_key_selection_expression: None,
            arn: None,
            body: None,
            count: None,
            credentials_arn: None,
            depends_on: None,
            description: None,
            disable_execute_api_endpoint: None,
            execution_arn: None,
            fail_on_warnings: None,
            for_each: None,
            id: None,
            ip_address_type: None,
            name,
            protocol_type,
            provider: None,
            region: None,
            route_key: None,
            route_selection_expression: None,
            tags: None,
            tags_all: None,
            target: None,
            version: None,
            cors_configuration: None,
        }
    }
}
impl crate::prelude::TerraJson for AwsApigatewayv2ApiDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl crate::prelude::TerraResource for AwsApigatewayv2ApiDetails {
    fn resource_type(&self) -> &'static str {
        "aws_apigatewayv2_api"
    }
    fn provider(&self) -> &'static crate::prelude::TerraProvider {
        &crate::prelude::TerraProvider::AWS
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AwsApigatewayv2IntegrationDetails {
    #[serde(skip_serializing_if = "beet_core::prelude::SmolStr::is_empty")]
    pub api_id: beet_core::prelude::SmolStr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_type: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_handling_strategy: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials_arn: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub for_each: Option<Vec<beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration_method: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration_response_selection_expression: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration_subtype: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "beet_core::prelude::SmolStr::is_empty")]
    pub integration_type: beet_core::prelude::SmolStr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration_uri: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passthrough_behavior: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_format_version: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<beet_core::prelude::SmolStr>,
    /// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_parameters: Option<
        Map<beet_core::prelude::SmolStr, beet_core::prelude::SmolStr>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_templates: Option<
        Map<beet_core::prelude::SmolStr, beet_core::prelude::SmolStr>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_selection_expression: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_milliseconds: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_parameters: Option<
        Vec<AwsApigatewayv2IntegrationResourceBlockTypeResponseParameters>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls_config: Option<Vec<AwsApigatewayv2IntegrationResourceBlockTypeTlsConfig>>,
}
impl AwsApigatewayv2IntegrationDetails {
    pub fn new(
        api_id: beet_core::prelude::SmolStr,
        integration_type: beet_core::prelude::SmolStr,
    ) -> Self {
        Self {
            api_id,
            connection_id: None,
            connection_type: None,
            content_handling_strategy: None,
            count: None,
            credentials_arn: None,
            depends_on: None,
            description: None,
            for_each: None,
            id: None,
            integration_method: None,
            integration_response_selection_expression: None,
            integration_subtype: None,
            integration_type,
            integration_uri: None,
            passthrough_behavior: None,
            payload_format_version: None,
            provider: None,
            region: None,
            request_parameters: None,
            request_templates: None,
            template_selection_expression: None,
            timeout_milliseconds: None,
            response_parameters: None,
            tls_config: None,
        }
    }
}
impl crate::prelude::TerraJson for AwsApigatewayv2IntegrationDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl crate::prelude::TerraResource for AwsApigatewayv2IntegrationDetails {
    fn resource_type(&self) -> &'static str {
        "aws_apigatewayv2_integration"
    }
    fn provider(&self) -> &'static crate::prelude::TerraProvider {
        &crate::prelude::TerraProvider::AWS
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AwsApigatewayv2RouteDetails {
    #[serde(skip_serializing_if = "beet_core::prelude::SmolStr::is_empty")]
    pub api_id: beet_core::prelude::SmolStr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization_scopes: Option<Vec<beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization_type: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorizer_id: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub for_each: Option<Vec<beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_selection_expression: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_name: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<beet_core::prelude::SmolStr>,
    /// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_models: Option<
        Map<beet_core::prelude::SmolStr, beet_core::prelude::SmolStr>,
    >,
    #[serde(skip_serializing_if = "beet_core::prelude::SmolStr::is_empty")]
    pub route_key: beet_core::prelude::SmolStr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_response_selection_expression: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_parameter: Option<
        Vec<AwsApigatewayv2RouteResourceBlockTypeRequestParameter>,
    >,
}
impl AwsApigatewayv2RouteDetails {
    pub fn new(
        api_id: beet_core::prelude::SmolStr,
        route_key: beet_core::prelude::SmolStr,
    ) -> Self {
        Self {
            api_id,
            api_key_required: None,
            authorization_scopes: None,
            authorization_type: None,
            authorizer_id: None,
            count: None,
            depends_on: None,
            for_each: None,
            id: None,
            model_selection_expression: None,
            operation_name: None,
            provider: None,
            region: None,
            request_models: None,
            route_key,
            route_response_selection_expression: None,
            target: None,
            request_parameter: None,
        }
    }
}
impl crate::prelude::TerraJson for AwsApigatewayv2RouteDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl crate::prelude::TerraResource for AwsApigatewayv2RouteDetails {
    fn resource_type(&self) -> &'static str {
        "aws_apigatewayv2_route"
    }
    fn provider(&self) -> &'static crate::prelude::TerraProvider {
        &crate::prelude::TerraProvider::AWS
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AwsApigatewayv2StageDetails {
    #[serde(skip_serializing_if = "beet_core::prelude::SmolStr::is_empty")]
    pub api_id: beet_core::prelude::SmolStr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arn: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_deploy: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_certificate_id: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_id: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_arn: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub for_each: Option<Vec<beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoke_url: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "beet_core::prelude::SmolStr::is_empty")]
    pub name: beet_core::prelude::SmolStr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<beet_core::prelude::SmolStr>,
    /// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage_variables: Option<
        Map<beet_core::prelude::SmolStr, beet_core::prelude::SmolStr>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Map<beet_core::prelude::SmolStr, beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags_all: Option<Map<beet_core::prelude::SmolStr, beet_core::prelude::SmolStr>>,
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
impl AwsApigatewayv2StageDetails {
    pub fn new(
        api_id: beet_core::prelude::SmolStr,
        name: beet_core::prelude::SmolStr,
    ) -> Self {
        Self {
            api_id,
            arn: None,
            auto_deploy: None,
            client_certificate_id: None,
            count: None,
            depends_on: None,
            deployment_id: None,
            description: None,
            execution_arn: None,
            for_each: None,
            id: None,
            invoke_url: None,
            name,
            provider: None,
            region: None,
            stage_variables: None,
            tags: None,
            tags_all: None,
            access_log_settings: None,
            default_route_settings: None,
            route_settings: None,
        }
    }
}
impl crate::prelude::TerraJson for AwsApigatewayv2StageDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl crate::prelude::TerraResource for AwsApigatewayv2StageDetails {
    fn resource_type(&self) -> &'static str {
        "aws_apigatewayv2_stage"
    }
    fn provider(&self) -> &'static crate::prelude::TerraProvider {
        &crate::prelude::TerraProvider::AWS
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AwsLambdaFunctionDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub architectures: Option<Vec<beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arn: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_sha256: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_signing_config_arn: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub for_each: Option<Vec<beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "beet_core::prelude::SmolStr::is_empty")]
    pub function_name: beet_core::prelude::SmolStr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handler: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_uri: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoke_arn: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kms_key_arn: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layers: Option<Vec<beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_size: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_type: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publish: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publish_to: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qualified_arn: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qualified_invoke_arn: Option<beet_core::prelude::SmolStr>,
    /// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replace_security_groups_on_destroy: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement_security_group_ids: Option<Vec<beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reserved_concurrent_executions: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_streaming_invoke_arn: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "beet_core::prelude::SmolStr::is_empty")]
    pub role: beet_core::prelude::SmolStr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s3_bucket: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s3_key: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s3_object_version: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signing_job_arn: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signing_profile_version_arn: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_destroy: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_code_hash: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_code_size: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_kms_key_arn: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Map<beet_core::prelude::SmolStr, beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags_all: Option<Map<beet_core::prelude::SmolStr, beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<beet_core::prelude::SmolStr>,
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
impl AwsLambdaFunctionDetails {
    pub fn new(
        function_name: beet_core::prelude::SmolStr,
        role: beet_core::prelude::SmolStr,
    ) -> Self {
        Self {
            architectures: None,
            arn: None,
            code_sha256: None,
            code_signing_config_arn: None,
            count: None,
            depends_on: None,
            description: None,
            filename: None,
            for_each: None,
            function_name,
            handler: None,
            id: None,
            image_uri: None,
            invoke_arn: None,
            kms_key_arn: None,
            last_modified: None,
            layers: None,
            memory_size: None,
            package_type: None,
            provider: None,
            publish: None,
            publish_to: None,
            qualified_arn: None,
            qualified_invoke_arn: None,
            region: None,
            replace_security_groups_on_destroy: None,
            replacement_security_group_ids: None,
            reserved_concurrent_executions: None,
            response_streaming_invoke_arn: None,
            role,
            runtime: None,
            s3_bucket: None,
            s3_key: None,
            s3_object_version: None,
            signing_job_arn: None,
            signing_profile_version_arn: None,
            skip_destroy: None,
            source_code_hash: None,
            source_code_size: None,
            source_kms_key_arn: None,
            tags: None,
            tags_all: None,
            timeout: None,
            version: None,
            dead_letter_config: None,
            durable_config: None,
            environment: None,
            ephemeral_storage: None,
            file_system_config: None,
            image_config: None,
            logging_config: None,
            snap_start: None,
            tenancy_config: None,
            timeouts: None,
            tracing_config: None,
            vpc_config: None,
        }
    }
}
impl crate::prelude::TerraJson for AwsLambdaFunctionDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl crate::prelude::TerraResource for AwsLambdaFunctionDetails {
    fn resource_type(&self) -> &'static str {
        "aws_lambda_function"
    }
    fn provider(&self) -> &'static crate::prelude::TerraProvider {
        &crate::prelude::TerraProvider::AWS
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AwsLambdaFunctionUrlDetails {
    #[serde(skip_serializing_if = "beet_core::prelude::SmolStr::is_empty")]
    pub authorization_type: beet_core::prelude::SmolStr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub for_each: Option<Vec<beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_arn: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "beet_core::prelude::SmolStr::is_empty")]
    pub function_name: beet_core::prelude::SmolStr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_url: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoke_mode: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qualifier: Option<beet_core::prelude::SmolStr>,
    /// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_id: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cors: Option<Vec<AwsLambdaFunctionUrlResourceBlockTypeCors>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeouts: Option<Vec<AwsLambdaFunctionUrlResourceBlockTypeTimeouts>>,
}
impl AwsLambdaFunctionUrlDetails {
    pub fn new(
        authorization_type: beet_core::prelude::SmolStr,
        function_name: beet_core::prelude::SmolStr,
    ) -> Self {
        Self {
            authorization_type,
            count: None,
            depends_on: None,
            for_each: None,
            function_arn: None,
            function_name,
            function_url: None,
            id: None,
            invoke_mode: None,
            provider: None,
            qualifier: None,
            region: None,
            url_id: None,
            cors: None,
            timeouts: None,
        }
    }
}
impl crate::prelude::TerraJson for AwsLambdaFunctionUrlDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl crate::prelude::TerraResource for AwsLambdaFunctionUrlDetails {
    fn resource_type(&self) -> &'static str {
        "aws_lambda_function_url"
    }
    fn provider(&self) -> &'static crate::prelude::TerraProvider {
        &crate::prelude::TerraProvider::AWS
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AwsLambdaPermissionDetails {
    #[serde(skip_serializing_if = "beet_core::prelude::SmolStr::is_empty")]
    pub action: beet_core::prelude::SmolStr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_source_token: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub for_each: Option<Vec<beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "beet_core::prelude::SmolStr::is_empty")]
    pub function_name: beet_core::prelude::SmolStr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_url_auth_type: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoked_via_function_url: Option<bool>,
    #[serde(skip_serializing_if = "beet_core::prelude::SmolStr::is_empty")]
    pub principal: beet_core::prelude::SmolStr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub principal_org_id: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qualifier: Option<beet_core::prelude::SmolStr>,
    /// Region where this resource will be [managed](https://docs.aws.amazon.com/general/latest/gr/rande.html#regional-endpoints). Defaults to the Region set in the [provider configuration](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#aws-configuration-reference).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_account: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_arn: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statement_id: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statement_id_prefix: Option<beet_core::prelude::SmolStr>,
}
impl AwsLambdaPermissionDetails {
    pub fn new(
        action: beet_core::prelude::SmolStr,
        function_name: beet_core::prelude::SmolStr,
        principal: beet_core::prelude::SmolStr,
    ) -> Self {
        Self {
            action,
            count: None,
            depends_on: None,
            event_source_token: None,
            for_each: None,
            function_name,
            function_url_auth_type: None,
            id: None,
            invoked_via_function_url: None,
            principal,
            principal_org_id: None,
            provider: None,
            qualifier: None,
            region: None,
            source_account: None,
            source_arn: None,
            statement_id: None,
            statement_id_prefix: None,
        }
    }
}
impl crate::prelude::TerraJson for AwsLambdaPermissionDetails {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialization should not fail")
    }
}
impl crate::prelude::TerraResource for AwsLambdaPermissionDetails {
    fn resource_type(&self) -> &'static str {
        "aws_lambda_permission"
    }
    fn provider(&self) -> &'static crate::prelude::TerraProvider {
        &crate::prelude::TerraProvider::AWS
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename = "endpoint_configuration")]
pub struct AwsApiGatewayRestApiResourceBlockTypeEndpointConfiguration {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address_type: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub types: Vec<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vpc_endpoint_ids: Option<Vec<beet_core::prelude::SmolStr>>,
}
impl AwsApiGatewayRestApiResourceBlockTypeEndpointConfiguration {
    pub fn new(types: Vec<beet_core::prelude::SmolStr>) -> Self {
        Self {
            ip_address_type: None,
            types,
            vpc_endpoint_ids: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "cors_configuration")]
pub struct AwsApigatewayv2ApiResourceBlockTypeCorsConfiguration {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_credentials: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_headers: Option<Vec<beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_methods: Option<Vec<beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_origins: Option<Vec<beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expose_headers: Option<Vec<beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_age: Option<i64>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename = "response_parameters")]
pub struct AwsApigatewayv2IntegrationResourceBlockTypeResponseParameters {
    pub mappings: Map<beet_core::prelude::SmolStr, beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "beet_core::prelude::SmolStr::is_empty")]
    pub status_code: beet_core::prelude::SmolStr,
}
impl AwsApigatewayv2IntegrationResourceBlockTypeResponseParameters {
    pub fn new(
        mappings: Map<beet_core::prelude::SmolStr, beet_core::prelude::SmolStr>,
        status_code: beet_core::prelude::SmolStr,
    ) -> Self {
        Self { mappings, status_code }
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "tls_config")]
pub struct AwsApigatewayv2IntegrationResourceBlockTypeTlsConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_name_to_verify: Option<beet_core::prelude::SmolStr>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename = "request_parameter")]
pub struct AwsApigatewayv2RouteResourceBlockTypeRequestParameter {
    #[serde(skip_serializing_if = "beet_core::prelude::SmolStr::is_empty")]
    pub request_parameter_key: beet_core::prelude::SmolStr,
    pub required: bool,
}
impl AwsApigatewayv2RouteResourceBlockTypeRequestParameter {
    pub fn new(
        request_parameter_key: beet_core::prelude::SmolStr,
        required: bool,
    ) -> Self {
        Self {
            request_parameter_key,
            required,
        }
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename = "access_log_settings")]
pub struct AwsApigatewayv2StageResourceBlockTypeAccessLogSettings {
    #[serde(skip_serializing_if = "beet_core::prelude::SmolStr::is_empty")]
    pub destination_arn: beet_core::prelude::SmolStr,
    #[serde(skip_serializing_if = "beet_core::prelude::SmolStr::is_empty")]
    pub format: beet_core::prelude::SmolStr,
}
impl AwsApigatewayv2StageResourceBlockTypeAccessLogSettings {
    pub fn new(
        destination_arn: beet_core::prelude::SmolStr,
        format: beet_core::prelude::SmolStr,
    ) -> Self {
        Self { destination_arn, format }
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "default_route_settings")]
pub struct AwsApigatewayv2StageResourceBlockTypeDefaultRouteSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_trace_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detailed_metrics_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging_level: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub throttling_burst_limit: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub throttling_rate_limit: Option<i64>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename = "route_settings")]
pub struct AwsApigatewayv2StageResourceBlockTypeRouteSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_trace_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detailed_metrics_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging_level: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "beet_core::prelude::SmolStr::is_empty")]
    pub route_key: beet_core::prelude::SmolStr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub throttling_burst_limit: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub throttling_rate_limit: Option<i64>,
}
impl AwsApigatewayv2StageResourceBlockTypeRouteSettings {
    pub fn new(route_key: beet_core::prelude::SmolStr) -> Self {
        Self {
            data_trace_enabled: None,
            detailed_metrics_enabled: None,
            logging_level: None,
            route_key,
            throttling_burst_limit: None,
            throttling_rate_limit: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename = "dead_letter_config")]
pub struct AwsLambdaFunctionResourceBlockTypeDeadLetterConfig {
    #[serde(skip_serializing_if = "beet_core::prelude::SmolStr::is_empty")]
    pub target_arn: beet_core::prelude::SmolStr,
}
impl AwsLambdaFunctionResourceBlockTypeDeadLetterConfig {
    pub fn new(target_arn: beet_core::prelude::SmolStr) -> Self {
        Self { target_arn }
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename = "durable_config")]
pub struct AwsLambdaFunctionResourceBlockTypeDurableConfig {
    pub execution_timeout: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_period: Option<i64>,
}
impl AwsLambdaFunctionResourceBlockTypeDurableConfig {
    pub fn new(execution_timeout: i64) -> Self {
        Self {
            execution_timeout,
            retention_period: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "environment")]
pub struct AwsLambdaFunctionResourceBlockTypeEnvironment {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Map<beet_core::prelude::SmolStr, beet_core::prelude::SmolStr>>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "ephemeral_storage")]
pub struct AwsLambdaFunctionResourceBlockTypeEphemeralStorage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<i64>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename = "file_system_config")]
pub struct AwsLambdaFunctionResourceBlockTypeFileSystemConfig {
    #[serde(skip_serializing_if = "beet_core::prelude::SmolStr::is_empty")]
    pub arn: beet_core::prelude::SmolStr,
    #[serde(skip_serializing_if = "beet_core::prelude::SmolStr::is_empty")]
    pub local_mount_path: beet_core::prelude::SmolStr,
}
impl AwsLambdaFunctionResourceBlockTypeFileSystemConfig {
    pub fn new(
        arn: beet_core::prelude::SmolStr,
        local_mount_path: beet_core::prelude::SmolStr,
    ) -> Self {
        Self { arn, local_mount_path }
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "image_config")]
pub struct AwsLambdaFunctionResourceBlockTypeImageConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<Vec<beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_point: Option<Vec<beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<beet_core::prelude::SmolStr>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename = "logging_config")]
pub struct AwsLambdaFunctionResourceBlockTypeLoggingConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub application_log_level: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "beet_core::prelude::SmolStr::is_empty")]
    pub log_format: beet_core::prelude::SmolStr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_group: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_log_level: Option<beet_core::prelude::SmolStr>,
}
impl AwsLambdaFunctionResourceBlockTypeLoggingConfig {
    pub fn new(log_format: beet_core::prelude::SmolStr) -> Self {
        Self {
            application_log_level: None,
            log_format,
            log_group: None,
            system_log_level: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename = "snap_start")]
pub struct AwsLambdaFunctionResourceBlockTypeSnapStart {
    #[serde(skip_serializing_if = "beet_core::prelude::SmolStr::is_empty")]
    pub apply_on: beet_core::prelude::SmolStr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optimization_status: Option<beet_core::prelude::SmolStr>,
}
impl AwsLambdaFunctionResourceBlockTypeSnapStart {
    pub fn new(apply_on: beet_core::prelude::SmolStr) -> Self {
        Self {
            apply_on,
            optimization_status: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename = "tenancy_config")]
pub struct AwsLambdaFunctionResourceBlockTypeTenancyConfig {
    #[serde(skip_serializing_if = "beet_core::prelude::SmolStr::is_empty")]
    pub tenant_isolation_mode: beet_core::prelude::SmolStr,
}
impl AwsLambdaFunctionResourceBlockTypeTenancyConfig {
    pub fn new(tenant_isolation_mode: beet_core::prelude::SmolStr) -> Self {
        Self { tenant_isolation_mode }
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "timeouts")]
pub struct AwsLambdaFunctionResourceBlockTypeTimeouts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete: Option<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update: Option<beet_core::prelude::SmolStr>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename = "tracing_config")]
pub struct AwsLambdaFunctionResourceBlockTypeTracingConfig {
    #[serde(skip_serializing_if = "beet_core::prelude::SmolStr::is_empty")]
    pub mode: beet_core::prelude::SmolStr,
}
impl AwsLambdaFunctionResourceBlockTypeTracingConfig {
    pub fn new(mode: beet_core::prelude::SmolStr) -> Self {
        Self { mode }
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename = "vpc_config")]
pub struct AwsLambdaFunctionResourceBlockTypeVpcConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipv6_allowed_for_dual_stack: Option<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub security_group_ids: Vec<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub subnet_ids: Vec<beet_core::prelude::SmolStr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vpc_id: Option<beet_core::prelude::SmolStr>,
}
impl AwsLambdaFunctionResourceBlockTypeVpcConfig {
    pub fn new(
        security_group_ids: Vec<beet_core::prelude::SmolStr>,
        subnet_ids: Vec<beet_core::prelude::SmolStr>,
    ) -> Self {
        Self {
            ipv6_allowed_for_dual_stack: None,
            security_group_ids,
            subnet_ids,
            vpc_id: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "cors")]
pub struct AwsLambdaFunctionUrlResourceBlockTypeCors {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_credentials: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_headers: Option<Vec<beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_methods: Option<Vec<beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_origins: Option<Vec<beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expose_headers: Option<Vec<beet_core::prelude::SmolStr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_age: Option<i64>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename = "timeouts")]
pub struct AwsLambdaFunctionUrlResourceBlockTypeTimeouts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create: Option<beet_core::prelude::SmolStr>,
}
