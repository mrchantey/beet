use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Calls functions and inserts the output as posts.
///
/// An agent's tools are the routes of the [`Router`] beneath it, its own url
/// space, so each call decodes its name to a route path
/// ([`FunctionToolDefinition::decode_name`]) and is exchanged with that
/// router, its arguments a JSON body, which a route reading its params off
/// the request reads as params and a typed-input route as its input. Whatever answers is the call's output, the router's not-found
/// with its "did you mean" included, so an agent reaches only the tools it
/// was given.
///
/// [`Router`]: beet_router::prelude::Router
pub(crate) async fn call_functions(
	agent: AsyncEntity,
	function_calls: impl IntoIterator<Item = OwnedFunctionCall>,
) -> Result {
	let router = agent
		.with_state::<ThreadQuery, _>(|entity, query| query.tool_router(entity))
		.await??;
	for call in function_calls.into_iter() {
		let output = match router {
			Some(router) => {
				call_function(&agent.world().entity(router), &call).await
			}
			None => format!(
				"Function call refused '{}': this agent has no tools",
				call.name()
			),
		};

		agent
			.with_state::<ThreadWindowQuery, Result>(
				move |entity, mut window_mut| {
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
				},
			)
			.await??;
	}

	Ok(())
}

/// The output of one call exchanged with the agent's `router`: the response
/// body, a non-JSON one as a JSON string, or the error it answered with.
async fn call_function(
	router: &AsyncEntity,
	call: &OwnedFunctionCall,
) -> String {
	let request =
		Request::get(FunctionToolDefinition::decode_name(call.name()))
			.with_body(call.arguments())
			.with_header::<header::ContentType>(MediaType::Json)
			.with_header::<header::Accept>(MediaType::Json);

	match router.call::<Request, Response>(request).await {
		Ok(res) => match res.into_result().await {
			Ok(res) => {
				let is_json = res
					.parts
					.headers
					.get::<header::ContentType>()
					.and_then(|r| r.ok())
					.map_or(false, |ct| ct == MediaType::Json);
				let body = res.body.into_string().await.unwrap_or_else(|err| {
					format!("Failed to read response body as string: {err}")
				});
				if is_json {
					body
				} else {
					// Wrap non-JSON responses as a JSON string value
					serde_json::to_string(&body).unwrap_or(body)
				}
			}
			Err(err) => {
				format!("Function call returned error '{}': {err}", call.name())
			}
		},
		Err(err) => {
			format!("Function call failed '{}': {err}", call.name())
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// Each agent's tools are the routes of its own router: the writer's
	/// `write-text` lands, while the reader, given `read_only`, has no
	/// `write-text` in its url space and none of the writer's, so the call is
	/// its router's not-found.
	#[beet_core::test]
	async fn read_only_toolset_cannot_write() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.install_type::<ThreadPlugin>();
		let source = r#"
<Thread {Sequence}>
	<CreateActor name="User" kind="User"><CreatePost text="go"/></CreateActor>
	<CreateActor name="Writer" kind="Agent" {MockPostStreamer{script:[Call{tool:"write-text", arguments:{path:"a.txt", text:"hi"}}]}}>
		<Router><StoreToolset/></Router>
	</CreateActor>
	<CreateActor name="Reader" kind="Agent" {MockPostStreamer{script:[
		Call{tool:"write-text", arguments:{path:"b.txt", text:"h"}},
		Call{tool:"read-blob", arguments:{path:"a.txt"}},
	]}}>
		<Router><StoreToolset read_only=true/></Router>
	</CreateActor>
</Thread>
"#;
		let store = BlobStore::temp();
		let thread = BsxTemplate::parse_entry(app.world(), source)
			.unwrap()
			.spawn(app.world_mut())
			.unwrap();
		app.world_mut().entity_mut(thread).insert(store.clone());
		app.update();
		ThreadWindow::reduce_now(app.world_mut());
		let agents = app
			.world_mut()
			.query_filtered::<Entity, With<MockPostStreamer>>()
			.iter(app.world())
			.collect::<Vec<_>>();
		let paths = |app: &mut App, agent: Entity| {
			app.world_mut().with_state::<ThreadQuery, _>(|query| {
				query
					.tools(agent)
					.unwrap()
					.into_iter()
					.filter_map(|(_, tool)| match tool {
						ToolDefinition::Function(func) => {
							Some(func.path().to_string())
						}
						_ => None,
					})
					.collect::<Vec<_>>()
			})
		};
		// the writer has all five tools, the reader two, each at its own path
		let (reader, writer) = match paths(&mut app, agents[0]).len() {
			2 => (agents[0], agents[1]),
			_ => (agents[1], agents[0]),
		};
		paths(&mut app, reader)
			.xpect_eq(vec!["list-blobs".to_string(), "read-blob".to_string()]);
		paths(&mut app, writer).len().xpect_eq(5);
		for agent in [writer, reader, reader] {
			app.world_mut()
				.entity_mut(agent)
				.call::<(), Outcome>(())
				.await
				.unwrap();
		}
		store
			.get(&RelPath::new("a.txt"))
			.await
			.unwrap()
			.to_vec()
			.xpect_eq(b"hi".to_vec());
		store
			.exists(&RelPath::new("b.txt"))
			.await
			.unwrap()
			.xpect_false();
		let outputs = app
			.world()
			.get::<ThreadWindow>(thread)
			.unwrap()
			.posts()
			.iter()
			.filter_map(|post| match post.as_agent_post() {
				AgentPost::FunctionCallOutput(output) => {
					Some(output.output().to_string())
				}
				_ => None,
			})
			.collect::<Vec<_>>();
		outputs[1]
			.as_str()
			.xpect_contains("write-text")
			.xpect_contains("404");
		outputs[2].as_str().xpect_contains("hi");
	}

	/// An agent holding tools outside a router beneath it fails its turn with
	/// an error post naming the fix, rather than calling into an ancestor url
	/// space.
	#[beet_core::test]
	async fn tools_outside_a_router_fail_the_turn() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.install_type::<ThreadPlugin>();
		let source = r#"
<Router>
	<Thread {Sequence}>
		<CreateActor name="User" kind="User"><CreatePost text="go"/></CreateActor>
		<CreateActor name="Agent" kind="Agent" {MockPostStreamer}><StoreToolset/></CreateActor>
	</Thread>
</Router>
"#;
		let root = BsxTemplate::parse_entry(app.world(), source)
			.unwrap()
			.spawn(app.world_mut())
			.unwrap();
		app.update();
		ThreadWindow::reduce_now(app.world_mut());
		let thread = app.world().entity(root).get::<Children>().unwrap()[0];
		app.world_mut()
			.entity_mut(thread)
			.call::<(), Outcome>(())
			.await
			.unwrap();
		app.world()
			.get::<ThreadWindow>(thread)
			.unwrap()
			.last_post()
			.unwrap()
			.to_string()
			.xpect_contains("outside a `<Router>` beneath it");
	}
}
