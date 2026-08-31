//! Tofu apply step for deploy sequences.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Settings for the [`TofuApplyAction`] on this entity (its defaults when absent).
///
/// A deploy publishes into its stores and then rolls the service that reads
/// them, so the route applies once per phase: `<TofuApply layer="storage"/>`
/// brings up the buckets, the image registry and the tables, the image push and
/// the content sync fill them, and a bare `<TofuApply/>` converges the whole
/// stack, rolling a task definition onto content that is already there. One
/// apply cannot express that ordering: it rolls the new task definition while
/// the bucket is still empty, so the task boots, finds no entry document in its
/// store and exits, and it names an image tag the push has not created yet.
///
/// A layer is a named set of `tofu apply -target` addresses under the one stack
/// state, declared by blocks via
/// [`Config::add_layer_resource`](terra::Config::add_layer_resource): milestones
/// through the graph rather than partitions of it, since a targeted apply pulls
/// in each target's dependencies. Blocks default their publish-into resources to
/// the [`STORAGE_LAYER`](terra::Config::STORAGE_LAYER) convention and expose the
/// assignment as a field, so a route can declare more layers and order them
/// freely.
#[derive(Debug, Default, Clone, Get, SetWith, Component, Reflect)]
#[reflect(Component, Default)]
#[require(TofuApplyAction)]
pub struct TofuApply {
	/// The layer this apply converges, the whole stack when absent. A layered
	/// apply skips the artifact upload and the ledger publish: those belong to
	/// the full apply, which alone converges resources that read artifacts and
	/// marks the deploy current. Naming a layer no resource declares is an
	/// error, never a silent no-op.
	#[set_with(unwrap_option)]
	layer: Option<SmolStr>,
}

/// Builds terraform config, uploads artifacts, publishes the ledger, and applies.
///
/// Collects each [`BuildArtifact`] + [`ArtifactLabel`] pair from
/// stack descendants to build the [`ArtifactLedger`], using
/// [`BuildArtifact::compute_source_hash`] for the hash.
///
/// Reads its own [`TofuApply`] for the layer, so the config component requires
/// this action (not the reverse, which would cycle): a layered apply skips the
/// artifacts entirely, since nothing that reads one converges in it.
#[action(handler_only)]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn TofuApplyAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let apply = cx
		.caller
		.get_cloned::<TofuApply>()
		.await
		.unwrap_or_default();
	trace!("TofuApplyAction: starting, layer {:?}", apply.layer());
	// step 1: build the project and collect variables and artifact pairs
	trace!(
		"TofuApplyAction: step 1 - building project and collecting artifacts"
	);
	let (project, stack, deployment, artifacts, variables) = cx
		.caller
		.with_world(|world, entity| -> Result<_> {
			let scope = RenderScope::render(world, entity)?;
			let variables = scope.variables();
			// each declared artifact, paired with the label its block inserted
			let artifacts =
				world
					.with_state::<(StackQuery, Query<(&ArtifactLabel, &BuildArtifact)>), _>(
						|(stacks, artifacts)| -> Result<_> {
							stacks
								.declared(entity)?
								.into_iter()
								.filter_map(|child| artifacts.get(child).ok())
								.map(|(label, artifact)| {
									(artifact.clone(), label.0.clone())
								})
								.collect::<Vec<_>>()
								.xok()
						},
					)?;
			let (stack, deployment, config) = scope.finish()?;
			let project =
				terra::Project::new(stack.clone(), deployment.clone(), config);
			(project, stack, deployment, artifacts, variables).xok()
		})
		.await??;
	trace!(
		"TofuApplyAction: collected {} artifacts, {} variables",
		artifacts.len(),
		variables.len()
	);

	// steps 2 and 3 belong to the full apply: a layered apply converges no
	// resource that reads an artifact, and publishing the ledger before the
	// service rolls would mark an undeployed version current.
	if apply.layer().is_none() {
		// step 2: build ledger, upload artifacts to S3
		trace!("TofuApplyAction: step 2 - ensuring artifacts bucket exists");
		let mut client = deployment.artifacts_client(&stack);
		client.ensure_store().await?;
		trace!("TofuApplyAction: artifacts bucket ready");

		trace!("TofuApplyAction: uploading {} artifacts", artifacts.len());
		for (artifact, label) in &artifacts {
			// build before reading: a block is declared under its `<Stack>`
			// rather than as a sequence step, so this is the only thing that
			// runs its build, and uploading a file some earlier deploy left on
			// disk is how a stale binary ships while the deploy reports success.
			artifact.build().await?;
			trace!("TofuApplyAction: uploading artifact '{}'", label);
			let artifact_path = AbsPathBuf::new(artifact.artifact_path())?;
			let bytes = fs_ext::read_async(artifact_path.as_path()).await?;
			let source_hash = artifact.compute_source_hash()?;
			let artifact_key = deployment.artifact_key(label);

			client
				.upload_artifact(label, bytes, ArtifactEntry {
					bucket_key: artifact_key.clone().into(),
					source_hash: source_hash.into(),
				})
				.await?;
			info!(
				"uploaded artifact to s3://{}/{}",
				deployment.artifact_bucket_name(&stack),
				artifact_key,
			);
		}

		// step 3: publish ledger
		trace!("TofuApplyAction: step 3 - publishing artifact ledger");
		client.publish_ledger().await.map_err(|err| {
			bevyhow!("failed to publish artifact ledger: {err}")
		})?;
		trace!(
			"TofuApplyAction: published artifact ledger: {}",
			client.ledger().deploy_id
		);
	}

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
	// step 5: apply, narrowed to the layer's addresses when one is named. An
	// unknown layer errors rather than silently widening to the whole stack.
	let targets: &[String] = match apply.layer() {
		None => &[],
		Some(layer) => project.config().layer_targets(layer)?,
	};
	trace!("TofuApplyAction: step 5 - applying terraform");
	let result = project.apply_with_vars(&resolved_vars, targets).await?;
	trace!("TofuApplyAction: terraform apply complete");
	trace!("{result}");
	Pass(cx.input).xok()
}
