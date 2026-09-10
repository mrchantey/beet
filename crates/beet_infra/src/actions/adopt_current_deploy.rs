//! Pointing a launch at the version that is already live.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// The step a CONTENT-ONLY verb runs before it publishes: read the artifact
/// ledger and adopt the deploy id it names, so everything this launch addresses
/// per deploy resolves to the version currently being served.
///
/// A deploy mints a version. Every artifact it uploads, and every store its
/// deploy-versioned buckets resolve, nests under that new id, and the binary it
/// ships is baked to read it. That is exactly right for `deploy` and exactly
/// wrong for `sync`: a `sync` changes content without rolling the function, so a
/// launch minting its own id would publish a complete, correct copy of the site
/// into a prefix no running binary has ever heard of, and report success.
///
/// So a `sync` route declares this first:
///
/// ```bsx
/// <Route path="sync" {ExchangeSequence}>
///     <AdoptCurrentDeploy/>
///     <DirSync bucket="app" local_dir="."/>
/// </Route>
/// ```
///
/// The deploy route must NOT: it is the thing minting the new version, and
/// adopting the old one would publish this deploy's document over the running
/// one, which is the skew the versioning exists to remove.
///
/// The same read a rollback does ([`Rollback`](crate::prelude::Rollback) calls
/// it through the ledger too), minus the apply: this changes what the launch
/// addresses, never what is deployed.
#[action]
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn AdoptCurrentDeploy(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let client = cx
		.caller
		.with_state::<StackQuery, _>(|entity, stacks| {
			stacks.artifacts_client(entity)
		})
		.await??;
	let ledger = client.current_ledger().await?.ok_or_else(|| {
		bevyhow!(
			"no current artifact ledger: nothing is deployed yet, so there is \
			 no live version to publish into. Run the stack's `deploy` verb."
		)
	})?;
	let deploy_id = ledger.deploy_id;
	cx.caller
		.world()
		.with_resource::<Deployment, _>(move |mut deployment| {
			deployment.update_from_ledger(&ledger);
		})
		.await;
	info!("publishing into the live deploy {deploy_id}");
	Pass(cx.input).xok()
}
