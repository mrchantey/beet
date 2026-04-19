use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Repeat control-flow component for agent tool-call loops.
///
/// Calls its single child in a loop, continuing as long as the
/// thread's most recent post is a [`AgentPost::FunctionCallOutput`].
/// This drives the standard agent loop: stream response → call tools →
/// check if the agent needs another turn.
///
/// Returns [`Outcome::Pass`] when the agent's final post is not a
/// function call output (ie a text response).
/// Returns [`Outcome::Fail`] immediately if the child fails.
#[derive(Debug, Clone, Copy, Default, Component, Reflect)]
#[require(Action<(), Outcome> = Action::new_async(repeat_while_function_call_output_action))]
#[reflect(Component, Default)]
pub struct RepeatWhileFunctionCallOutput;

impl RepeatWhileFunctionCallOutput {
	pub fn new() -> Self { Self }
}

/// Checks whether the last post in the thread is a function call output.
/// The `thread_entity` should be an entity within a thread tree
/// (the Thread entity itself or any descendant).
async fn has_pending_function_call_output(
	thread_entity: &AsyncEntity,
) -> Result<bool> {
	thread_entity
		.with_state::<ThreadQuery, _>(|entity, query| -> Result<bool> {
			let thread = query.thread(entity)?;
			thread
				.posts()
				.last()
				.map(|post| AgentPost::new(post.post).is_function_call_output())
				.unwrap_or(false)
				.xok()
		})
		.await
}

async fn repeat_while_function_call_output_action(
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
		.get(|meta: &ActionMeta| *meta)
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
		// using the child entity which is within the thread tree
		if !has_pending_function_call_output(&child_entity).await? {
			return Outcome::PASS.xok();
		}
	}
}
