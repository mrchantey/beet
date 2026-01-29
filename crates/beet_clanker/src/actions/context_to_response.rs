use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;


/// Converts context to a response.
///
/// This action collects assistant responses from context and assembles
/// them into a `Response` on the agent entity.
pub fn context_to_response() -> impl Bundle {
	OnSpawn::observe(
		|ev: On<GetOutcome>,
		 contexts: Query<(&ContextRole, &TextContext)>,
		 agents: AgentQuery<&ThreadContext>,
		 mut commands: Commands|
		 -> Result {
			let action = ev.target();
			let items = agents.get(action)?;
			let agent = agents.entity(action);

			let mut response_parts = Vec::new();
			for (role, text) in
				items.iter().filter_map(|entity| contexts.get(entity).ok())
			{
				if role == &ContextRole::Assistant {
					response_parts.push(text.0.clone());
				}
			}
			let response_text = response_parts.join("\n");
			commands
				.entity(agent)
				.insert(Response::ok().with_body(response_text));
			commands.entity(action).trigger_target(Outcome::Pass);
			Ok(())
		},
	)
}
