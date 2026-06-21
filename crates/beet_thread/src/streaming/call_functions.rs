use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

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
			match agent.call_detached(route_action(), request).await {
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
			.with_state::<WindowMut, Result>(move |entity, mut window_mut| {
				let actor_id = window_mut.actor_id(entity)?;
				let thread_id = window_mut.thread_id(entity)?;

				let post = AgentPost::new_function_call_output(
					actor_id,
					thread_id,
					call.call_id().to_string(),
					output,
					call.name().to_string().xsome(),
					PostStatus::Completed,
				);
				window_mut.push_post(entity, post)
			})
			.await??;
	}

	Ok(())
}
