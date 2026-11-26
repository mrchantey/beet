use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Whether the newest post of the thread that is not a log is a function
/// call output its call answered, rather than one an interrupt closed.
/// `entity` is the window or any descendant.
async fn has_pending_function_call_output(
	entity: &AsyncEntity,
) -> Result<bool> {
	entity
		.with_state::<ThreadQuery, _>(|entity, query| -> Result<bool> {
			query
				.records(entity)?
				.last_content_post()
				.is_some_and(|post| {
					post.status() == PostStatus::Completed
						&& post.kind() == &PostKind::ToolOutput
				})
				.xok()
		})
		.await
		.flatten()
}

/// Repeat control-flow component for agent tool-call loops.
///
/// Calls its single child in a loop, continuing as long as the
/// thread's newest post that is not a log is a completed
/// [`AgentPost::FunctionCallOutput`].
/// This drives the standard agent loop: stream a response, call its tools,
/// then call the model again if it has outputs to read.
///
/// Returns [`Outcome::Pass`] when that post is anything else, ie a text
/// response, a person's post, or the output of a call an [`InterruptTurn`]
/// cut short, so an interrupted turn ends its loop.
/// Returns [`Outcome::Fail`] immediately if the child fails.
#[action]
#[derive(Debug, Clone, Copy, Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn RepeatWhileFunctionCallOutput(
	cx: ActionContext,
) -> Result<Outcome> {
	let child = match cx
		.caller
		.get(|children: &Children| children.first().copied())
		.await
	{
		Ok(Some(child)) => child,
		_ => return Outcome::PASS.xok(),
	};

	let world = cx.world();
	let child_entity = world.entity(child);

	let action_meta = child_entity
		.get(|meta: &ActionMeta| meta.clone())
		.await
		.map_err(|err| {
			bevyhow!(
				"RepeatWhileFunctionCallOutput child has no action: {child:?}, error: {err}"
			)
		})?;
	action_meta.assert_match::<(), Outcome>()?;

	loop {
		// run the child (the agent sequence)
		match child_entity.call::<(), Outcome>(()).await? {
			Outcome::Pass(_) => {}
			Outcome::Fail(_) => return Outcome::FAIL.xok(),
		}

		// check if the agent produced tool call outputs that need processing,
		// using the child entity, which is at or under the window
		if !has_pending_function_call_output(&child_entity).await? {
			return Outcome::PASS.xok();
		}
	}
}
