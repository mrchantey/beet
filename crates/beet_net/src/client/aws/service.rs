use beet_core::prelude::*;

/// One AWS service as its Smithy model describes it: the endpoint prefix,
/// the signing name and the protocol it speaks, and for a global service the
/// one region it answers from. [`AwsClient`](crate::prelude::AwsClient)
/// turns one into requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AwsService {
	/// The first label of the regional endpoint, ie `api.ecr`.
	pub endpoint_prefix: &'static str,
	/// The name the credential scope names, ie `ecr`.
	pub signing_name: &'static str,
	/// The wire protocol.
	pub protocol: AwsProtocol,
	/// For a global service, its host and the region it is signed for.
	pub global: Option<(&'static str, &'static str)>,
}

/// The wire protocols beet speaks, per the `aws.protocols` trait of each
/// service's model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AwsProtocol {
	/// `awsJson1_0`: a POST to `/` naming the operation in `X-Amz-Target`.
	AwsJson1_0 {
		/// The target prefix, the service shape's name.
		target_prefix: &'static str,
	},
	/// `awsJson1_1`, the same with its own content type.
	AwsJson1_1 {
		/// The target prefix, the service shape's name.
		target_prefix: &'static str,
	},
	/// `restJson1`: the operation is the method and path, bodies are json.
	RestJson1,
	/// `restXml`: the operation is the method and path, bodies are xml (S3).
	RestXml,
	/// `awsQuery`: a form POST naming `Action` and `Version`, answered in xml.
	AwsQuery {
		/// The api version every request names.
		version: &'static str,
	},
	/// `ec2Query`, the same with EC2's own error and list shapes.
	Ec2Query {
		/// The api version every request names.
		version: &'static str,
	},
}

impl AwsProtocol {
	/// The json content type of an `awsJson` protocol.
	pub fn json_content_type(&self) -> Option<&'static str> {
		match self {
			Self::AwsJson1_0 { .. } => Some("application/x-amz-json-1.0"),
			Self::AwsJson1_1 { .. } => Some("application/x-amz-json-1.1"),
			_ => None,
		}
	}
}

impl AwsService {
	const fn regional(
		endpoint_prefix: &'static str,
		signing_name: &'static str,
		protocol: AwsProtocol,
	) -> Self {
		Self {
			endpoint_prefix,
			signing_name,
			protocol,
			global: None,
		}
	}

	/// DynamoDB.
	pub const DYNAMODB: Self =
		Self::regional("dynamodb", "dynamodb", AwsProtocol::AwsJson1_0 {
			target_prefix: "DynamoDB_20120810",
		});
	/// S3.
	pub const S3: Self = Self::regional("s3", "s3", AwsProtocol::RestXml);
	/// STS, at its regional endpoints.
	pub const STS: Self = Self::regional("sts", "sts", AwsProtocol::AwsQuery {
		version: "2011-06-15",
	});
	/// IAM, global: one host, signed for `us-east-1`.
	pub const IAM: Self = Self {
		endpoint_prefix: "iam",
		signing_name: "iam",
		protocol: AwsProtocol::AwsQuery {
			version: "2010-05-08",
		},
		global: Some(("iam.amazonaws.com", "us-east-1")),
	};
	/// Organizations, answered by the management account from `us-east-1`.
	pub const ORGANIZATIONS: Self = Self {
		endpoint_prefix: "organizations",
		signing_name: "organizations",
		protocol: AwsProtocol::AwsJson1_1 {
			target_prefix: "AWSOrganizationsV20161128",
		},
		global: Some(("organizations.us-east-1.amazonaws.com", "us-east-1")),
	};
	/// EC2.
	pub const EC2: Self = Self::regional("ec2", "ec2", AwsProtocol::Ec2Query {
		version: "2016-11-15",
	});
	/// Systems Manager, ie parameter store.
	pub const SSM: Self =
		Self::regional("ssm", "ssm", AwsProtocol::AwsJson1_1 {
			target_prefix: "AmazonSSM",
		});
	/// CloudWatch Logs.
	pub const LOGS: Self =
		Self::regional("logs", "logs", AwsProtocol::AwsJson1_1 {
			target_prefix: "Logs_20140328",
		});
	/// CloudWatch metrics, over the json protocol its model now prefers to
	/// its query one.
	pub const MONITORING: Self =
		Self::regional("monitoring", "monitoring", AwsProtocol::AwsJson1_0 {
			target_prefix: "GraniteServiceVersion20100801",
		});
	/// The container registry.
	pub const ECR: Self =
		Self::regional("api.ecr", "ecr", AwsProtocol::AwsJson1_1 {
			target_prefix: "AmazonEC2ContainerRegistry_V20150921",
		});
	/// SES v2.
	pub const SES: Self =
		Self::regional("email", "ses", AwsProtocol::RestJson1);
	/// Lambda.
	pub const LAMBDA: Self =
		Self::regional("lambda", "lambda", AwsProtocol::RestJson1);
	/// Recycle Bin.
	pub const RBIN: Self =
		Self::regional("rbin", "rbin", AwsProtocol::RestJson1);
	/// IAM Identity Center administration.
	pub const SSO_ADMIN: Self =
		Self::regional("sso", "sso", AwsProtocol::AwsJson1_1 {
			target_prefix: "SWBExternalService",
		});
	/// The Identity Center identity store.
	pub const IDENTITY_STORE: Self = Self::regional(
		"identitystore",
		"identitystore",
		AwsProtocol::AwsJson1_1 {
			target_prefix: "AWSIdentityStore",
		},
	);
	/// The AWS access portal, authenticated by an Identity Center token
	/// rather than a signature.
	pub const SSO_PORTAL: Self =
		Self::regional("portal.sso", "awsssoportal", AwsProtocol::RestJson1);
	/// Identity Center's OAuth service, unauthenticated.
	pub const SSO_OIDC: Self =
		Self::regional("oidc", "sso-oauth", AwsProtocol::RestJson1);

	/// The host this service answers `region` from.
	pub fn host(&self, region: &str) -> SmolStr {
		match self.global {
			Some((host, _)) => host.into(),
			None => format!("{}.{region}.amazonaws.com", self.endpoint_prefix)
				.into(),
		}
	}

	/// The region a request to this service is signed for: a global
	/// service's own, else the client's.
	pub fn signing_region<'a>(&self, region: &'a str) -> &'a str {
		match self.global {
			Some((_, region)) => region,
			None => region,
		}
	}
}
