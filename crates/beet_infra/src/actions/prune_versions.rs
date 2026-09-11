//! The retention window over a stack's deployed versions.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// `<PruneVersions keep=10/>` — drop every deployed version outside the
/// retention window, artifact and document together.
///
/// ## What a version is
///
/// Two things, in two places: the artifact the deploy built, keyed by deploy id
/// in the artifact store, and the entry document it published, under the same
/// deploy id in the stack's repo store. Prune them on separate policies and you
/// get a version whose artifact exists and whose document does not, which a
/// rollback fails on. So this is ONE policy over the ledger's version list, and
/// each version's halves go together.
///
/// The order within a version is deliberate: the ledger goes first (see
/// [`ArtifactsClient::remove_version`]), so the version leaves the rollback
/// range before any document it names does.
///
/// ## Retention is by count
///
/// Because `rollback` counts back through the versions that remain, so what is
/// kept IS the rollback range, and the current version is never a candidate
/// however old it is. A stack that has not deployed in a year still has every
/// version it kept.
///
/// ## Where it goes
///
/// In the deploy group AFTER the full `<TofuApply/>`. That apply publishes the
/// ledger and rolls the service, so by the time this runs the live version is
/// the new one and cannot be what gets pruned. It has no teardown half, since a
/// prune is not a resource, so it joins the deploy group alone like a probe.
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
	// the ledger's client, and the stack's repo store rooted at the top rather
	// than at this launch's version, since what it prunes is other versions
	let (client, repo) = cx
		.caller
		.with_state::<(StackQuery, Query<&S3BucketBlock>), _>(
			|entity, (stacks, stores)| -> Result<_> {
				let (_, stack) = stacks.root(entity)?;
				let repo = stacks
					.declared(entity)?
					.into_iter()
					.filter_map(|entity| stores.get(entity).ok())
					.find(|store| store.label() == RepoBucket::LABEL)
					.map(|store| BlobStore::new(store.store(&stack, None)));
				(stacks.deployment().artifacts_client(&stack), repo).xok()
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
		if let Some(repo) = &repo {
			let documents =
				repo.with_subdir(SmolPath::new(version.to_string()));
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
