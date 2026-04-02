use beet_core::prelude::*;

#[derive(Get, Component)]
pub struct TofuStack {
	/// The backend configuration for storing the state of this stack,
	/// defaults to a local backend at `./infra-state`
	backend: TofuBackend,
}


impl TofuStack {
	pub fn new(backend: impl Into<TofuBackend>) -> Self {
		Self {
			backend: backend.into(),
		}
	}
}

impl Default for TofuStack {
	fn default() -> Self { Self::new(S3Backend::default()) }
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

// impl TofuBackend {
// 	fn key(app_name: &str) -> String { format!("{app_name}--tofu-state") }
// }

#[derive(Serialize, Deserialize)]
pub struct LocalBackend {
	path: AbsPathBuf,
}
impl Default for LocalBackend {
	fn default() -> Self {
		Self {
			path: WsPathBuf::new(STATE_RESOURCE_NAME).into(),
		}
	}
}


const STATE_RESOURCE_NAME: &str = "tofu-state";

/// https://opentofu.org/docs/language/settings/backends/s3/
/// Use the s3 backend with a lockfile enabled via `use_lockfile`
#[derive(SetWith, Serialize, Deserialize)]
pub struct S3Backend {
	/// Directory in the bucket for the state, defaults to root `/`
	#[serde(rename = "key")]
	dir: AbsPathBuf,
}

impl Default for S3Backend {
	fn default() -> Self {
		Self {
			dir: AbsPathBuf::new_unchecked("/"),
		}
	}
}


#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize)]
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
