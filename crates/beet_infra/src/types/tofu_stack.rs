use beet_core::prelude::*;


pub struct TofuStack {
	app_name: String,
	backend: TofuBackend,
}

impl TofuStack {
	pub fn new(app_name: String, backend: impl Into<TofuBackend>) -> Self {
		Self {
			app_name,
			backend: backend.into(),
		}
	}
	pub fn new_s3() {}
}


/// https://opentofu.org/docs/language/settings/backends/configuration/
pub enum TofuBackend {
	Local(LocalBackend),
	S3(S3Backend),
}
impl Into<TofuBackend> for LocalBackend {
	fn into(self) -> TofuBackend { TofuBackend::Local(self) }
}
impl Into<TofuBackend> for S3Backend {
	fn into(self) -> TofuBackend { TofuBackend::S3(self) }
}

#[derive(Serialize, Deserialize)]
pub struct LocalBackend {
	path: AbsPathBuf,
}
impl Default for LocalBackend {
	fn default() -> Self {
		Self {
			path: WsPathBuf::new("infra-state").into(),
		}
	}
}


/// https://opentofu.org/docs/language/settings/backends/s3/
/// Use the s3 backend with a lockfile enabled via `use_lockfile`
#[derive(Default, Serialize, Deserialize)]
pub struct S3Backend {
	/// Optionally specify the bucket name,
	/// defaults to `{TofuStack::app_name}--backend`
	bucket: String,
	/// Directory in the bucket for the state
	#[serde(rename = "key")]
	dir: AbsPathBuf,
	/// AWS region where the bucket is located, defaults to `us-east-1`
	region: AwsRegion,
}
impl S3Backend {
	pub fn new(bucket: &str) -> Self {
		Self {
			bucket: bucket.to_string(),
			dir: AbsPathBuf::new_unchecked("/"),
			region: AwsRegion::default(),
		}
	}
}


#[derive(Default, Serialize, Deserialize)]
pub enum AwsRegion {
	#[default]
	#[serde(rename = "us-east-1")]
	UsEast1,
	#[serde(rename = "us-west-2")]
	UsWest2,
}

impl std::fmt::Display for AwsRegion {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			AwsRegion::UsEast1 => write!(f, "us-east-1"),
			AwsRegion::UsWest2 => write!(f, "us-west-2"),
		}
	}
}
