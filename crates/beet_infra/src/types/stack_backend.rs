use crate::bindings::aws;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// How this launch keeps the tofu state of every stack it declares, as
/// DECLARED: an entry's authoring plus whatever the default fills in, which is
/// not yet enough to address anything. [`resolve`](Self::resolve) turns it into
/// a [`ResolvedBackend`], the only form that renders or addresses, exactly as
/// [`Stack`](crate::prelude::Stack) resolves into
/// [`ResolvedStack`](crate::prelude::ResolvedStack) and for the same reason: a
/// declaration is allowed to leave things out.
/// https://opentofu.org/docs/language/settings/backends/configuration/
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StackBackend {
	Local(LocalBackend),
	S3(S3Backend),
}

impl Default for StackBackend {
	fn default() -> Self {
		cfg_if! {
			// the S3 backend needs the native SDK (`S3Store` is native-only), so a
			// wasm build defaults to local state exactly as an sdk-free one does.
			if #[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))] {
				StackBackend::S3(S3Backend::default())
			} else {
				StackBackend::Local(LocalBackend::default())
			}
		}
	}
}

impl From<LocalBackend> for StackBackend {
	fn from(b: LocalBackend) -> Self { StackBackend::Local(b) }
}
impl From<S3Backend> for StackBackend {
	fn from(b: S3Backend) -> Self { StackBackend::S3(b) }
}
impl StackBackend {
	/// This declaration with everything it left out filled in, ready to render
	/// and address: see [`S3Backend::resolve`]. Memoized per declaration for
	/// the life of the process, so a launch pays for the discovery once however
	/// many stacks and verbs ask.
	pub async fn resolve(&self) -> Result<ResolvedBackend> {
		static RESOLVED: LazyPool<
			StackBackend,
			ResolvedBackend,
			Result<ResolvedBackend>,
		> = LazyPool::new(|backend| {
			let backend = backend.clone();
			Box::pin(async move {
				match backend {
					StackBackend::Local(local) => {
						ResolvedBackend::Local(local).xok()
					}
					StackBackend::S3(s3) => {
						s3.resolve().await.map(ResolvedBackend::S3)
					}
				}
			})
		});
		RESOLVED.try_get(self).await
	}
}

/// A [`StackBackend`] with nothing left to discover: a state directory, or a
/// bucket AND the region it is in. The only form that renders a backend block
/// or addresses a state store, so holding one is proof the discovery ran, and
/// every operation below is total.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedBackend {
	Local(LocalBackend),
	S3(ResolvedS3Backend),
}

impl From<LocalBackend> for ResolvedBackend {
	fn from(b: LocalBackend) -> Self { ResolvedBackend::Local(b) }
}
impl From<ResolvedS3Backend> for ResolvedBackend {
	fn from(b: ResolvedS3Backend) -> Self { ResolvedBackend::S3(b) }
}

impl ResolvedBackend {
	/// Create the body of the opentofu "backend" field.
	pub fn to_json(&self, key: &str) -> Value {
		match self {
			Self::Local(b) => b.to_json(key),
			Self::S3(b) => b.to_json(key),
		}
	}

	/// The uri of the state store itself: the local state directory, or the
	/// state bucket in its region.
	pub fn uri(&self) -> StoreUri {
		match self {
			Self::Local(local) => local.uri(),
			Self::S3(s3) => s3.uri(),
		}
	}

	/// The state store, see [`uri`](Self::uri). Errors without a compiled
	/// backend for it, ie an S3 state backend in an `aws_sdk`-free build.
	pub fn store(&self) -> Result<BlobStore> {
		BlobStore::from_uri(&self.uri())
	}

	/// Ensure the backend exists, creating the directory or s3 bucket if it
	/// doesn't exist; a bucket it creates is created with object versioning
	/// on, see below.
	pub async fn ensure_exists(&self) -> Result {
		let store = self.store()?;
		if store.store_exists().await? {
			return Ok(());
		}
		store.store_create().await?;
		// every apply overwrites the state object in place, so a version is its
		// only undo: a bucket born here is born versioned, which is what makes a
		// fresh account's first deploy as recoverable as an established one's.
		// Only at creation, so an existing bucket's settings stay its owner's.
		#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
		if let Self::S3(s3) = self {
			S3Store::from_uri(&s3.uri())?
				.set_object_versioning()
				.await?;
		}
		Ok(())
	}

	/// Clear stale lock files if the backend supports it.
	pub fn clear_stale_locks(&self) {
		match self {
			Self::Local(local) => local.clear_stale_locks(),
			Self::S3(_) => {
				// S3 lock management is handled server-side by OpenTofu's native lockfile mechanism
			}
		}
	}

	/// Remove this backend bucket if its empty
	pub async fn remove_if_empty(&self) -> Result {
		let store = self.store()?;
		if store.store_is_empty().await? {
			store.store_remove().await?;
		}
		Ok(())
	}
}

/// Local filesystem backend, defaults to `.beet/infra`
/// https://opentofu.org/docs/language/settings/backends/local/
#[derive(Debug, Clone, PartialEq, Eq, Hash, Get)]
pub struct LocalBackend {
	/// The path on the local filesystem where the state file will be stored, defaults to `.beet/infra`.
	path: AbsPath,
}

impl Default for LocalBackend {
	fn default() -> Self {
		Self {
			path: WsPath::new(".beet/infra").into(),
		}
	}
}
impl LocalBackend {
	/// A state directory of the caller's choosing: a test's own, so its
	/// state never meets another's under the shared default.
	pub fn new(path: impl Into<AbsPath>) -> Self { Self { path: path.into() } }

	fn to_json(&self, key: &str) -> Value {
		// Use the absolute path string directly. AbsPath's Serialize impl
		// converts to a workspace-relative path, but terraform's local backend
		// resolves relative paths from the tofu working directory, not the
		// workspace root.
		let state_path = self.path.join(key).to_string();
		value!({ "local": { "path": state_path } })
	}
	/// The state directory as a store uri.
	pub fn uri(&self) -> StoreUri {
		StoreUri::Fs {
			path_prefix: Some(self.path.clone().into()),
		}
	}
	/// Remove stale `.*.lock.info` files left by interrupted tofu processes.
	pub fn clear_stale_locks(&self) {
		if let Ok(entries) =
			std::fs::read_dir(self.path.as_ref() as &std::path::Path)
		{
			for entry in entries.flatten() {
				let name = entry.file_name();
				let name = name.to_string_lossy();
				if name.starts_with('.') && name.ends_with(".lock.info") {
					std::fs::remove_file(entry.path()).ok();
				}
			}
		}
	}
}

/// The default state bucket is `beet-state-` and the AWS account id, which is
/// unique by construction where a fixed name is not: an S3 bucket name is
/// global, so one account holding `beet-state` would leave every other account
/// naming something else. Derived rather than declared, so the second project a
/// user deploys finds the first one's state bucket with nothing authored.
const DEFAULT_STATE_PREFIX: &str = "beet-state-";

/// Where a state bucket is created when nothing says otherwise, and the
/// endpoint a discovery asks: `sts:GetCallerIdentity` and `GetBucketLocation`
/// both answer here for an account and a bucket anywhere.
const DEFAULT_STATE_REGION: &str = aws::region::US_EAST_1;

/// The S3 state backend as DECLARED: a bucket and a region, each optional
/// because neither is normally authored.
/// https://opentofu.org/docs/language/settings/backends/s3/
///
/// [`resolve`](Self::resolve) derives the bucket from the AWS account and reads
/// the region off that bucket, so every project one account deploys shares a
/// state bucket that none of them names, with a key each
/// (`<app>--<stage>--tofu-tfstate`). The bucket is created on first use with
/// object versioning on, so a machine with credentials and nothing else deploys
/// without provisioning a backend by hand, and the state it writes has an undo
/// from the start.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Get, SetWith)]
pub struct S3Backend {
	/// The bucket holding every stack's state file. Unset is this account's
	/// own, [`DEFAULT_STATE_PREFIX`] and its id.
	bucket: Option<SmolStr>,
	/// Where the bucket is. Unset is read off the bucket itself, which is what
	/// lets one project find the state bucket another project of the same
	/// account created; a bucket that does not exist yet is created in the
	/// declared region, else [`DEFAULT_STATE_REGION`].
	region: Option<SmolStr>,
	/// Enable OpenTofu's native S3 lockfile.
	use_lockfile: bool,
}

impl Default for S3Backend {
	fn default() -> Self {
		Self {
			// this account's own bucket, wherever it already is: see `resolve`
			bucket: None,
			region: None,
			use_lockfile: true,
		}
	}
}

impl S3Backend {
	/// This declaration with both halves known: the bucket derived from the AWS
	/// account (see [`DEFAULT_STATE_PREFIX`]) when it was not named, and the
	/// region read off that bucket. One `sts:GetCallerIdentity` per process and
	/// one `GetBucketLocation` per declaration, both behind
	/// [`StackBackend::resolve`]'s memo.
	///
	/// A declared region is where a bucket that does not exist yet is CREATED.
	/// Once it exists the bucket is the fact, so a declaration that disagrees
	/// warns and loses: the alternative is addressing a region the state is not
	/// in, which fails every verb.
	pub async fn resolve(&self) -> Result<ResolvedS3Backend> {
		cfg_if! {
			if #[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))] {
				let bucket = match &self.bucket {
					Some(bucket) => bucket.clone(),
					None => format!(
						"{DEFAULT_STATE_PREFIX}{}",
						beet_net::prelude::aws_ext::account_id().await?
					)
					.into(),
				};
				let located = S3Store::from_uri(&StoreUri::S3 {
					name: bucket.clone(),
					path_prefix: None,
					endpoint: None,
					region: Some(DEFAULT_STATE_REGION.into()),
				})?
				.bucket_region()
				.await?;
				if let (Some(located), Some(declared)) = (&located, &self.region)
					&& located != declared
				{
					warn!(
						"state bucket `{bucket}` is in {located}, not the \
						declared {declared}; using {located}, since the bucket \
						is where it is"
					);
				}
				ResolvedS3Backend {
					bucket,
					region: located
						.or_else(|| self.region.clone())
						.unwrap_or_else(|| DEFAULT_STATE_REGION.into()),
					use_lockfile: self.use_lockfile,
				}
				.xok()
			} else {
				bevybail!(
					"cannot resolve the S3 state backend: this build has no aws \
					sdk. Declare a local backend, or build with `aws_sdk`."
				)
			}
		}
	}
}

/// An [`S3Backend`] that knows its bucket and the region that bucket is in,
/// which is what tofu's backend block and every state read need.
#[derive(Debug, Clone, PartialEq, Eq, Get)]
pub struct ResolvedS3Backend {
	bucket: SmolStr,
	region: SmolStr,
	use_lockfile: bool,
}

impl ResolvedS3Backend {
	/// The state bucket, pinned to its region, as a store uri.
	pub fn uri(&self) -> StoreUri {
		StoreUri::S3 {
			name: self.bucket.clone(),
			path_prefix: None,
			endpoint: None,
			region: Some(self.region.clone()),
		}
	}

	fn to_json(&self, key: &str) -> Value {
		value!({
			"s3": {
				"bucket": (self.bucket.clone()),
				"key": key,
				"region": (self.region.clone()),
				"use_lockfile": (self.use_lockfile),
			}
		})
	}
}

#[cfg(feature = "deploy")]
use crate::prelude::Deployment;

/// `<S3StateBackend region="ap-southeast-2"/>` — where this launch keeps every
/// stack's tofu state, for the two cases the default cannot know.
///
/// Authoring nothing is the normal case: the bucket is this account's own and
/// it is found wherever it already is (see [`S3Backend::resolved`]), so every
/// project a user deploys shares one state bucket having declared none of it.
/// `region` is the one-time choice of where that bucket is CREATED, worth
/// making in the first project a user deploys and worth repeating in none of
/// them. `bucket` names a shared bucket instead, which every project sharing it
/// must then declare.
///
/// The backend is a property of the LAUNCH rather than of a stack's identity,
/// so it lands on the process [`Deployment`] and is authored once per entry,
/// outside every `<Stack>`, beside the credentials the same launch loads.
///
/// Deploy-only, like the verbs it configures: a runtime build never reads a
/// state backend, so an entry that also builds lean authors this under the same
/// `bx:cfg` as its `<DeployRoutes>`.
#[cfg(feature = "deploy")]
#[template(system)]
pub fn S3StateBackend(
	/// A bucket of this entry's own, which every project sharing it declares.
	/// Unset is this account's own bucket, which no project declares.
	#[prop]
	bucket: Option<String>,
	/// Where to CREATE the bucket, when there is not one yet. An existing
	/// bucket is found wherever it is, so this is the one-time choice a first
	/// project makes and every later project inherits without repeating it.
	#[prop]
	region: Option<String>,
	mut deployment: ResMut<Deployment>,
) {
	deployment.set_backend(
		S3Backend::default()
			.with_bucket(bucket.map(SmolStr::from))
			.with_region(region.map(SmolStr::from))
			.into(),
	);
}
