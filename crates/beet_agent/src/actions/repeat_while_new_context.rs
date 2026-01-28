//! Action for repeating while new context is added.
//!
//! The [`RepeatWhileNewContext`] component runs its child repeatedly
//! as long as new external context (tool results, user input) is added to the thread.
//! This enables agentic loops where the model can call tools and receive results.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;

/// Repeats child actions while new external context is added to the thread.
///
/// This is designed for tool-calling loops where:
/// 1. A model action runs and may produce function calls
/// 2. A call_tool action executes those calls and adds FunctionOutputContext
/// 3. The loop repeats because new context was added
/// 4. The loop stops when no new context is added (model gives final answer)
///
/// ## Example
///
/// ```ignore
/// use beet_agent::prelude::*;
/// use beet_flow::prelude::*;
///
/// // Agentic tool-calling loop
/// (RepeatWhileNewContext::new(10), Sequence, children![
///     ModelAction::new(provider),
///     call_tool(),
/// ])
/// ```
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
#[component(on_add=on_add)]
#[require(PreventPropagateEnd)]
pub struct RepeatWhileNewContext {
	/// Maximum iterations to prevent infinite loops,
	/// or None for infinite (default)
	pub max_iterations: Option<usize>,
	/// Current iteration count.
	pub iterations: usize,
}

impl Default for RepeatWhileNewContext {
	fn default() -> Self {
		Self {
			max_iterations: None,
			iterations: 0,
		}
	}
}

impl RepeatWhileNewContext {
	/// Creates a new RepeatWhileNewContext with the given max iterations.
	pub fn new_with_max(max_iterations: usize) -> Self {
		Self {
			max_iterations: Some(max_iterations),
			iterations: 0,
		}
	}
}

fn on_add(mut world: DeferredWorld, cx: HookContext) {
	world
		.commands()
		.entity(cx.entity)
		.insert(OnSpawn::observe(on_outcome));
}

fn on_outcome(
	ev: On<Outcome>,
	agents: AgentQuery<&ThreadContext>,
	mut states: Query<&mut RepeatWhileNewContext>,
	children: Query<&Children>,
	model_actions: Query<&ModelAction>,
	mut commands: Commands,
) -> Result {
	let action = ev.target();

	// If child failed, propagate failure
	if ev.event() == &Outcome::Fail {
		commands.entity(action).trigger_target(Outcome::Fail);
		return Ok(());
	}

	let mut state = states.get_mut(action)?;

	// Check max iterations
	if let Some(max_iterations) = state.max_iterations
		&& state.iterations >= max_iterations
	{
		cross_log!(
			"RepeatWhileNewContext reached max iterations ({})",
			max_iterations
		);
		commands.entity(action).trigger_target(Outcome::Fail);
		return Ok(());
	}
	state.iterations += 1;


	// get current context
	let model_actions = children
		.iter_descendants_inclusive(action)
		.filter_map(|child| model_actions.get(child).ok())
		.collect::<Vec<_>>();
	if model_actions.len() != 1 {
		bevybail!(
			"RepeatWhileNewContext requires exactly one ModelAction descendant, found {}",
			model_actions.len()
		);
	}
	let sent_context = model_actions[0].sent_context_entities();

	// Get current context entities
	let current_context: HashSet<Entity> = agents
		.get(action)
		.map(|cx| cx.iter().collect())
		.unwrap_or_default();

	// Find new context entities
	let contains_new_context =
		current_context.difference(&sent_context).next().is_some();

	if contains_new_context {
		// New external context found - repeat by retriggering GetOutcome
		commands
			.entity(action)
			.insert(TriggerDeferred::get_outcome());
	} else {
		// No new external context - we're done
		commands.entity(action).trigger_target(Outcome::Pass);
	}

	Ok(())
}


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn stops_when_no_new_context() {
		let mut world = FlowAgentPlugin::world();

		let counter =
			std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
		let counter_clone = counter.clone();

		// Spawn agent
		let agent = world.spawn(Name::new("TestAgent")).id();

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

		// Spawn repeat action with counting child
		world
			.spawn((
				Name::new("RepeatAction"),
				ChildOf(agent),
				ActionOf(agent),
				RepeatWhileNewContext::default(),
				children![counting_action],
			))
			.trigger_target(GetOutcome)
			.flush();

		// Should run exactly once since no new context is added
		counter
			.load(std::sync::atomic::Ordering::SeqCst)
			.xpect_eq(1);
	}

	#[test]
	fn respects_max_iterations() {
		let mut world = FlowAgentPlugin::world();

		let counter =
			std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
		let counter_clone = counter.clone();

		// Spawn agent
		let agent_id = world.spawn(Name::new("TestAgent")).id();

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

		// Spawn repeat action with max 3 iterations
		world
			.spawn((
				Name::new("RepeatAction"),
				ChildOf(agent_id),
				ActionOf(agent_id),
				RepeatWhileNewContext::new_with_max(3),
				children![action_that_adds_context],
			))
			.trigger_target(GetOutcome)
			.flush();

		// Run a few update cycles to let deferred triggers fire
		for _ in 0..10 {
			world.update_local();
		}

		// Should stop at max iterations
		counter
			.load(std::sync::atomic::Ordering::SeqCst)
			.xpect_eq(3);
	}
}
