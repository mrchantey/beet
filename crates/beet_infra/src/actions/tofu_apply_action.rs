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
	trace!("TofuApplyAction: starting");
	// step 1: build the project and collect variables and artifact pairs
	trace!(
		"TofuApplyAction: step 1 - building project and collecting artifacts"
	);
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
	trace!(
		"TofuApplyAction: collected {} artifacts, {} variables",
		artifacts.len(),
		variables.len()
	);

	// step 2: build ledger, upload artifacts to S3
	trace!("TofuApplyAction: step 2 - ensuring artifacts bucket exists");
	let mut client = stack.artifacts_client();
	client.ensure_bucket().await?;
	trace!("TofuApplyAction: artifacts bucket ready");

	trace!("TofuApplyAction: uploading {} artifacts", artifacts.len());
	for (artifact, label) in &artifacts {
		trace!("TofuApplyAction: uploading artifact '{}'", label);
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
	trace!("TofuApplyAction: step 3 - publishing artifact ledger");
	client
		.publish_ledger()
		.await
		.map_err(|err| bevyhow!("failed to publish artifact ledger: {err}"))?;
	trace!(
		"TofuApplyAction: published artifact ledger: {}",
		client.ledger().deploy_id
	);

	// step 4: resolve variables
	trace!(
		"TofuApplyAction: step 4 - resolving {} variables",
		variables.len()
	);
	let resolved_vars: Vec<(SmolStr, SmolStr)> = variables
		.iter()
		.map(|variable| {
			variable
				.resolve_value(cx.input.parts())
				.map(|value| (variable.key().clone(), value))
		})
		.collect::<Result<Vec<_>>>()?;
	trace!("TofuApplyAction: resolved variables: {:?}", resolved_vars);
	// step 5: apply terraform
	trace!("TofuApplyAction: step 5 - applying terraform");
	let result = project.apply_with_vars(&resolved_vars).await?;
	trace!("TofuApplyAction: terraform apply complete");
	trace!("{result}");
	Pass(cx.input).xok()
}
