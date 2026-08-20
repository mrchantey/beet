//! non-generated additions
#[allow(unused)]
use crate::bindings::*;
#[allow(unused)]
use crate::prelude::*;

pub mod region {
	/// The [`Stack`](crate::prelude::Stack)'s region fallback, and nothing
	/// else's. A block must not reach for this: region is a property of the
	/// scope a resource is declared in, so a block resolves it from its stack.
	pub(crate) const DEFAULT: &str = US_WEST_2;
	pub const US_EAST_1: &str = "us-east-1";
	pub const US_WEST_2: &str = "us-west-2";
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
