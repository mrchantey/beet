//! Tofu apply step for deploy sequences.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Tofu apply step for deploy exchange sequences.
/// Builds a [`terra::Project`] from the nearest [`Stack`] ancestor and applies it.
/// If an [`ArtifactLedger`] is present, passes artifact S3 keys and hashes
/// as Terraform variables and publishes the ledger on success.
#[action]
#[derive(Default, Component)]
pub async fn TofuApplyAction(cx: ActionContext<Request>) -> Result<Outcome<Request, Response>> {
	// build the project and optionally read the artifact ledger
	let (project, ledger, stack_opt) = cx
		.caller
		.with_state::<(StackQuery, AncestorQuery<&ArtifactLedger>, AncestorQuery<&Stack>), _>(
			|entity, (stack_query, ledger_query, anc_stack)| -> Result<_> {
				let project = stack_query.build_project(entity)?;
				let ledger = ledger_query.get(entity).ok().cloned();
				let stack = anc_stack.get(entity).ok().cloned();
				(project, ledger, stack).xok()
			},
		)
		.await?;

	let result = if let Some(ref ledger) = ledger {
		// collect terraform variables from the artifact ledger
		let vars = build_artifact_vars(ledger, &stack_opt);
		info!("applying with {} artifact variables", vars.len());
		project.apply_with_vars(&vars).await?
	} else {
		project.apply().await?
	};

	info!("{result}");

	// publish ledger to S3 on successful apply
	#[cfg(feature = "aws")]
	if let (Some(ledger), Some(stack)) = (&ledger, &stack_opt) {
		let client = stack.artifacts_client();
		client.publish_ledger(ledger).await?;
		info!("published artifact ledger: {}", ledger.uuid);
	}

	Pass(cx.input).xok()
}

/// Build Terraform `-var` pairs from an artifact ledger.
/// Currently supports lambda artifacts via the `lambda_s3_bucket`,
/// `lambda_s3_key`, and `lambda_source_hash` variables.
fn build_artifact_vars(
	ledger: &ArtifactLedger,
	stack: &Option<Stack>,
) -> Vec<(SmolStr, SmolStr)> {
	let mut vars = Vec::new();
	// use the first artifact for lambda vars; multi-artifact support can be added later
	if let Some((_name, entry)) = ledger.artifacts.iter().next() {
		if let Some(stack) = stack {
			vars.push(("lambda_s3_bucket".into(), stack.artifact_bucket_name().into()));
		}
		vars.push(("lambda_s3_key".into(), entry.s3_key.clone()));
		vars.push(("lambda_source_hash".into(), entry.source_hash.clone()));
	}
	vars
}
