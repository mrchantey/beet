//! Generate artifact ledger step for deploy sequences.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Generates a new [`ArtifactLedger`] with a UUID v7 and inserts it
/// on the deploy sequence parent entity.
#[action]
#[derive(Default, Component)]
pub async fn GenerateArtifactLedger(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let ledger = ArtifactLedger::default();
	info!("generated artifact ledger: {}", ledger.uuid);
	// insert the ledger on the parent (deploy sequence root)
	let parent = cx.caller.get_cloned::<ChildOf>().await?.get();
	cx.caller.world().entity(parent).insert(ledger);

	Pass(cx.input).xok()
}
