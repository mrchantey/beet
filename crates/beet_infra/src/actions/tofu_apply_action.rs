//! Tofu apply step for deploy sequences.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Tofu apply step for deploy exchange sequences.
/// Builds a [`terra::Project`] from the nearest [`Stack`] ancestor and applies it.
#[action]
#[derive(Default, Component)]
pub async fn TofuApplyAction(cx: ActionContext<Request>) -> Result<Outcome<Request, Response>> {
	let result = cx
		.caller
		.with_state::<StackQuery, _>(|entity, query| {
			query.build_project(entity)
		})
		.await?
		.apply()
		.await?;
	// log the apply output
	info!("{result}");
	Pass(cx.input).xok()
}
