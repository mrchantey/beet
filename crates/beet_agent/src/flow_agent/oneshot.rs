use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;
use serde_json::Value;



pub fn oneshot() -> impl Bundle {
	(Sequence, children![
		request_to_context(),
		ai_agent_request(),
		EndpointBuilder::new().with_action(StatusCode::Ok)
	])
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

			let agent = OllamaAgent::from_env();
			commands.run_local(async move |world| -> Result {
				let res = agent
					.chat_req(&vec![Value::String(prompt)])?
					.send()
					.await?;

				let res = res.unwrap_str().await;
				println!("Ollama response: {}", res);

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


	#[beet_core::test]
	async fn foobar() {
		AsyncPlugin::world().spawn(flow_exchange(oneshot)).exchange_str(
			Request::from_cli_str("whats the captial of thailand? one word, captial first letter, no fullstop")
				.unwrap(),
		).await.xpect_eq("Bangkok");
	}
}
