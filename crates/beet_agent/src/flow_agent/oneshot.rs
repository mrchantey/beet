use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_router::prelude::*;



pub fn oneshot() -> impl Bundle {
	(Sequence, children![
		request_to_context(),
		ai_agent_request(),
		context_to_response()
	])
}

fn context_to_response() -> impl Bundle {
	EndpointBuilder::new().with_action(StatusCode::Ok)
}

fn request_to_context() -> impl Bundle {
	OnSpawn::observe(
		|ev: On<GetOutcome>,
		 agent_query: AgentQuery<&RequestMeta>,
		 mut commands: Commands|
		 -> Result {
			let action = ev.target();
			let req_meta = agent_query.get(action)?;
			let query = req_meta.path().join(" ").xprint_display();

			let agent = agent_query.entity(action);
			commands.spawn((ContextOf(agent), TextContext(query)));

			commands.entity(action).trigger_target(Outcome::Pass);
			Ok(())
		},
	)
}

fn ai_agent_request() -> impl Bundle {
	OnSpawn::observe(
		|ev: On<GetOutcome>,
		 query: ContextQuery,
		 mut commands: AsyncCommands|
		 -> Result {
			let action = ev.target();
			let texts = query.texts(ev.target());
			let prompt = texts
				.iter()
				.map(|t| t.0.as_str())
				.collect::<Vec<_>>()
				.join("\n");

			commands.run_local(async move |world| -> Result {
				let mut provider = OllamaProvider::default();

				let body = openresponses::RequestBody::new(
					provider.default_small_model(),
				)
				.with_input(prompt);
				println!("sending request.. {body:?}");
				let response = provider.send(body).await.unwrap();

				println!("Ollama response: {}", response.first_text().unwrap());

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


#[cfg(test)]
mod test {
	use beet_net::prelude::*;

	use super::*;


	#[beet_core::test(timeout_ms = 15_000)]
	async fn foobar() {
		FlowAgentPlugin::world().spawn(flow_exchange(oneshot)).exchange_str(
			Request::from_cli_str("whats the capital of thailand? one word, captial first letter, no fullstop")
				.unwrap(),
		).await.xpect_eq("Bangkok");
	}
}
