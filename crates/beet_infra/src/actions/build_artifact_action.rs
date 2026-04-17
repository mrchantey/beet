//! Build and upload artifact step for deploy sequences.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Runs the build process from [`BuildArtifact`] on an ancestor entity,
/// then uploads the result to the artifacts S3 bucket and registers it
/// in the [`ArtifactLedger`].
/// Errors if no [`ArtifactLedger`] or [`Stack`] ancestor is found,
/// as a ledger is the only way to deploy an artifact.
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

	// step 3: upload to S3 and register in the ledger
	upload_to_ledger(&cx, &build, &artifact_path).await?;

	Pass(cx.input).xok()
}

/// Read the built artifact, upload to S3, and register in the [`ArtifactLedger`].
/// Errors if no ledger or stack ancestor is found.
async fn upload_to_ledger(
	cx: &ActionContext<Request>,
	build: &BuildArtifact,
	artifact_path: &AbsPathBuf,
) -> Result<()> {
	// require both a ledger and stack ancestor
	let (ledger_uuid, stack_client, artifact_bucket_name) = cx
		.caller
		.with_state::<(AncestorQuery<&ArtifactLedger>, AncestorQuery<&Stack>), _>(
			|entity, (ledger_query, stack_query)| -> Result<_> {
				let ledger = ledger_query.get(entity)
					.map_err(|_| bevyhow!("no ArtifactLedger ancestor found, a ledger is required to build artifacts"))?;
				let stack = stack_query.get(entity)
					.map_err(|_| bevyhow!("no Stack ancestor found, a stack is required to build artifacts"))?;
				let client = stack.artifacts_client();
				let bucket_name = stack.artifact_bucket_name();
				(ledger.uuid, client, bucket_name).xok()
			},
		)
		.await?;

	// read the built artifact
	let bytes = fs_ext::read_async(artifact_path.as_path()).await?;
	info!("{} bytes", bytes.len());

	let source_hash = compute_source_hash(&bytes);

	// ensure the artifact bucket exists and upload
	stack_client.ensure_bucket().await?;
	let artifact_label = build.label();
	let s3_key: SmolStr =
		format!("versions/{}/{}", ledger_uuid, artifact_label).into();
	stack_client
		.upload_artifact(&ledger_uuid, artifact_label, bytes)
		.await?;
	info!(
		"uploaded artifact to s3://{}/{}",
		artifact_bucket_name, s3_key
	);

	// update the ledger with this artifact, using the BuildArtifact label as key
	let entry = ArtifactEntry {
		s3_key: s3_key.clone(),
		source_hash: source_hash.into(),
	};
	let label: SmolStr = artifact_label.clone();
	cx.caller
		.with_state::<AncestorQuery<&mut ArtifactLedger>, Result>(
			move |entity, mut query| {
				let mut ledger = query.get_mut(entity)?;
				ledger.push_artifact(label, entry)
			},
		)
		.await
}

/// Compute a base64-encoded SHA256 hash of the given bytes,
/// matching Terraform's `filebase64sha256` function.
fn compute_source_hash(bytes: &[u8]) -> String {
	cfg_if! {
		if #[cfg(feature = "aws")] {
			use base64::Engine;
			use sha2::Digest;
			let hash = sha2::Sha256::digest(bytes);
			base64::engine::general_purpose::STANDARD.encode(hash)
		} else {
			let _ = bytes;
			panic!("the `aws` feature is required for artifact hash computation")
		}
	}
}
