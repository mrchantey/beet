#![allow(unused)]
use beet_core::prelude::*;

/// Name or part of a name for resources and other types.
/// All non-ascii alphanumeric characters are replaced with a `-`
/// with the following guarantees:
/// - only ascii alphanumeric and dashes
/// - adjacent dashes are collapsed
///
/// These guarantees allow for deterministic joining,
/// where each part is seperated by a double dash `--`
///
/// The parameter T is used for type safety, and for
/// type-specific extension methods.
pub struct Slug<T> {
	value: String,
	phantom: PhantomData<T>,
}

impl<T> std::ops::Deref for Slug<T> {
	type Target = String;
	fn deref(&self) -> &Self::Target { &self.value }
}
impl<T> Clone for Slug<T> {
	fn clone(&self) -> Self {
		Self {
			value: self.value.clone(),
			phantom: PhantomData,
		}
	}
}

impl<T> Into<Slug<T>> for String {
	fn into(self) -> Slug<T> { Slug::new(self) }
}
impl<T> Into<Slug<T>> for &str {
	fn into(self) -> Slug<T> { Slug::new(self) }
}

impl<T> std::fmt::Debug for Slug<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "Slug<{}>({})", std::any::type_name::<T>(), self.value)
	}
}

impl<T> std::fmt::Display for Slug<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.value)
	}
}

impl<T> Slug<T> {
	pub fn new(val: impl Into<String>) -> Self {
		val.into()
			.chars()
			.map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
			.collect::<String>()
			.split('-')
			.filter(|s| !s.is_empty())
			.collect::<Vec<_>>()
			.join("-")
			.xmap(|value| Slug {
				value,
				phantom: PhantomData,
			})
	}
	pub fn join(&self, other: &str) -> String {
		format!("{}--{}", self.value, other)
	}
}

/// Used as a paramter for slugs
pub struct Stage;
/// Used as a paramter for slugs
pub struct ResourceDef;

#[derive(Get)]
pub struct AppDef {
	/// The name for this application.
	/// This should be unique amongst your provider accounts,
	/// defaults to `CARGO_PKG_NAME`.
	slug: Slug<Self>,
	/// The backend configuration for storing the state of this stack,
	/// defaults to a local backend at `./infra-state`
	backend: TofuBackend,
}


/// A string where the
#[derive(Debug, Clone, Deref)]
pub struct AppName(String);
impl AppName {
	pub fn new(name: impl Into<String>) -> Self { Self(name.into()) }

	pub fn backend_slug(&self) -> String {
		self.0.to_lowercase().replace(' ', "-")
	}
}

impl AppDef {
	pub fn new(
		slug: impl Into<Slug<Self>>,
		backend: impl Into<TofuBackend>,
	) -> Self {
		Self {
			slug: slug.into(),
			backend: backend.into(),
		}
	}
}

impl Default for AppDef {
	fn default() -> Self {
		let app_name = env!("CARGO_PKG_NAME").to_string();
		let app_slug = Slug::new(app_name);
		let backend = S3Backend::new(app_slug.join("state"));
		Self::new(app_slug, backend)
	}
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
#[derive(Serialize, Deserialize)]
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
	pub fn new(bucket: impl Into<String>) -> Self {
		Self {
			bucket: bucket.into(),
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
