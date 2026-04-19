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
	// step 1: build the project and collect variables and artifact pairs
	let (project, stack, artifacts, variables) = cx
		.caller
		.with_state::<(StackQuery, AncestorQuery<&Stack>), _>(
			|entity, (stack_query, anc_stack)| -> Result<_> {
				let project = stack_query.build_project(entity)?;
				let stack = anc_stack.get(entity)?.clone();
				let artifacts = stack_query.collect_artifacts(entity)?;
				let variables = stack_query.collect_variables(entity)?;
				(project, stack, artifacts, variables).xok()
			},
		)
		.await?;

	// step 2: build ledger, upload artifacts to S3
	let mut client = stack.artifacts_client();
	client.ensure_bucket().await?;

	for (artifact, label) in &artifacts {
		let artifact_path = AbsPathBuf::new(artifact.artifact_path())?;
		let bytes = fs_ext::read_async(artifact_path.as_path()).await?;
		let source_hash = artifact.compute_source_hash()?;
		let artifact_key = stack.artifact_key(label);

		client
			.upload_artifact(label, bytes, ArtifactEntry {
				bucket_key: artifact_key.clone().into(),
				source_hash: source_hash.into(),
			})
			.await?;
		info!(
			"uploaded artifact to s3://{}/{}",
			stack.artifact_bucket_name(),
			artifact_key,
		);
	}

	// step 3: publish ledger
	client
		.publish_ledger()
		.await
		.map_err(|err| bevyhow!("failed to publish artifact ledger: {err}"))?;
	info!("published artifact ledger: {}", client.ledger().deploy_id);

	// step 4: resolve variables
	let resolved_vars: Vec<(SmolStr, SmolStr)> = variables
		.iter()
		.map(|variable| {
			variable
				.resolve_value(cx.input.parts())
				.map(|value| (variable.key().clone(), value))
		})
		.collect::<Result<Vec<_>>>()?;
	// step 5: apply terraform
	let result = project.apply_with_vars(&resolved_vars).await?;
	info!("{result}");

	Pass(cx.input).xok()
}
