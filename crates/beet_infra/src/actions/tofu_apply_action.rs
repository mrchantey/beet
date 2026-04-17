//! Tofu apply step for deploy sequences.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Tofu apply step for deploy exchange sequences.
/// Builds a [`terra::Project`] from the nearest [`Stack`] ancestor and applies it.
/// If an [`ArtifactLedger`] is present, publishes the ledger to S3 first,
/// then passes artifact S3 keys and hashes as Terraform variables.
#[action]
#[derive(Default, Component)]
pub async fn TofuApplyAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	// build the project and read the artifact ledger + stack
	let (project, ledger, stack) = cx
		.caller
		.with_state::<(
			StackQuery,
			AncestorQuery<&ArtifactLedger>,
			AncestorQuery<&Stack>,
		), _>(
			|entity, (stack_query, ledger_query, anc_stack)| -> Result<_> {
				let project = stack_query.build_project(entity)?;
				let ledger = ledger_query.get(entity)?.clone();
				let stack = anc_stack.get(entity)?.clone();
				(project, ledger, stack).xok()
			},
		)
		.await?;

	// publish ledger to S3 before apply so the artifact is available
	stack
		.artifacts_client()
		.publish_ledger(&ledger)
		.await
		.map_err(|err| bevyhow!("failed to publish artifact ledger: {err}"))?;
	info!("published artifact ledger: {}", ledger.uuid);

	// collect terraform variables from the artifact ledger
	let vars = ledger.build_vars(&stack.artifact_bucket_name());
	info!("applying with {} artifact variables", vars.len());
	let result = project.apply_with_vars(&vars).await?;

	info!("{result}");

	Pass(cx.input).xok()
}
