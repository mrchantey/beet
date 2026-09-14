//! The retention window over a stack's deployed versions.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// `<PruneVersions keep=10/>` — drop every deployed version outside the
/// retention window.
///
/// ## What a version is
///
/// One prefix in the stack's repo store: the document under `repo/`, the
/// binaries under `bin/` and the ledger beside them (see [`ArtifactLedger`]).
/// So a prune is one removal per version, and a document can never outlive
/// its binary or the reverse, which is the shape a rollback relies on.
///
/// The order within a version is deliberate: the ledger goes first (see
/// [`ArtifactsClient::remove_version`]), so the version leaves the rollback
/// range before anything it names does.
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
	let client = cx
		.caller
		.with_state::<RepoStoreQuery, _>(|entity, repos| {
			repos.artifacts_client(entity)
		})
		.await??;

	let versions = client.prunable_versions(keep).await?;
	if versions.is_empty() {
		info!("no versions outside the last {keep}");
		return Pass(cx.input).xok();
	}
	for version in &versions {
		client.remove_version(version).await?;
		info!("pruned version {version}");
	}
	info!(
		"pruned {} version(s), keeping the last {keep}",
		versions.len()
	);
	Pass(cx.input).xok()
}
