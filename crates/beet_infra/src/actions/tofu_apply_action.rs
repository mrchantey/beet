//! Tofu apply step for deploy sequences.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Builds terraform config, uploads artifacts, publishes the ledger, and applies.
///
/// Collects each [`BuildArtifact`] + [`ErasedBlock`] pair from
/// stack descendants to build the [`ArtifactLedger`], using
/// [`Block::artifact_label`] for the label and
/// [`BuildArtifact::compute_source_hash`] for the hash.
#[action]
#[derive(Default, Component)]
pub async fn TofuApplyAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	// step 1: build the project and collect artifact pairs
	let (project, stack, artifacts) = cx
		.caller
		.with_state::<(StackQuery, AncestorQuery<&Stack>), _>(
			|entity, (stack_query, anc_stack)| -> Result<_> {
				let project = stack_query.build_project(entity)?;
				let stack = anc_stack.get(entity)?.clone();
				let artifacts = stack_query.collect_artifacts(entity)?;
				(project, stack, artifacts).xok()
			},
		)
		.await?;

	// step 2: build ledger, upload artifacts to S3
	let mut ledger = stack.create_ledger();
	let client = stack.artifacts_client();
	client.ensure_bucket().await?;

	for (artifact, label) in &artifacts {
		let artifact_path = AbsPathBuf::new(artifact.artifact_path())?;
		let bytes = fs_ext::read_async(artifact_path.as_path()).await?;
		let source_hash = artifact.compute_source_hash()?;
		let s3_key = stack.artifact_key(label);

		client
			.upload_artifact(&ledger.deploy_id, label, bytes)
			.await?;
		info!(
			"uploaded artifact to s3://{}/{}",
			stack.artifact_bucket_name(),
			s3_key,
		);

		ledger.push_artifact(
			label.clone(),
			ArtifactEntry {
				s3_key: s3_key.into(),
				source_hash: source_hash.into(),
			},
		)?;
	}

	// step 3: publish ledger
	client
		.publish_ledger(&ledger)
		.await
		.map_err(|err| bevyhow!("failed to publish artifact ledger: {err}"))?;
	info!("published artifact ledger: {}", ledger.deploy_id);

	// step 4: apply terraform
	let result = project.apply().await?;
	info!("{result}");

	Pass(cx.input).xok()
}
