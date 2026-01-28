//! Action for executing tool calls from context.
//!
//! The [`call_tool`] action processes [`FunctionCallContext`] items in the thread
//! and routes them to the appropriate tool endpoint, spawning [`FunctionOutputContext`]
//! with the results.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::ExchangeTarget;

/// Creates an action that executes pending tool calls in the thread context.
///
/// ## Example
///
/// ```
/// use beet_agent::prelude::*;
/// use beet_flow::prelude::*;
///
/// // Typical usage in a tool-calling loop
/// (Sequence, children![
///     ModelAction::new(provider),
///     call_tool(),
/// ])
/// ```
pub fn call_tool() -> impl Bundle {
	OnSpawn::observe(
		|ev: On<GetOutcome>,
		 agents: AgentQuery<&ThreadContext>,
		 function_calls: Query<&FunctionCallContext>,
		 function_outputs: Query<&FunctionOutputContext>,
		 mut commands: AsyncCommands|
		 -> Result {
			let action = ev.target();
			let agent = agents.entity(action);
			let context = agents.get(action).ok();

			// Collect all call_ids that already have outputs
			let existing_outputs: HashSet<String> = function_outputs
				.iter()
				.map(|fo| fo.call_id.clone())
				.collect();

			// Find function calls that need execution
			let pending_calls: Vec<FunctionCallContext> = context
				.map(|cx| {
					cx.iter()
						.filter_map(|entity| function_calls.get(entity).ok())
						.filter(|fc| !existing_outputs.contains(&fc.call_id))
						.map(|fc| fc.clone())
						.collect()
				})
				.unwrap_or_default();

			if pending_calls.is_empty() {
				// No pending calls, pass through
				commands
					.commands
					.entity(action)
					.trigger_target(Outcome::Pass);
				return Ok(());
			}

			commands.run_local(async move |world| -> Result {
				// TODO parallel tool calls
				for func_call in pending_calls {
					// Parse the tool name to find the tool set and path
					// Execute the tool call by triggering an exchange on the tool entity
					match execute_tool_call(&world, &func_call).await {
						Ok(output) => {
							spawn_function_output(
								&world,
								agent,
								action,
								&func_call.call_id,
								&output,
							)
							.await;
						}
						Err(err) => {
							spawn_error_output(
								&world,
								agent,
								action,
								&func_call.call_id,
								&err.to_string(),
							)
							.await;
						}
					}
				}

				world
					.entity(action)
					.trigger_target_then(Outcome::Pass)
					.await;
				Ok(())
			});

			Ok(())
		},
	)
}

/// Executes a tool call by sending an exchange to the tool entity.
async fn execute_tool_call(
	world: &AsyncWorld,
	func_call: &FunctionCallContext,
) -> Result<String> {
	let name = func_call.name.clone();
	let Some((tool_entity, path)) = world
		.with_then(move |world| ToolQuery::parse_tool_name(&world, &name))
		.await
	else {
		bevybail!("Unknown tool: {}", func_call.name)
	};

	// Build a request for the tool endpoint
	let request = Request::post(path)
		.with_header("content-type", "application/json")
		.with_body(func_call.arguments.clone());

	// Execute via the exchange system
	let response: beet_core::prelude::Response =
		world.entity(tool_entity).exchange(request).await;

	// Check for errors
	if !response.status().is_ok() {
		bevybail!(
			"Tool call failed with status {}: {}",
			response.status(),
			response.text().await.unwrap_or_default()
		);
	}

	response.text().await
}

/// Spawns a FunctionOutputContext with the given output.
async fn spawn_function_output(
	world: &AsyncWorld,
	agent: Entity,
	action: Entity,
	call_id: &str,
	output: &str,
) {
	world
		.spawn_then((
			ThreadContextOf(agent),
			OwnedContextOf(action),
			ContextRole::User,
			ContextComplete,
			FunctionOutputContext {
				call_id: call_id.to_string(),
				output: output.to_string(),
			},
		))
		.await;
}

/// Spawns a FunctionOutputContext with an error message.
async fn spawn_error_output(
	world: &AsyncWorld,
	agent: Entity,
	action: Entity,
	call_id: &str,
	error: &str,
) {
	spawn_function_output(
		world,
		agent,
		action,
		call_id,
		&format!("Error: {}", error),
	)
	.await;
}


#[cfg(test)]
mod test {
	use super::*;
	use beet_router::prelude::*;

	fn test_tools() -> impl Bundle {
		related![
			Tools[tool_exchange(|| {
				(InfallibleSequence, children![
					EndpointBuilder::post()
						.with_path("/add")
						.with_description("Add two numbers")
						.with_action(|| "8"),
					EndpointBuilder::post()
						.with_path("/greet")
						.with_description("Greet someone")
						.with_action(|| "Hello!"),
				])
			})]
		]
	}

	#[beet_core::test]
	async fn passes_with_no_pending_calls() {
		let mut world = FlowAgentPlugin::world();

		// Spawn agent with tools but no function calls
		let agent = world.spawn((Name::new("TestAgent"), test_tools())).id();

		// Spawn action
		let action = world
			.spawn((
				Name::new("CallToolAction"),
				ChildOf(agent),
				ActionOf(agent),
				call_tool(),
			))
			.trigger_target(GetOutcome)
			.flush()
			.id();

		// Check that outcome was triggered (action should pass)
		world.entity(action).contains::<Running>().xpect_false();
	}

	#[beet_core::test]
	async fn executes_pending_tool_call() {
		let mut world = FlowAgentPlugin::world();

		// Spawn agent with tools
		let agent = world.spawn((Name::new("TestAgent"), test_tools()));
		let agent_id = agent.id();

		// Get tool entity to build proper tool name
		let tool_entity = agent.get::<Tools>().unwrap()[0];

		// Spawn a context entity to create the ThreadContext relationship
		world.spawn(ThreadContextOf(agent_id));

		// Spawn a function call context
		let call_id = "test-call-123".to_string();
		let tool_name = ToolQuery::tool_name(tool_entity, "/add");
		world.spawn((
			ThreadContextOf(agent_id),
			OwnedContextOf(agent_id),
			ContextRole::Assistant,
			ContextComplete,
			FunctionCallContext {
				call_id: call_id.clone(),
				name: tool_name.clone(),
				arguments: r#"{"a": 5, "b": 3}"#.to_string(),
			},
		));

		// Spawn action
		world
			.spawn((
				Name::new("CallToolAction"),
				ChildOf(agent_id),
				ActionOf(agent_id),
				call_tool(),
			))
			.trigger_target(GetOutcome)
			.flush();

		// Run the world to let async tasks complete
		// Use update_local which properly handles async task queues
		for _ in 0..50 {
			world.update_local();
			if world.query::<&FunctionOutputContext>().iter(&world).count() > 0
			{
				break;
			}
		}

		// Verify FunctionOutputContext was spawned
		let outputs: Vec<&FunctionOutputContext> = world
			.query::<&FunctionOutputContext>()
			.iter(&world)
			.collect();

		outputs.len().xpect_eq(1);
		outputs[0].call_id.xpect_eq(call_id);
		outputs[0].output.xpect_eq("8");
	}

	#[beet_core::test]
	async fn skips_already_processed_calls() {
		let mut world = FlowAgentPlugin::world();

		// Spawn agent with tools
		let agent = world.spawn((Name::new("TestAgent"), test_tools()));
		let agent_id = agent.id();
		let tool_entity = agent.get::<Tools>().unwrap()[0];

		let call_id = "already-done".to_string();
		let tool_name = ToolQuery::tool_name(tool_entity, "/add");

		// Spawn a context entity to create the ThreadContext relationship
		world.spawn(ThreadContextOf(agent_id));

		// Spawn function call
		world.spawn((
			ThreadContextOf(agent_id),
			OwnedContextOf(agent_id),
			ContextRole::Assistant,
			ContextComplete,
			FunctionCallContext {
				call_id: call_id.clone(),
				name: tool_name,
				arguments: r#"{"a": 1, "b": 1}"#.to_string(),
			},
		));

		// Spawn existing output (simulating already processed)
		world.spawn((
			ThreadContextOf(agent_id),
			OwnedContextOf(agent_id),
			ContextRole::User,
			ContextComplete,
			FunctionOutputContext {
				call_id: call_id.clone(),
				output: "2".to_string(),
			},
		));

		// Spawn action
		world
			.spawn((
				Name::new("CallToolAction"),
				ChildOf(agent_id),
				ActionOf(agent_id),
				call_tool(),
			))
			.trigger_target(GetOutcome)
			.flush();

		// Run a few updates
		for _ in 0..5 {
			world.update_local();
		}

		// Should still have only one output (no duplicate)
		let output_count =
			world.query::<&FunctionOutputContext>().iter(&world).count();
		output_count.xpect_eq(1);
	}
}
