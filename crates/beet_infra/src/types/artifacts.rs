//! Versioned artifact storage for deploy, rollback, and rollforward.
#[cfg(feature = "bindings_aws_common")]
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use bytes::Bytes;

/// Versioned artifact storage backed by an S3 bucket.
/// Provisions the bucket via the required [`S3BucketBlock`].
#[cfg(feature = "bindings_aws_common")]
#[derive(Debug, Clone, Component)]
#[require(S3BucketBlock = S3BucketBlock::new("artifacts").with_output(false))]
pub struct ArtifactsBucket;

/// Runtime client for artifact operations on a provisioned bucket.
#[derive(Debug, Clone)]
pub struct ArtifactsClient {
	bucket: Bucket,
}

/// Metadata about a deployed version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactLedger {
	pub uuid: Uuid,
	pub artifacts: Vec<SmolStr>,
	pub deployed_at: String,
}

/// Metadata about a single artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactMeta {
	pub uuid: Uuid,
	pub hash: u64,
	pub s3_key: String,
	pub deployed_at: String,
}

impl ArtifactsClient {
	pub fn new(bucket: Bucket) -> Self {
		Self { bucket }
	}

	pub async fn upload(
		&self,
		artifact_name: &str,
		bytes: impl Into<Bytes>,
	) -> Result<Uuid> {
		let bytes: Bytes = bytes.into();
		let uuid = Uuid::now_v7();
		let hash = fs_ext::hash_bytes(&bytes);
		let timestamp = now_timestamp();
		// store binary artifact
		let binary_key = version_artifact_key(&uuid, artifact_name);
		self.bucket.insert(&binary_key, bytes).await?;
		// store per-artifact metadata
		let meta = ArtifactMeta {
			uuid,
			hash,
			s3_key: binary_key.to_string(),
			deployed_at: timestamp.clone(),
		};
		let meta_key = version_meta_key(&uuid, artifact_name);
		self.bucket
			.insert(&meta_key, serde_json::to_vec_pretty(&meta)?)
			.await?;
		// store version ledger
		let ledger = ArtifactLedger {
			uuid,
			artifacts: vec![artifact_name.into()],
			deployed_at: timestamp,
		};
		let ledger_key = version_ledger_key(&uuid);
		self.bucket
			.insert(&ledger_key, serde_json::to_vec_pretty(&ledger)?)
			.await?;
		Ok(uuid)
	}

	pub async fn download(
		&self,
		artifact_name: &str,
		version: &Uuid,
	) -> Result<Bytes> {
		let key = version_artifact_key(version, artifact_name);
		self.bucket.get(&key).await
	}

	pub async fn current_version(&self) -> Result<Option<Uuid>> {
		let key = current_ledger_key();
		if !self.bucket.exists(&key).await? {
			return Ok(None);
		}
		let bytes = self.bucket.get(&key).await?;
		let ledger: ArtifactLedger = serde_json::from_slice(&bytes)?;
		Ok(Some(ledger.uuid))
	}

	pub async fn set_current(&self, version: &Uuid) -> Result {
		// copy version ledger to current
		let ledger_bytes = self
			.bucket
			.get(&version_ledger_key(version))
			.await?;
		self.bucket
			.insert(&current_ledger_key(), ledger_bytes.clone())
			.await?;
		// copy each artifact metadata to current
		let ledger: ArtifactLedger =
			serde_json::from_slice(&ledger_bytes)?;
		for artifact_name in &ledger.artifacts {
			let src_meta = self
				.bucket
				.get(&version_meta_key(version, artifact_name))
				.await?;
			self.bucket
				.insert(&current_meta_key(artifact_name), src_meta)
				.await?;
		}
		Ok(())
	}

	pub async fn list_versions(&self) -> Result<Vec<Uuid>> {
		let all_keys = self.bucket.list().await?;
		let mut versions: Vec<Uuid> = all_keys
			.iter()
			.filter_map(|key| {
				let key_str = key.to_string();
				let rest = key_str.strip_prefix("versions/")?;
				let uuid_str = rest.strip_suffix("/ledger.json")?;
				Uuid::parse_str(uuid_str).ok()
			})
			.collect();
		versions.sort();
		Ok(versions)
	}

	pub async fn list_artifacts(
		&self,
		version: &Uuid,
	) -> Result<Vec<SmolStr>> {
		let ledger_bytes =
			self.bucket.get(&version_ledger_key(version)).await?;
		let ledger: ArtifactLedger =
			serde_json::from_slice(&ledger_bytes)?;
		Ok(ledger.artifacts)
	}

	pub async fn rollback(&self, count: usize) -> Result<Uuid> {
		let versions = self.list_versions().await?;
		let current = self
			.current_version()
			.await?
			.ok_or_else(|| bevyhow!("no current version to rollback from"))?;
		let current_idx = versions
			.iter()
			.position(|ver| *ver == current)
			.ok_or_else(|| {
				bevyhow!("current version not found in version list")
			})?;
		let target_idx =
			current_idx.checked_sub(count).ok_or_else(|| {
				bevyhow!(
					"cannot rollback {count} versions, only {current_idx} available"
				)
			})?;
		let target = versions[target_idx];
		self.set_current(&target).await?;
		Ok(target)
	}

	pub async fn rollforward(&self) -> Result<Uuid> {
		let versions = self.list_versions().await?;
		let target = versions
			.last()
			.ok_or_else(|| {
				bevyhow!("no versions available to rollforward to")
			})?;
		self.set_current(target).await?;
		Ok(*target)
	}
}

fn current_ledger_key() -> RelPath {
	RelPath::new("current/ledger.json")
}
fn current_meta_key(artifact_name: &str) -> RelPath {
	RelPath::new(format!("current/{artifact_name}.json"))
}
fn version_ledger_key(uuid: &Uuid) -> RelPath {
	RelPath::new(format!("versions/{uuid}/ledger.json"))
}
fn version_meta_key(uuid: &Uuid, artifact_name: &str) -> RelPath {
	RelPath::new(format!("versions/{uuid}/{artifact_name}.json"))
}
fn version_artifact_key(uuid: &Uuid, artifact_name: &str) -> RelPath {
	RelPath::new(format!("versions/{uuid}/{artifact_name}"))
}

fn now_timestamp() -> String {
	let duration = SystemTime::now()
		.duration_since(SystemTime::UNIX_EPOCH)
		.unwrap_or_default();
	format!("{}s", duration.as_secs())
}


#[cfg(test)]
mod tests {
	use super::*;

	#[beet_core::test]
	async fn upload_and_download() {
		let client = ArtifactsClient::new(temp_bucket());
		let bytes = b"hello world".to_vec();
		let version =
			client.upload("my-app.zip", bytes.clone()).await.unwrap();
		let downloaded =
			client.download("my-app.zip", &version).await.unwrap();
		downloaded.to_vec().xpect_eq(bytes);
	}

	#[beet_core::test]
	async fn current_version_lifecycle() {
		let client = ArtifactsClient::new(temp_bucket());
		client.current_version().await.unwrap().xpect_eq(None);
		let ver1 =
			client.upload("app.zip", b"v1".to_vec()).await.unwrap();
		client.set_current(&ver1).await.unwrap();
		client
			.current_version()
			.await
			.unwrap()
			.xpect_eq(Some(ver1));
		let ver2 =
			client.upload("app.zip", b"v2".to_vec()).await.unwrap();
		client.set_current(&ver2).await.unwrap();
		client
			.current_version()
			.await
			.unwrap()
			.xpect_eq(Some(ver2));
	}

	#[beet_core::test]
	async fn list_versions_sorted() {
		let client = ArtifactsClient::new(temp_bucket());
		let ver1 =
			client.upload("app.zip", b"v1".to_vec()).await.unwrap();
		let ver2 =
			client.upload("app.zip", b"v2".to_vec()).await.unwrap();
		let versions = client.list_versions().await.unwrap();
		versions.xpect_eq(vec![ver1, ver2]);
	}

	#[beet_core::test]
	async fn rollback_and_rollforward() {
		let client = ArtifactsClient::new(temp_bucket());
		let ver1 =
			client.upload("app.zip", b"v1".to_vec()).await.unwrap();
		let ver2 =
			client.upload("app.zip", b"v2".to_vec()).await.unwrap();
		let ver3 =
			client.upload("app.zip", b"v3".to_vec()).await.unwrap();
		client.set_current(&ver3).await.unwrap();
		let rolled = client.rollback(1).await.unwrap();
		rolled.xpect_eq(ver2);
		client
			.current_version()
			.await
			.unwrap()
			.xpect_eq(Some(ver2));
		let rolled = client.rollback(1).await.unwrap();
		rolled.xpect_eq(ver1);
		let forwarded = client.rollforward().await.unwrap();
		forwarded.xpect_eq(ver3);
		client
			.current_version()
			.await
			.unwrap()
			.xpect_eq(Some(ver3));
	}

	#[beet_core::test]
	async fn rollback_too_far_fails() {
		let client = ArtifactsClient::new(temp_bucket());
		let ver1 =
			client.upload("app.zip", b"v1".to_vec()).await.unwrap();
		client.set_current(&ver1).await.unwrap();
		client.rollback(1).await.unwrap_err();
	}

	#[beet_core::test]
	async fn list_artifacts_for_version() {
		let client = ArtifactsClient::new(temp_bucket());
		let version =
			client.upload("my-app.zip", b"data".to_vec()).await.unwrap();
		let artifacts =
			client.list_artifacts(&version).await.unwrap();
		artifacts.xpect_eq(vec![SmolStr::from("my-app.zip")]);
	}
}
