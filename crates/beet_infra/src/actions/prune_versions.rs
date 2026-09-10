//! The retention window over a stack's deployed versions.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// `<PruneVersions keep=10/>` — drop every deployed version outside the
/// retention window, binary and document together.
///
/// ## What a version is
///
/// After the entry document became deploy-versioned a version is TWO things: the
/// artifact binary under `versions/<id>/` in the artifacts bucket, and the
/// document root under `<id>/` in every deploy-versioned bucket the stack
/// declares. Pruning them on separate policies produces a version whose binary
/// exists and whose document does not, and a rollback onto that fails hard on a
/// missing entry — a worse failure than the unbounded growth being fixed. So
/// this is one policy over the ledger's version list, and each version's halves
/// go together.
///
/// The order within a version is deliberate: the artifact ledger goes first
/// (see [`ArtifactsClient::remove_version`]), so the version leaves the rollback
/// range before any document it names does.
///
/// ## Why not an S3 lifecycle rule
///
/// Because "expire after N days" would delete the LIVE site the moment the site
/// went undeployed for N days, which is a way to take a production site down by
/// doing nothing at all. Retention here is by COUNT and reads the ledger, so the
/// current version is never a candidate however old it is.
///
/// ## Where it goes
///
/// In the deploy group AFTER the full `<TofuApply/>`. That apply publishes the
/// ledger and rolls the service, so by the time this runs the live version is
/// the new one and cannot be what gets pruned. It has no teardown half — a
/// prune is not a resource — so it joins the deploy group alone, like a probe.
#[action]
#[derive(Debug, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn PruneVersions(
	/// How many versions to keep. This IS the rollback range, not a cleanup
	/// threshold: `rollback` counts back through the retained list, so a stack
	/// wanting to reach further back says so here.
	#[field(default = 10usize)]
	keep: usize,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	// the ledger's client, and one store per deploy-versioned bucket under this
	// stack: each of those holds a document root per version, and a data store
	// that opted out of versioning holds none.
	let (client, buckets) = cx
		.caller
		.with_state::<(StackQuery, Query<&S3BucketBlock>), _>(
			|entity, (stacks, buckets)| -> Result<_> {
				let (_, stack) = stacks.root(entity)?;
				let client = stacks.deployment().artifacts_client(&stack);
				let buckets = stacks
					.declared(entity)?
					.into_iter()
					.filter_map(|entity| buckets.get(entity).ok())
					.filter(|bucket| bucket.deploy_versioned())
					.map(|bucket| {
						BlobStore::new(bucket.store(&stack, None))
					})
					.collect::<Vec<_>>();
				(client, buckets).xok()
			},
		)
		.await??;

	let versions = client.prunable_versions(keep).await?;
	if versions.is_empty() {
		info!("no versions outside the last {keep}");
		return Pass(cx.input).xok();
	}
	for version in &versions {
		client.remove_version(version).await?;
		for bucket in &buckets {
			let documents =
				bucket.with_subdir(SmolPath::new(version.to_string()));
			for key in documents.list().await? {
				documents.remove(&key).await?;
			}
		}
		info!("pruned version {version}");
	}
	info!(
		"pruned {} version(s), keeping the last {keep}",
		versions.len()
	);
	Pass(cx.input).xok()
}
