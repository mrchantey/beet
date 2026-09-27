use crate::bindings::aws;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Strategy for maintaining the Terraform state for this stack.
/// By default, the state for each stack is
/// stored in an individual directory in a shared state bucket.
/// https://opentofu.org/docs/language/settings/backends/configuration/
#[derive(Debug, Clone, PartialEq, Eq)]
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
	/// The body of the opentofu "backend" field, or `None` for a backend this
	/// launch has not resolved yet, see [`resolved`](Self::resolved). A verb
	/// resolves before it drives tofu and re-renders the block then; one that
	/// somehow did not is refused by `Project::init`, since a config with no
	/// backend block is a config that keeps its state in the work directory.
	pub fn to_json(&self, key: &str) -> Option<Value> {
		match self {
			Self::Local(b) => b.to_json(&key).xsome(),
			Self::S3(b) => b.to_json(&key),
		}
	}

	/// Whether both halves of this backend are known, see
	/// [`resolved`](Self::resolved).
	pub fn is_resolved(&self) -> bool {
		match self {
			Self::Local(_) => true,
			Self::S3(s3) => s3.is_resolved(),
		}
	}

	/// The uri of the state store itself: the local state directory, or the
	/// state bucket in its region.
	pub fn uri(&self) -> Result<StoreUri> {
		match self {
			Self::Local(local) => local.uri().xok(),
			Self::S3(s3) => s3.uri(),
		}
	}

	/// This backend with whatever the launch was not told filled in, see
	/// [`S3Backend::resolved`]. A local backend, and an S3 one told both
	/// halves, resolve to themselves without a round trip.
	pub async fn resolved(&self) -> Result<Self> {
		match self {
			Self::Local(_) => self.clone().xok(),
			Self::S3(s3) => s3.resolved().await.map(Self::S3),
		}
	}

	/// The state store, see [`uri`](Self::uri). Errors without a compiled
	/// backend for it, ie an S3 state backend in an `aws_sdk`-free build.
	pub fn store(&self) -> Result<BlobStore> {
		BlobStore::from_uri(&self.uri()?)
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
			S3Store::from_uri(&s3.uri()?)?
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
#[derive(Debug, Clone, PartialEq, Eq, Get)]
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

/// S3 backend for remote state storage.
/// https://opentofu.org/docs/language/settings/backends/s3/
///
/// One bucket for every app and stage, a key each, created on first use by
/// [`StackBackend::ensure_exists`] with object versioning on, so a machine with
/// credentials and nothing else deploys without provisioning a backend by hand
/// first, and the state it writes has an undo from the start.
///
/// Both halves are optional because neither is normally authored: see
/// [`resolved`](Self::resolved), which derives the bucket from the account and
/// reads the region off the bucket, so one account's projects share a state
/// bucket that none of them names.
#[derive(Debug, Clone, PartialEq, Eq, Get, SetWith)]
pub struct S3Backend {
	/// The bucket holding every stack's state file. Unset is this account's
	/// own, [`DEFAULT_STATE_PREFIX`] and its id, filled in by
	/// [`resolved`](Self::resolved).
	bucket: Option<SmolStr>,
	/// The region the bucket is in. Unset is read off the bucket itself, which
	/// is what lets one project find the state bucket another project of the
	/// same account created; a bucket that does not exist yet is created in the
	/// declared region, else [`DEFAULT_STATE_REGION`].
	region: Option<SmolStr>,
	/// Enable OpenTofu's native S3 lockfile.
	use_lockfile: bool,
}

impl Default for S3Backend {
	fn default() -> Self {
		Self {
			// this account's own bucket, wherever it already is: see `resolved`
			bucket: None,
			region: None,
			use_lockfile: true,
		}
	}
}

impl S3Backend {
	/// The state bucket, pinned to its region, as a store uri. Errors on a
	/// backend nothing resolved, see [`resolved`](Self::resolved).
	pub fn uri(&self) -> Result<StoreUri> {
		let (bucket, region) = self.resolved_parts()?;
		StoreUri::S3 {
			name: bucket,
			path_prefix: None,
			endpoint: None,
			region: Some(region),
		}
		.xok()
	}

	/// This backend with whatever the launch was not told filled in: the bucket
	/// derived from the AWS account (see [`DEFAULT_STATE_PREFIX`]) and the
	/// region read off that bucket. Costs one `sts:GetCallerIdentity` per
	/// process and one `GetBucketLocation` per call, and nothing at all once
	/// both halves are known, which is how a launch resolves once and every
	/// project built afterwards renders the same backend.
	///
	/// A declared region is where a bucket that does not exist yet is CREATED.
	/// Once it exists the bucket is the fact, so a declaration that disagrees
	/// warns and loses: the alternative is addressing a region the state is not
	/// in, which fails every verb.
	pub async fn resolved(&self) -> Result<Self> {
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
				let region = located
					.or_else(|| self.region.clone())
					.unwrap_or_else(|| DEFAULT_STATE_REGION.into());
				Self {
					bucket: Some(bucket),
					region: Some(region),
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

	/// The two halves a render needs, or the error that says nothing has
	/// resolved them yet.
	fn resolved_parts(&self) -> Result<(SmolStr, SmolStr)> {
		match (&self.bucket, &self.region) {
			(Some(bucket), Some(region)) => {
				(bucket.clone(), region.clone()).xok()
			}
			_ => bevybail!(
				"the state backend is unresolved: its bucket is this account's \
				own and its region is wherever that bucket is, both read by the \
				first verb of a launch (`Project::resolved`)"
			),
		}
	}

	/// Whether both halves are known, ie nothing is left to discover.
	pub fn is_resolved(&self) -> bool {
		self.bucket.is_some() && self.region.is_some()
	}

	fn to_json(&self, key: &str) -> Option<Value> {
		let (bucket, region) = self.resolved_parts().ok()?;
		value!({
			"s3": {
				"bucket": bucket,
				"key": key,
				"region": region,
				"use_lockfile": (self.use_lockfile),
			}
		})
		.xsome()
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
