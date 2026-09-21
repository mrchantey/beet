//! non-generated additions
#[allow(unused)]
use crate::bindings::*;
#[allow(unused)]
use crate::prelude::*;

/// The regions the live stacks and their tests name. No default: a stack
/// declares its region ([`AwsRegion`](crate::prelude::AwsRegion)) and a block
/// reads it from its resolved stack.
pub mod region {
	pub const US_EAST_1: &str = "us-east-1";
	pub const US_WEST_2: &str = "us-west-2";
	pub const AP_SOUTHEAST_2: &str = "ap-southeast-2";
}

#[cfg(feature = "bindings_aws_common")]
impl terra::PrimaryResource for AwsS3BucketDetails {
	fn set_primary_identifier(&mut self, name: &str) {
		self.bucket = Some(name.into())
	}
}
#[cfg(feature = "bindings_aws_common")]
impl terra::PrimaryResource for AwsIamRoleDetails {
	fn set_primary_identifier(&mut self, name: &str) {
		self.name = Some(name.into())
	}
}
#[cfg(feature = "bindings_aws_common")]
impl terra::PrimaryResource for AwsIamUserDetails {
	fn set_primary_identifier(&mut self, name: &str) { self.name = name.into() }
}
#[cfg(feature = "bindings_aws_dynamo")]
impl terra::PrimaryResource for AwsDynamodbTableDetails {
	fn set_primary_identifier(&mut self, name: &str) { self.name = name.into() }
}
#[cfg(feature = "bindings_aws_lambda")]
impl terra::PrimaryResource for AwsLambdaFunctionDetails {
	fn set_primary_identifier(&mut self, name: &str) {
		self.function_name = name.into()
	}
}
#[cfg(feature = "bindings_aws_lambda")]
impl terra::PrimaryResource for AwsApigatewayv2ApiDetails {
	fn set_primary_identifier(&mut self, name: &str) { self.name = name.into() }
}

#[cfg(feature = "bindings_aws_fargate")]
impl terra::PrimaryResource for AwsEcrRepositoryDetails {
	fn set_primary_identifier(&mut self, name: &str) { self.name = name.into() }
}
#[cfg(feature = "bindings_aws_fargate")]
impl terra::PrimaryResource for AwsEcsClusterDetails {
	fn set_primary_identifier(&mut self, name: &str) { self.name = name.into() }
}
#[cfg(feature = "bindings_aws_fargate")]
impl terra::PrimaryResource for AwsEcsTaskDefinitionDetails {
	fn set_primary_identifier(&mut self, name: &str) {
		self.family = name.into()
	}
}
#[cfg(feature = "bindings_aws_fargate")]
impl terra::PrimaryResource for AwsLbDetails {
	fn set_primary_identifier(&mut self, name: &str) {
		self.name = Some(name.into())
	}
}

#[cfg(feature = "bindings_aws_vpc")]
impl terra::PrimaryResource for AwsSecurityGroupDetails {
	fn set_primary_identifier(&mut self, name: &str) {
		self.name = Some(name.into())
	}
}
