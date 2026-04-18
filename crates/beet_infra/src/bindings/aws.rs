//! non-generated additions
#[allow(unused)]
use crate::bindings::*;
#[allow(unused)]
use crate::prelude::*;




pub mod region {
	pub const DEFAULT: &str = US_EAST_1;
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
	fn set_primary_identifier(&mut self, name: &str) {
		self.name = name.into()
	}
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
