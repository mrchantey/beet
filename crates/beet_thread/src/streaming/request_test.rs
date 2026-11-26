//! Entering a window and building the requests its agents' models read,
//! shared by the tests.
use crate::prelude::*;
use async_openai::types::chat::CreateChatCompletionRequest;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// A model at no address, for building requests.
pub(crate) fn model() -> ModelDef {
	ModelDef {
		provider_slug: "provider".into(),
		model_slug: "model".into(),
		url: "http://localhost".into(),
		auth: None,
	}
}

/// Enter the window `window` with a plain request, as an entry would.
pub(crate) async fn enter(app: &mut App, window: Entity) {
	app.world_mut()
		.run_async_local_then(async move |world: AsyncWorld| -> Result {
			Thread::enter(
				world.clone(),
				window,
				ThreadEntry::parse(Request::get("chat").parts().clone())?,
			)
			.await
		})
		.await
		.unwrap();
}

/// The behavior entity acting as `actor`.
pub(crate) fn actor_entity(app: &mut App, actor: &ActorId) -> Entity {
	app.world_mut()
		.query::<(Entity, &ActorRef)>()
		.iter(app.world())
		.find(|(_, actor_ref)| &actor_ref.0 == actor)
		.unwrap()
		.0
}

/// The request `streamer` builds for the agent at `agent`.
pub(crate) async fn responses_request(
	app: &mut App,
	agent: Entity,
	streamer: O11sStreamer,
) -> o11s::RequestBody {
	app.world_mut()
		.run_async_local_then(async move |world: AsyncWorld| {
			streamer.build_request(world.entity(agent)).await
		})
		.await
		.unwrap()
		.0
}

/// The request the completions protocol builds for the agent at `agent`.
pub(crate) async fn completions_request(
	app: &mut App,
	agent: Entity,
) -> CreateChatCompletionRequest {
	app.world_mut()
		.run_async_local_then(async move |world: AsyncWorld| {
			CompletionsStreamer::new(model())
				.build_request(world.entity(agent))
				.await
		})
		.await
		.unwrap()
		.0
}

/// The open responses request, then the completions one, the agent at
/// `agent` builds, as json.
pub(crate) async fn requests(app: &mut App, agent: Entity) -> [Value; 2] {
	let responses =
		responses_request(app, agent, O11sStreamer::new(model())).await;
	let completions = completions_request(app, agent).await;
	[
		Value::from_serde(&responses).unwrap(),
		Value::from_serde(&completions).unwrap(),
	]
}
