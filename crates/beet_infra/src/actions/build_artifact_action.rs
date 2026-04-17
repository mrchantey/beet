//! Build and optionally upload artifact step for deploy sequences.
#[allow(unused_imports)]
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Runs the build process from [`BuildArtifact`] on an ancestor entity.
/// If an [`ArtifactLedger`] and [`Stack`] are also present on ancestors,
/// uploads the result to the artifacts S3 bucket and registers it in the ledger.
/// Otherwise just runs the build.
#[action]
#[derive(Default, Component)]
pub async fn BuildArtifactAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	// step 1: read build artifact config from ancestor
	let build = cx
		.caller
		.with_state::<AncestorQuery<&BuildArtifact>, _>(|entity, query| {
			query.get(entity).cloned()
		})
		.await?;

	// step 2: run the build process
	info!("building: {}", build.process());
	build
		.process()
		.clone()
		.run_async()
		.await
		.map_err(|err| bevyhow!("build failed: {err}"))?;

	let artifact_path = AbsPathBuf::new(build.artifact_path())?;
	info!("artifact built: {}", artifact_path.display());

	// step 3: upload to S3 if an artifact ledger is present
	#[cfg(feature = "aws")]
	upload_to_ledger(&cx, &artifact_path).await?;

	Pass(cx.input).xok()
}

/// Read the built artifact, upload to S3, and register in the [`ArtifactLedger`].
/// Silently skips if no ledger ancestor is found.
#[cfg(feature = "aws")]
async fn upload_to_ledger(
	cx: &ActionContext<Request>,
	artifact_path: &AbsPathBuf,
) -> Result<()> {
	// check if a ledger ancestor exists; skip upload if not
	let ledger_info = cx
		.caller
		.with_state::<(AncestorQuery<&ArtifactLedger>, AncestorQuery<&Stack>), _>(
			|entity, (ledger_query, stack_query)| -> Option<_> {
				let ledger = ledger_query.get(entity).ok()?;
				let stack = stack_query.get(entity).ok()?;
				let client = stack.artifacts_client();
				let bucket_name = stack.artifact_bucket_name();
				Some((ledger.uuid, client, bucket_name))
			},
		)
		.await;

	let Some((ledger_uuid, stack_client, artifact_bucket_name)) = ledger_info else {
		return Ok(());
	};

	let bytes = fs_ext::read_async(artifact_path.as_path()).await?;
	info!("{} bytes", bytes.len());

	let source_hash = compute_source_hash(&bytes);

	stack_client.ensure_bucket().await?;
	let artifact_name = artifact_path
		.file_name()
		.map(|name| name.to_string_lossy().to_string())
		.unwrap_or_else(|| "artifact".to_string());
	let s3_key: SmolStr =
		format!("versions/{}/{}", ledger_uuid, artifact_name).into();
	stack_client
		.upload_artifact(&ledger_uuid, &artifact_name, bytes)
		.await?;
	info!(
		"uploaded artifact to s3://{}/{}",
		artifact_bucket_name, s3_key
	);

	// update the ledger with this artifact
	let entry = ArtifactEntry {
		s3_key: s3_key.clone(),
		source_hash: source_hash.into(),
	};
	let artifact_name_key: SmolStr = artifact_name.into();
	cx.caller
		.with_state::<AncestorQuery<&mut ArtifactLedger>, _>(
			move |entity, mut query| {
				if let Ok(mut ledger) = query.get_mut(entity) {
					ledger.push_artifact(artifact_name_key, entry);
				}
			},
		)
		.await;

	Ok(())
}

/// Compute a base64-encoded SHA256 hash of the given bytes,
/// matching Terraform's `filebase64sha256` function.
#[cfg(feature = "aws")]
fn compute_source_hash(bytes: &[u8]) -> String {
	use base64::Engine;
	use sha2::Digest;
	let hash = sha2::Sha256::digest(bytes);
	base64::engine::general_purpose::STANDARD.encode(hash)
}
