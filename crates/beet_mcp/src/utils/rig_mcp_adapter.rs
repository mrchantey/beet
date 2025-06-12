use anyhow::Result;
use rig::agent::AgentBuilder;
use rig::completion::CompletionModel;
use rig::embeddings::EmbeddingModel;
use rig::embeddings::EmbeddingsBuilder;
use rig::tool::ToolDyn as RigTool;
use rig::tool::ToolEmbeddingDyn;
use rig::tool::ToolSet;
use rig::vector_store::in_memory_store::InMemoryVectorStore;
use rmcp::RoleClient;
use rmcp::Service;
use rmcp::model::CallToolRequestParam;
use rmcp::model::Tool as McpTool;
use rmcp::service::RunningService;
use rmcp::service::ServerSink;

pub struct McpToolAdaptor {
	tool: McpTool,
	server: ServerSink,
}


#[extend::ext(name=MyTypeExt)]
#[allow(async_fn_in_trait)]
pub impl<M: CompletionModel> AgentBuilder<M>
where
	Self: Sized,
{
	// async fn add_stdio_mcp_from_env()

	/// Add the beet MCP tools to the agent.
	/// - `tool_embedding_model` is the embedding model used to embed the tools, this can be different from
	/// the rag model used with the vector database.
	async fn add_mcp_tools<
		S: Service<RoleClient>,
		E: 'static + EmbeddingModel,
	>(
		mut self,
		client: &RunningService<RoleClient, S>,
		tool_embedding_model: E,
	) -> Result<Self> {
		let tool_set = McpToolAdaptor::get_tools(client).await?;

		let embeddings = EmbeddingsBuilder::new(tool_embedding_model.clone())
			.documents(tool_set.schemas()?)?
			.build()
			.await?;
		let store =
			InMemoryVectorStore::from_documents_with_id_f(embeddings, |f| {
				tracing::info!("store tool {}", f.name);
				f.name.clone()
			});
		let index = store.index(tool_embedding_model);
		const MAX_TOOLS_TO_RETRIEVE: usize = 4;

		self = self.dynamic_tools(MAX_TOOLS_TO_RETRIEVE, index, tool_set);

		Ok(self)
	}
}


impl McpToolAdaptor {
	pub async fn get_tools<S: Service<RoleClient>>(
		client: &RunningService<RoleClient, S>,
	) -> Result<ToolSet> {
		let mut tool_set = ToolSet::default();
		let mut task = tokio::task::JoinSet::<Result<_>>::new();
		task.spawn(tools_from_server(client.peer().clone()));
		let results = task.join_all().await;
		for result in results {
			match result {
				Err(e) => {
					tracing::error!(error = %e, "Failed to get tool set");
				}
				Ok(tools) => {
					tool_set.add_tools(tools);
				}
			}
		}
		Ok(tool_set)
	}
}



async fn tools_from_server(server: ServerSink) -> Result<ToolSet> {
	let tools = server.list_all_tools().await?;
	let mut tool_builder = ToolSet::builder();
	for tool in tools {
		tracing::info!("get tool: {}", tool.name);
		let adaptor = McpToolAdaptor {
			tool: tool.clone(),
			server: server.clone(),
		};
		tool_builder = tool_builder.dynamic_tool(adaptor);
	}
	let tool_set = tool_builder.build();
	Ok(tool_set)
}



impl RigTool for McpToolAdaptor {
	fn name(&self) -> String { self.tool.name.to_string() }

	fn definition(
		&self,
		_prompt: String,
	) -> std::pin::Pin<
		Box<
			dyn Future<Output = rig::completion::ToolDefinition>
				+ Send
				+ Sync
				+ '_,
		>,
	> {
		Box::pin(std::future::ready(rig::completion::ToolDefinition {
			name: self.name(),
			description: self
				.tool
				.description
				.as_deref()
				.unwrap_or_default()
				.to_string(),
			parameters: self.tool.schema_as_json_value(),
		}))
	}

	fn call(
		&self,
		args: String,
	) -> std::pin::Pin<
		Box<
			dyn Future<Output = Result<String, rig::tool::ToolError>>
				+ Send
				+ Sync
				+ '_,
		>,
	> {
		let server = self.server.clone();
		Box::pin(async move {
			let call_mcp_tool_result = server
				.call_tool(CallToolRequestParam {
					name: self.tool.name.clone(),
					arguments: serde_json::from_str(&args)
						.map_err(rig::tool::ToolError::JsonError)?,
				})
				.await
				.inspect(|result| tracing::debug!(?result))
				.inspect_err(|error| tracing::error!(%error))
				.map_err(|e| {
					rig::tool::ToolError::ToolCallError(Box::new(e))
				})?;

			Ok(serde_json::to_string(&call_mcp_tool_result)?)
		})
	}
}


impl ToolEmbeddingDyn for McpToolAdaptor {
	fn context(&self) -> serde_json::Result<serde_json::Value> {
		serde_json::to_value(self.tool.clone())
	}

	fn embedding_docs(&self) -> Vec<String> {
		vec![
			self.tool
				.description
				.as_deref()
				.unwrap_or_default()
				.to_string(),
		]
	}
}
