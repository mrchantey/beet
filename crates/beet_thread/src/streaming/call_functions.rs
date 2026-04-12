use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;
use beet_tool::prelude::*;




/// Calls functions and inserts the output as posts
pub async fn call_functions(
	agent: AsyncEntity,
	function_calls: impl IntoIterator<Item = OwnedFunctionCall>,
) -> Result {
	for call in function_calls.into_iter() {
		let request = Request::get(call.name())
			.with_body(call.arguments())
			.with_header::<header::ContentType>(MediaType::Json)
			.with_header::<header::Accept>(MediaType::Json);

		let output =
			match agent.call_detached(Router2.into_tool(), request).await {
				Ok(res) => match res.into_result().await {
					Ok(res) => {
						let is_json = res
							.parts
							.headers
							.get::<header::ContentType>()
							.and_then(|r| r.ok())
							.map_or(false, |ct| ct == MediaType::Json);
						let body = res.body.into_string().await.unwrap_or_else(
							|err| {
								format!(
									"Failed to read response body as string: {err}"
								)
							},
						);
						if is_json {
							body
						} else {
							// Wrap non-JSON responses as a JSON string value
							serde_json::to_string(&body).unwrap_or(body)
						}
					}
					Err(err) => {
						format!(
							"Function call returned error '{}': {err}",
							call.name()
						)
					}
				},
				Err(err) => {
					format!("Function call failed '{}': {err}", call.name())
				}
			};

		agent
			.with_state::<ThreadQuery, Result>(move |entity, mut query| {
				let thread = query.thread(entity)?;
				let actor = thread.actor(entity)?;

				let post = AgentPost::new_function_call_output(
					actor.id(),
					thread.id(),
					call.call_id().to_string(),
					output,
					call.name().to_string().xsome(),
					PostStatus::Completed,
				);
				query.commands.spawn((ChildOf(entity), post));

				Ok(())
			})
			.await?;
	}

	Ok(())
}
