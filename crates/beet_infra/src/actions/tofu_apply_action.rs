//! Tofu apply step for deploy sequences.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// The deploy step that converges the stack, whole or one layer at a time.
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
///
/// A full apply builds the terraform config, uploads the artifacts, publishes
/// the ledger and applies. It collects each [`BuildArtifact`] paired with the
/// [`artifact_label`](ErasedBlock::artifact_label) its [`ErasedBlock`] carries
/// from stack descendants to build the [`ArtifactLedger`], using
/// [`BuildArtifact::compute_source_hash`] for the hash. A layered apply skips
/// the artifacts entirely, since nothing that reads one converges in it.
#[action]
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn TofuApply(
	/// The layer this apply converges, the whole stack when absent. A layered
	/// apply skips the artifact upload and the ledger publish: those belong to
	/// the full apply, which alone converges resources that read artifacts and
	/// marks the deploy current. Naming a layer no resource declares is an
	/// error, never a silent no-op.
	#[field]
	layer: Option<SmolStr>,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	trace!("TofuApply: starting, layer {layer:?}");
	// step 1: build the project and collect variables and artifact pairs
	trace!("TofuApply: step 1 - building project and collecting artifacts");
	let (project, stack, deployment, artifacts, repo, variables) = cx
		.caller
		.with_world(|world, entity| -> Result<_> {
			let scope = RenderScope::render(world, entity)?;
			let variables = scope.variables();
			// each declared artifact, paired with the label its block declared,
			// and the stack's repo store, which the ledger records so a machine
			// resolving its release per start resolves its document with it
			let (artifacts, repo) =
				world.with_state::<(
					StackQuery,
					Query<(&ErasedBlock, &BuildArtifact)>,
					RepoStoreQuery,
				), _>(|(stacks, artifacts, repos)| -> Result<_> {
					let declared = stacks.declared(entity)?;
					let built = declared
						.iter()
						.filter_map(|child| artifacts.get(*child).ok())
						.filter_map(|(erased, artifact)| {
							erased
								.artifact_label
								.clone()
								.map(|label| (artifact.clone(), label))
						})
						.collect::<Vec<_>>();
					let repo = repos
						.find(entity)?
						.map(|repo| repo.store_uri())
						.transpose()?;
					(built, repo).xok()
				})?;
			let (stack, deployment, config) = scope.finish()?;
			// with the declared variables, so the apply resolves the content
			// ones from their source rather than expecting them on the request
			let project = terra::Project::new_with_variables(
				stack.clone(),
				deployment.clone(),
				config,
				variables.clone(),
			);
			(project, stack, deployment, artifacts, repo, variables).xok()
		})
		.await??;
	trace!(
		"TofuApply: collected {} artifacts, {} variables",
		artifacts.len(),
		variables.len()
	);

	// steps 2 and 3 belong to the full apply: a layered apply converges no
	// resource that reads an artifact, and publishing the ledger before the
	// service rolls would mark an undeployed version current.
	if layer.is_none() {
		// step 2: build ledger, upload artifacts to S3
		trace!("TofuApply: step 2 - ensuring artifacts bucket exists");
		let mut client = deployment.artifacts_client(&stack);
		if let Some(repo) = &repo {
			client = client.with_repo(repo.clone());
		}
		client.ensure_store().await?;
		trace!("TofuApply: artifacts bucket ready");

		trace!("TofuApply: uploading {} artifacts", artifacts.len());
		for (artifact, label) in &artifacts {
			// build before reading: a block is declared under its `<Stack>`
			// rather than as a sequence step, so this is the only thing that
			// runs its build, and uploading a file some earlier deploy left on
			// disk is how a stale binary ships while the deploy reports success.
			artifact.build().await?;
			trace!("TofuApply: uploading artifact '{}'", label);
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
				deployment.artifact_store_name(&stack),
				artifact_key,
			);
		}

		// step 3: publish ledger
		trace!("TofuApply: step 3 - publishing artifact ledger");
		client.publish_ledger().await.map_err(|err| {
			bevyhow!("failed to publish artifact ledger: {err}")
		})?;
		trace!(
			"TofuApply: published artifact ledger: {}",
			client.ledger().deploy_id
		);
	}

	// step 4: resolve variables
	trace!(
		"TofuApply: step 4 - resolving {} variables",
		variables.len()
	);
	// only the AMBIENT ones: a content variable's value is not in flight on this
	// request, it is a fact about the stack, so `apply_with_vars` reads it from
	// its source alongside the state passphrase. Asking the request for it would
	// fail, and defaulting it is the revocation this split exists to prevent.
	let resolved_vars: Vec<(SmolStr, SmolStr)> = variables
		.iter()
		.filter(|variable| !variable.is_content())
		.map(|variable| {
			variable
				.resolve_value(cx.input.parts())
				.map(|value| (variable.key().clone(), value))
		})
		.collect::<Result<Vec<_>>>()?;
	trace!("TofuApply: resolved variables: {:?}", resolved_vars);
	// step 5: apply, narrowed to the layer's addresses when one is named. An
	// unknown layer errors rather than silently widening to the whole stack.
	let targets: &[String] = match layer.as_ref() {
		None => &[],
		Some(layer) => project.config().layer_targets(layer)?,
	};
	trace!("TofuApply: step 5 - applying terraform");
	let result = project.apply_with_vars(&resolved_vars, targets).await?;
	trace!("TofuApply: terraform apply complete");
	trace!("{result}");
	Pass(cx.input).xok()
}

impl TofuApply {
	/// Converge only the named layer, rather than the whole stack.
	pub fn for_layer(layer: impl Into<SmolStr>) -> Self {
		Self {
			layer: Some(layer.into()),
		}
	}
}
