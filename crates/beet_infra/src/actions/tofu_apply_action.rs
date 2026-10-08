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
/// A full apply builds the terraform config, builds and uploads the artifacts
/// into the stack's repo store, publishes the ledger and applies. It collects
/// each [`BuildArtifact`] paired with the
/// [`artifact_label`](ErasedBlock::artifact_label) its [`ErasedBlock`] carries
/// from stack descendants to build the [`ArtifactLedger`], using
/// [`BuildArtifact::compute_source_hash`] for the hash. A layered apply skips
/// the artifacts entirely, since nothing that reads one converges in it.
///
/// The ledger lives in the repo store ([`RepoStoreQuery::artifacts_client`]),
/// so a stack shipping an artifact must declare a versioned one and is told
/// what to declare when it does not; a stack with a repo store and no
/// artifact still publishes its ledger, since that is what a content sync
/// adopts and a rollback indexes.
///
/// Before any of it writes, an apply holding the sealed Cloudflare deploy
/// token refuses a planned change that token is not allowed to make, ie to an
/// R2 bucket's configuration, naming the elevated run that is
/// ([`DeployerToken::elevated`]).
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
	let backend = terra::Project::resolve_backend(&cx.caller).await?;
	let (project, artifacts, client, variables) = cx
		.caller
		.with_world(move |world, entity| -> Result<_> {
			let scope = RenderScope::render(world, entity)?;
			let variables = scope.variables();
			// each declared artifact, paired with the label its block declared,
			// the client publishing them into the stack's repo store, and the
			// secret store a content variable resolves through
			let (artifacts, client, secrets) =
				world.with_state::<(
					StackQuery,
					Query<(&ErasedBlock, &BuildArtifact)>,
					RepoStoreQuery,
				), _>(|(stacks, artifacts, repos)| -> Result<_> {
					let declared = stacks.declared(entity)?;
					let secrets = stacks.secret_store(entity)?;
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
					// a binary needs a versioned repo store to publish into,
					// and a stack shipping one without it is told what to
					// declare here rather than after the build
					let client = match built.is_empty() {
						true => repos.find_artifacts_client(entity)?,
						false => Some(repos.artifacts_client(entity)?),
					};
					(built, client, secrets).xok()
				})?;
			let (stack, deployment, config) = scope.finish()?;
			// with the declared variables, so the apply resolves the content
			// ones from their source rather than expecting them on the request
			let project = terra::Project::new_with_variables(
				stack,
				deployment,
				config,
				backend,
				variables.clone(),
			)
			.with_secret_store(secrets);
			(project, artifacts, client, variables).xok()
		})
		.await??;
	trace!(
		"TofuApply: collected {} artifacts, {} variables",
		artifacts.len(),
		variables.len()
	);

	// step 2: resolve variables
	trace!(
		"TofuApply: step 2 - resolving {} variables",
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
	// narrowed to the layer's addresses when one is named. An unknown layer
	// errors rather than silently widening to the whole stack.
	let targets: &[String] = match layer.as_ref() {
		None => &[],
		Some(layer) => project.config().layer_targets(layer)?,
	};

	// step 3: refuse, before anything is written, a change the Cloudflare
	// deploy token is deliberately not allowed to make
	let route = cx.input.parts().path().join("/");
	TofuApply::refuse_elevated(
		&cx.caller,
		&route,
		&project,
		&resolved_vars,
		targets,
	)
	.await?;

	// steps 4 and 5 belong to the full apply: a layered apply converges no
	// resource that reads an artifact, and publishing the ledger before the
	// service rolls would mark an undeployed version current.
	if let (None, Some(mut client)) = (&layer, client) {
		// step 4: build and upload each artifact under this launch's version
		trace!(
			"TofuApply: step 4 - uploading {} artifacts",
			artifacts.len()
		);
		for (artifact, label) in &artifacts {
			// build before reading: a block is declared under its `<Stack>`
			// rather than as a sequence step, so this is the only thing that
			// runs its build, and uploading a file some earlier deploy left on
			// disk is how a stale binary ships while the deploy reports success.
			artifact.build().await?;
			trace!("TofuApply: uploading artifact '{}'", label);
			let artifact_path = AbsPath::new(artifact.artifact_path())?;
			let bytes = fs_ext::read_async(artifact_path).await?;
			let source_hash = artifact.compute_source_hash()?;
			let artifact_key = client.ledger().artifact_key(label);

			client
				.upload_artifact(label, bytes, ArtifactEntry {
					bucket_key: artifact_key.to_string().into(),
					source_hash: source_hash.into(),
				})
				.await?;
			info!(
				"uploaded artifact {label} to {}/{artifact_key}",
				client.store().describe()
			);
		}

		// step 5: publish ledger
		trace!("TofuApply: step 5 - publishing artifact ledger");
		client.publish_ledger().await.map_err(|err| {
			bevyhow!("failed to publish artifact ledger: {err}")
		})?;
		trace!(
			"TofuApply: published artifact ledger: {}",
			client.ledger().deploy_id
		);
	}

	// step 6: apply
	trace!("TofuApply: step 6 - applying terraform");
	let result = project.apply_with_vars(&resolved_vars, targets).await?;
	trace!("TofuApply: terraform apply complete");
	trace!("{result}");
	// the summary alone at info: a deploy log must show what each apply did
	// without carrying tofu's full narration, which repeats the plan
	let scope = layer.as_deref().unwrap_or("stack");
	for line in result.lines().filter(|line| {
		line.starts_with("Apply complete!") || line.starts_with("No changes.")
	}) {
		info!("tofu apply ({scope}): {line}");
	}
	Pass(cx.input).xok()
}

impl TofuApply {
	/// Refuse a planned write to a type the Cloudflare deploy token only
	/// refreshes ([`DeployerToken::elevated_for`]) while this launch holds that
	/// token, naming the elevated run of `route` that holds what the write
	/// needs. Checked before the apply rather than learned from it: Cloudflare's
	/// 403 names no permission group, and it lands mid-apply, after a replaced
	/// box and before its provision.
	///
	/// Plans only when the stack declares such a type and the token is the
	/// sealed one, so every other stack, and the elevated run itself, pays
	/// nothing. A token passed for the command that is not the sealed one (an
	/// elevated run, CI's) is Cloudflare's to judge.
	async fn refuse_elevated(
		caller: &AsyncEntity,
		route: &str,
		project: &terra::Project,
		vars: &[(SmolStr, SmolStr)],
		targets: &[String],
	) -> Result {
		let declares_elevated = project
			.config()
			.declared_types()
			.into_iter()
			.any(|declared| !DeployerToken::elevated_for(declared).is_empty());
		if !declares_elevated {
			return OK;
		}
		let (sealed, mint) = caller
			.with_world(|world, _| {
				let sealed = OpenSecrets::find(world, CloudflareMint::RECORD)
					.ok()
					.map(|secret| secret.value);
				let mint = world
					.query::<&CloudflareMint>()
					.iter(world)
					.find_map(|mint| mint.command.clone());
				(sealed, mint)
			})
			.await?;
		if sealed.is_none() || sealed != cloudflare_api_ext::token().ok() {
			return OK;
		}
		let changes = project
			.planned_changes(vars, targets)
			.await?
			.into_iter()
			.filter(|change| {
				!DeployerToken::elevated_for(&change.resource_type).is_empty()
			})
			.collect::<Vec<_>>();
		let Some(first) = changes.first() else {
			return OK;
		};
		let mint = mint.unwrap_or_else(|| "beet cloudflare/mint".into());
		bevybail!(
			"this apply would change {}, which the Cloudflare deploy token only \
			reads: writing it needs {}, which reaches what no deploy rebuilds, so \
			no held token carries it. Nothing is written yet. Run this deploy \
			elevated in a terminal, which asks for the mint token and holds {} \
			for the one command:\n\n\t{mint} -- {route}",
			changes
				.iter()
				.map(|change| format!(
					"`{}` ({})",
					change.address, change.action
				))
				.collect::<Vec<_>>()
				.join(", "),
			DeployerToken::elevated_for(&first.resource_type)
				.iter()
				.map(TokenPermission::name)
				.collect::<Vec<_>>()
				.join(", "),
			match DeployerToken::elevated_for(&first.resource_type).len() {
				1 => "it",
				_ => "them",
			},
		)
	}

	/// Converge only the named layer, rather than the whole stack.
	pub fn for_layer(layer: impl Into<SmolStr>) -> Self {
		Self {
			layer: Some(layer.into()),
		}
	}
}
