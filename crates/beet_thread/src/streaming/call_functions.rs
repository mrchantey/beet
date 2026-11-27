use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Calls functions and inserts the output as posts.
///
/// An agent's tools are the routes of the [`Router`] beneath it, its own url
/// space, so each call decodes its name to a route path
/// ([`FunctionToolDefinition::decode_name`]) and is exchanged with that
/// router, its arguments the JSON body a typed-input route reads as its
/// input. Whatever answers is the call's output, the router's not-found
/// with its "did you mean" included, so an agent reaches only the tools it
/// was given. An answer of bytes, ie an image, is kept as the output's file
/// ([`AgentPost::new_function_call_output_files`]), so any route answering
/// media is a tool an agent can look through, and any route's
/// `multipart/mixed` answer is split into its text and its files
/// ([`Multipart`]).
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
		let answer = match router {
			Some(router) => {
				call_function(&agent.world().entity(router), &call).await
			}
			None => ToolAnswer::Text(format!(
				"Function call refused '{}': this agent has no tools",
				call.name()
			)),
		};

		agent
			.with_state::<ThreadRecordsQuery, Result>(
				move |entity, mut records_mut| {
					let actor_id = records_mut.actor_id(entity)?;
					let thread_id = records_mut.thread_id(entity)?;
					let post = match answer {
						ToolAnswer::Text(output) => {
							AgentPost::new_function_call_output(
								actor_id,
								thread_id,
								call.post(),
								call.call_id().to_string(),
								output,
								PostStatus::Completed,
							)
						}
						ToolAnswer::Files { text, files } => {
							AgentPost::new_function_call_output_files(
								actor_id,
								thread_id,
								call.post(),
								call.call_id().to_string(),
								text,
								files,
								PostStatus::Completed,
							)
						}
					};
					records_mut.push_post(entity, post)
				},
			)
			.await??;
	}

	Ok(())
}

/// What a tool answered: text, or files beside any text, each a model reads
/// as a file.
enum ToolAnswer {
	Text(String),
	Files {
		text: String,
		files: Vec<(MediaBytes, Option<String>)>,
	},
}

impl ToolAnswer {
	/// A `multipart/mixed` answer: each part without a filename that is
	/// text joins the answer's text, a line each, and every other part is a
	/// file, its stem its filename's, else the call's.
	fn multipart(multipart: Multipart, call: &OwnedFunctionCall) -> Self {
		let mut text = Vec::new();
		let mut files = Vec::new();
		for part in multipart.into_parts() {
			match (part.filename(), part.media().media_type().is_text()) {
				(None, true) => text.push(
					String::from_utf8_lossy(part.media().bytes()).to_string(),
				),
				_ => {
					let stem = part
						.file_stem()
						.map(str::to_string)
						.or_else(|| call.file_stem());
					files.push((MediaBytes::from(part), stem));
				}
			}
		}
		Self::Files {
			text: text.join("\n"),
			files,
		}
	}
}

/// The answer of one call exchanged with the agent's `router`: a JSON body
/// as it is, a body of bytes, ie an image, as a file, any other as a JSON
/// string, or the error it answered with.
async fn call_function(
	router: &AsyncEntity,
	call: &OwnedFunctionCall,
) -> ToolAnswer {
	let request =
		Request::get(FunctionToolDefinition::decode_name(call.name()))
			.with_body(call.arguments())
			.with_header::<header::ContentType>(MediaType::Json)
			.with_header::<header::Accept>(MediaType::Json);

	let res = match router.call::<Request, Response>(request).await {
		Ok(res) => match res.into_result().await {
			Ok(res) => res,
			Err(err) => {
				return ToolAnswer::Text(format!(
					"Function call returned error '{}': {err}",
					call.name()
				));
			}
		},
		Err(err) => {
			return ToolAnswer::Text(format!(
				"Function call failed '{}': {err}",
				call.name()
			));
		}
	};
	let res = match Multipart::from_response(res).await {
		Ok(Ok(multipart)) => return ToolAnswer::multipart(multipart, call),
		Ok(Err(res)) => res,
		Err(err) => {
			return ToolAnswer::Text(format!(
				"Failed to read the multipart answer of '{}': {err}",
				call.name()
			));
		}
	};
	let media_type = res
		.parts
		.headers
		.get::<header::ContentType>()
		.and_then(|media_type| media_type.ok());
	match media_type {
		Some(media_type) if media_type == MediaType::Json => {
			ToolAnswer::Text(res.body.into_string().await.unwrap_or_else(
				|err| format!("Failed to read response body as string: {err}"),
			))
		}
		// bytes, ie an image or a pdf
		Some(media_type) if !media_type.is_text() => {
			match res.body.into_bytes().await {
				Ok(bytes) => ToolAnswer::Files {
					text: String::new(),
					files: vec![(
						MediaBytes::new(media_type, bytes),
						call.file_stem(),
					)],
				},
				Err(err) => ToolAnswer::Text(format!(
					"Failed to read response body: {err}"
				)),
			}
		}
		_ => {
			let body = res.body.into_string().await.unwrap_or_else(|err| {
				format!("Failed to read response body as string: {err}")
			});
			// wrap a text body as a JSON string value
			ToolAnswer::Text(
				MediaBytes::serialize(MediaType::Json, &body)
					.and_then(|bytes| bytes.as_utf8().map(str::to_string))
					.unwrap_or(body),
			)
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_scene::prelude::*;

	/// Each agent's tools are the routes of its own router: the writer's
	/// `write-text` lands, while the reader, given `read_only`, has no
	/// `write-text` in its url space and none of the writer's, so the call is
	/// its router's not-found.
	#[beet_core::test]
	async fn read_only_toolset_cannot_write() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.install_types::<ThreadPlugin>();
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
		let thread = BsxParser::spawn(app.world_mut(), source).unwrap();
		app.world_mut().entity_mut(thread).insert(store.clone());
		app.update();
		ThreadRecords::reduce_now(app.world_mut());
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
			.get::<ThreadRecords>(thread)
			.unwrap()
			.posts()
			.iter()
			.filter_map(|post| match post.as_agent_post()? {
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
			.install_types::<ThreadPlugin>();
		let source = r#"
<Router>
	<Thread {Sequence}>
		<CreateActor name="User" kind="User"><CreatePost text="go"/></CreateActor>
		<CreateActor name="Agent" kind="Agent" {MockPostStreamer}><StoreToolset/></CreateActor>
	</Thread>
</Router>
"#;
		let root = BsxParser::spawn(app.world_mut(), source).unwrap();
		app.update();
		ThreadRecords::reduce_now(app.world_mut());
		let thread = app.world().entity(root).get::<Children>().unwrap()[0];
		app.world_mut()
			.entity_mut(thread)
			.call::<(), Outcome>(())
			.await
			.unwrap();
		app.world()
			.get::<ThreadRecords>(thread)
			.unwrap()
			.last_post()
			.unwrap()
			.to_string()
			.xpect_contains("outside a `<Router>` beneath it");
	}
}
