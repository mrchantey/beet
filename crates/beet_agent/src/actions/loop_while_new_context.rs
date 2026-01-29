//! Action for looping while new context is added.
//!
//! The [`LoopWhileNewContext`] component loops back to the first sibling
//! as long as new external context (tool results, user input) is added to the thread.
//! This enables agentic loops where the model can call tools and receive results.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;

/// Loops back to the first sibling while new external context is added to the thread.
///
/// This is designed for tool-calling loops where:
/// 1. A model action runs and may produce function calls
/// 2. A call_tool action executes those calls and adds FunctionOutputContext
/// 3. The loop repeats because new context was added
/// 4. The loop stops when no new context is added (model gives final answer)
///
/// ## Example
///
/// ```
/// # use beet_agent::prelude::*;
/// # use beet_flow::prelude::*;
/// # use beet_core::prelude::*;
/// # let mut world = FlowAgentPlugin::world();
/// # let provider = MockModelProvider::default();
/// // Agentic tool-calling loop
/// (Sequence, children![
///     ModelAction::new(provider),
///     call_tool(),
///     LoopWhileNewContext::default(),
/// ])
/// # ;
/// ```
#[action(loop_while_new_context)]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct LoopWhileNewContext {
	/// Maximum iterations to prevent infinite loops,
	/// or None for infinite (default)
	pub max_iterations: Option<usize>,
	/// Current iteration count.
	pub iterations: usize,
}

impl Default for LoopWhileNewContext {
	fn default() -> Self {
		Self {
			max_iterations: None,
			iterations: 0,
		}
	}
}

impl LoopWhileNewContext {
	/// Creates a new LoopWhileNewContext with the given max iterations.
	pub fn new_with_max(max_iterations: usize) -> Self {
		Self {
			max_iterations: Some(max_iterations),
			iterations: 0,
		}
	}
}

fn loop_while_new_context(
	ev: On<GetOutcome>,
	agents: AgentQuery<&ThreadContext>,
	mut query: Query<(&mut LoopWhileNewContext, &ChildOf)>,
	children: Query<&Children>,
	model_actions: Query<&ModelAction>,
	owned_context: Query<&OwnedContextOf>,
	mut commands: Commands,
) -> Result {
	let target = ev.target();
	let (mut action, parent) = query.get_mut(target)?;

	// Check max iterations
	if let Some(max_iterations) = action.max_iterations
		&& action.iterations >= max_iterations
	{
		cross_log!(
			"LoopWhileNewContext reached max iterations ({})",
			max_iterations
		);
		commands.entity(target).trigger_target(Outcome::Fail);
		return Ok(());
	}
	action.iterations += 1;

	// Find the ModelAction sibling
	let model_action_entity = children
		.iter_descendants_inclusive(parent.get())
		.find(|child| model_actions.get(*child).is_ok())
		.ok_or_else(|| {
			bevyhow!(
				"LoopWhileNewContext requires exactly one ModelAction sibling"
			)
		})?;

	let model_action = model_actions.get(model_action_entity)?;
	let sent_context = model_action.sent_context_entities();

	// Get current context entities
	let current_context: HashSet<Entity> = agents
		.get(target)
		.map(|cx| cx.iter().collect())
		.unwrap_or_default();

	// Find new context entities that are NOT owned by the ModelAction
	// (i.e., external context like tool results or user input)
	let contains_new_external_context =
		current_context.difference(&sent_context).any(|entity| {
			owned_context
				.get(*entity)
				.map(|owner| owner.0 != model_action_entity)
				.unwrap_or(true) // If no owner, consider it external
		});

	if contains_new_external_context {
		// New external context found - loop by retriggering first sibling
		let first = children.get(parent.get())?.first().ok_or_else(|| {
			bevyhow!("LoopWhileNewContext parent has no children to trigger")
		})?;
		commands.entity(*first).trigger_target(GetOutcome);
	} else {
		// No new external context - we're done
		commands.entity(target).trigger_target(Outcome::Pass);
	}

	Ok(())
}


#[cfg(test)]
mod test {
	use super::*;
	use beet_router::prelude::*;

	#[test]
	#[should_panic]
	fn no_parent() {
		let mut world = FlowAgentPlugin::world();
		world
			.spawn(LoopWhileNewContext::default())
			.trigger_target(GetOutcome)
			.flush();
	}

	#[beet_core::test]
	async fn stops_when_no_new_context() {
		let mut world = FlowAgentPlugin::world();

		let counter =
			std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
		let counter_clone = counter.clone();

		// Spawn agent
		let agent = world.spawn(Name::new("TestAgent")).id();

		// Spawn some initial context so ModelAction has something to send
		world.spawn((
			ThreadContextOf(agent),
			OwnedContextOf(agent),
			TextContext::new("initial context"),
			ContextComplete,
		));

		let provider = MockModelProvider::default();

		// Create a simple action that increments counter and passes
		let counting_action = {
			let counter = counter_clone.clone();
			OnSpawn::observe(
				move |ev: On<GetOutcome>, mut commands: Commands| {
					counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
					commands.entity(ev.target()).trigger_target(Outcome::Pass);
				},
			)
		};

		// Spawn sequence with ModelAction, counting action, and loop
		world
			.spawn((
				Name::new("Sequence"),
				ChildOf(agent),
				ActionOf(agent),
				Sequence,
				children![
					ModelAction::new(provider),
					counting_action,
					LoopWhileNewContext::default()
				],
			))
			.trigger_target(GetOutcome)
			.flush();

		// Run updates to let async complete
		for _ in 0..50 {
			world.update_local();
		}

		// Should run exactly once since no new context is added
		counter
			.load(std::sync::atomic::Ordering::SeqCst)
			.xpect_eq(1);
	}

	#[beet_core::test]
	async fn respects_max_iterations() {
		let mut world = FlowAgentPlugin::world();

		let counter =
			std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
		let counter_clone = counter.clone();

		// Spawn agent
		let agent_id = world.spawn(Name::new("TestAgent")).id();

		// Spawn some initial context so ModelAction has something to send
		world.spawn((
			ThreadContextOf(agent_id),
			OwnedContextOf(agent_id),
			TextContext::new("initial context"),
			ContextComplete,
		));

		let provider = MockModelProvider::default();

		// Create an action that always adds new context from the agent (not action)
		let action_that_adds_context = {
			let counter = counter_clone.clone();
			OnSpawn::observe(
				move |ev: On<GetOutcome>,
				      agents: AgentQuery,
				      mut commands: Commands| {
					counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
					let action = ev.target();
					let agent = agents.entity(action);

					// Spawn new context owned by agent (not action)
					// This simulates external context like tool results
					commands.spawn((
						ThreadContextOf(agent),
						OwnedContextOf(agent),
						TextContext::new("external context"),
						ContextComplete,
					));

					commands.entity(action).trigger_target(Outcome::Pass);
				},
			)
		};

		// Spawn sequence with max 3 iterations
		world
			.spawn((
				Name::new("Sequence"),
				ChildOf(agent_id),
				ActionOf(agent_id),
				Sequence,
				children![
					ModelAction::new(provider),
					action_that_adds_context,
					LoopWhileNewContext::new_with_max(3)
				],
			))
			.trigger_target(GetOutcome)
			.flush();

		// Run a few update cycles to let deferred triggers fire
		for _ in 0..50 {
			world.update_local();
		}

		// Should stop at max iterations
		// Note: runs one extra time for the loop to check no new context
		counter
			.load(std::sync::atomic::Ordering::SeqCst)
			.xpect_eq(4);
	}

	#[beet_core::test]
	async fn simulates_tool_calling_loop() {
		let mut world = FlowAgentPlugin::world();

		// Create a mock tool that returns a response
		fn test_tool() -> impl Bundle {
			related![
				Tools[tool_exchange(|| {
					EndpointBuilder::post()
						.with_path("/calculate")
						.with_description("Calculate something")
						.with_action(|| "The answer is 42")
				})]
			]
		}

		// Spawn agent with tools
		let agent = world.spawn((Name::new("TestAgent"), test_tool()));
		let agent_id = agent.id();

		// Spawn some initial context so ModelAction has something to send
		world.spawn((
			ThreadContextOf(agent_id),
			OwnedContextOf(agent_id),
			TextContext::new("What is the answer?"),
			ContextComplete,
		));

		// Use MockModelProvider which calls the first tool on first request,
		// then returns text on subsequent requests
		let provider = MockModelProvider::default();

		// Spawn the agentic loop demonstrating the new pattern:
		// LoopWhileNewContext is now the FINAL SIBLING (not the parent)
		world
			.spawn((
				Name::new("AgentLoop"),
				ChildOf(agent_id),
				ActionOf(agent_id),
				Sequence,
				children![
					ModelAction::new(provider),
					call_tool(),
					LoopWhileNewContext::new_with_max(5), // Final sibling!
				],
			))
			.trigger_target(GetOutcome)
			.flush();

		// Run the world to let async tasks complete
		for _ in 0..100 {
			world.update_local();
			// Check if we have both tool output and final text
			let has_output =
				world.query::<&FunctionOutputContext>().iter(&world).count()
					> 0;
			let has_text = world
				.query::<&TextContext>()
				.iter(&world)
				.any(|t| t.0.contains("you said:"));
			if has_output && has_text {
				break;
			}
		}

		// Verify we have a tool output in the context
		// Note: MockProvider calls tools multiple times in the loop,
		// so we just check that at least one output exists
		let outputs: Vec<&FunctionOutputContext> = world
			.query::<&FunctionOutputContext>()
			.iter(&world)
			.collect();
		(outputs.len() >= 1).xpect_true();
		outputs.first().unwrap().output.xpect_eq("The answer is 42");

		// Verify we have text context (initial + any responses)
		let text_count = world.query::<&TextContext>().iter(&world).count();
		(text_count >= 1).xpect_true();

		// This test demonstrates the full agentic tool-calling loop:
		// 1. ModelAction runs with initial context ("What is the answer?")
		// 2. MockProvider simulates the model returning a tool call
		// 3. call_tool executes the tool and spawns FunctionOutputContext
		//    - This is "external context" (owned by call_tool, not ModelAction)
		// 4. LoopWhileNewContext detects new external context and loops back
		//    by triggering the FIRST SIBLING (ModelAction)
		// 5. ModelAction runs again, now with the tool results in context
		// 6. Process repeats until MockProvider stops calling tools OR max iterations
		//
		// Key insight: LoopWhileNewContext only loops when context NOT owned
		// by ModelAction is added (i.e., tool results, user input), preventing
		// infinite loops from the model's own responses.
	}
}
