#![allow(unexpected_cfgs)]
use crate::prelude::*;
use anyhow::Result;
use rmcp::Error as McpError;
use rmcp::RoleServer;
use rmcp::ServerHandler;
use rmcp::ServiceExt;
use rmcp::const_string;
use rmcp::model::*;
use rmcp::schemars;
use rmcp::service::RequestContext;
use rmcp::tool;
use rmcp::transport::sse_server::SseServer;
use rmcp::transport::stdio;

use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use std::fs;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(description = "A query for a crate's documentation or source code")]
pub struct CrateRagQuery {
	#[serde(default, flatten)]
	pub rag_query: RagQuery,
	#[serde(default, flatten)]
	pub crate_meta: CrateMeta,
	#[serde(default, flatten)]
	pub content_type: ContentTypeStr,
}

impl CrateRagQuery {
	pub fn source_key(&self) -> ContentSourceKey {
		ContentSourceKey {
			crate_meta: self.crate_meta.clone(),
			content_type: self.content_type.clone().into(),
		}
	}
	// /// for use with guides queries to ensure we're getting up-to-date information
	// pub fn versioned_query(&self) -> RagQuery {
	// 	RagQuery {
	// 		max_docs: self.rag_query.max_docs,
	// 		search_query: format!(
	// 			"version {} {}",
	// 			self.crate_meta.crate_version, self.rag_query.search_query,
	// 		),
	// 	}
	// }
}

#[derive(Clone)]
pub struct McpServer<E: BeetEmbedModel> {
	/// example of persistant server state.
	/// for stdio requests this will always be 0.
	request_count: Arc<Mutex<i32>>,
	/// a test database of a fictional world
	nexus_db: Database<E>,
	#[allow(unused)]
	embedding_model: E,
	known_sources: KnownSources,
}

#[tool(tool_box)]
impl<E: BeetEmbedModel> McpServer<E> {
	pub async fn new(
		embedding_model: E,
		known_sources: KnownSources,
	) -> Result<Self> {
		Ok(Self {
			request_count: Arc::new(Mutex::new(0)),
			nexus_db: Database::connect(
				embedding_model.clone(),
				".cache/nexus_arcana.db",
			)
			.await?,
			embedding_model,
			known_sources,
		})
	}
	/// Start the server using SSE transport.
	/// This will spawn a new insance of [`McpServer`] for each client that
	/// connects via `bind_address/sse`.
	pub async fn serve_sse(
		embedding_model: E,
		bind_address: &str,
		sources: KnownSources,
	) -> Result<()> {
		let addr = bind_address.to_string();
		tracing::info!(
			"Listening for mcp clients\nServer URL http://{addr}/sse"
		);
		let ct = SseServer::serve(bind_address.parse()?)
			.await?
			.with_service_directly(move || {
				futures::executor::block_on(async {
					tracing::info!("Starting MCP Server on {addr}");
					Self::new(embedding_model.clone(), sources.clone())
						.await
						.unwrap()
				})
			});

		tokio::signal::ctrl_c().await?;
		ct.cancel();

		Ok(())
	}

	pub async fn serve_stdio(self) -> Result<()> {
		self.serve(stdio())
			.await
			.inspect_err(|e| {
				tracing::error!("serving error: {:?}", e);
			})?
			.waiting()
			.await?;
		Ok(())
	}

	fn create_resource_text(&self, uri: &str, name: &str) -> Resource {
		RawResource::new(uri, name.to_string()).no_annotation()
	}

	async fn increment_count(&self) -> Result<i32, McpError> {
		let mut counter = self.request_count.lock().await;
		*counter += 1;
		Ok(*counter)
	}

	#[tool(description = "Get the number of requests made to the server")]
	async fn get_count(
		&self,
		// bug: rig client fails for tools without params
		#[tool(param)]
		#[schemars(description = "set this to true always")]
		_true_is_true: bool,
	) -> Result<CallToolResult, McpError> {
		Ok(CallToolResult::success(vec![Content::text(
			self.increment_count().await?.to_string(),
		)]))
	}

	#[tool(description = "Ping the server to check if it's alive")]
	async fn ping(
		&self,
		// bug: rig client fails for tools without params
		#[tool(param)]
		#[schemars(description = "set this to true always")]
		_true_is_true: bool,
	) -> Result<CallToolResult, McpError> {
		tracing::info!("ping");
		self.increment_count().await?;
		let now = std::time::SystemTime::now();
		let formatted = now
			.duration_since(std::time::UNIX_EPOCH)
			.map_err(|_| McpError::internal_error("time_error", None))?
			.as_secs();
		Ok(CallToolResult::success(vec![Content::text(format!(
			"pong at {:2} seconds since epoch",
			formatted
		))]))
	}
	#[tool(
		description = "Query a vector db about the fictional world of Nexus Arcana"
	)]
	async fn nexus_rag(
		&self,
		#[tool(aggr)] query: RagQuery,
	) -> Result<CallToolResult, McpError> {
		let db = self.nexus_db.clone();

		self.tool_middleware("nexus_rag", query, async move |q, _| {
			db.try_init_nexus_arcana().await?;
			db.query(&q).await
		})
		.await
	}
	#[tool(description = r#"
	Query for information about a crate, including documentation, examples, source code, etc
	"#)]
	async fn crate_rag(
		&self,
		#[tool(aggr)] query: CrateRagQuery,
	) -> Result<CallToolResult, McpError> {
		let model = self.embedding_model.clone();

		self.tool_middleware("crate_rag", query, async move |query, known_sources| {
			let key = query.source_key();
			let db_path = key.local_db_path(&model);
			known_sources.assert_exists(&key)?;
			if !fs::exists(&db_path)? {
				anyhow::bail!(
					"source is known but could not be found at: {}\nit may need to be indexed first, or the path is incorrect",
					db_path.to_string_lossy()
				);
			}

			Database::connect(model.clone(), &db_path.to_string_lossy())
				.await?
				.query(&query.rag_query)
				// .query(&query.versioned_query())
				.await
		})
		.await
	}

	/// wrap a tool call with tracing and error handling
	async fn tool_middleware<I: Serialize, O: IntoCallToolResult<M>, M>(
		&self,
		tool_name: &str,
		param: I,
		func: impl 'static
		+ for<'b> AsyncFn(I, &'b KnownSources) -> anyhow::Result<O>,
		// func: impl AsyncFn(I) -> anyhow::Result<O>,
	) -> Result<CallToolResult, McpError> {
		tracing::info!(
			"Tool Call: {tool_name} \n{}",
			serde_json::to_string_pretty(&param)
				.unwrap_or_else(|_| "invalid json".to_string())
		);
		let result = func(param, &self.known_sources).await.map_err(|e| {
			tracing::error!("Tool Call Error: {tool_name} - {e}");
			McpError::internal_error(
				"tool_call_error",
				Some(json!({ "error": e.to_string() })),
			)
		})?;
		Ok(result.into_call_tool_result())
	}
}

trait IntoCallToolResult<M> {
	fn into_call_tool_result(self) -> CallToolResult;
}
impl<T> IntoCallToolResult<(Vec<T>, String)> for Vec<T>
where
	T: ToString,
{
	fn into_call_tool_result(self) -> CallToolResult {
		CallToolResult::success(
			self.into_iter()
				.map(|val| {
					let val = val.to_string();
					Content::text(val)
				})
				.collect(),
		)
	}
}

const_string!(Echo = "echo");
#[tool(tool_box)]
impl<E: BeetEmbedModel> ServerHandler for McpServer<E> {
	fn get_info(&self) -> ServerInfo {
		ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_prompts()
                .enable_resources()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(r#"
This is an mcp server primarily for retrieving documents from vector stores.
For testing we use a fictional world called Nexus Arcana, see the `nexus_rag` tool.
"#.to_string()),
        }
	}

	async fn list_resources(
		&self,
		_request: Option<PaginatedRequestParam>,
		_: RequestContext<RoleServer>,
	) -> Result<ListResourcesResult, McpError> {
		Ok(ListResourcesResult {
			resources: vec![
				self.create_resource_text("str:////Users/to/some/path/", "cwd"),
				self.create_resource_text("memo://insights", "memo-name"),
			],
			next_cursor: None,
		})
	}

	async fn read_resource(
		&self,
		ReadResourceRequestParam { uri }: ReadResourceRequestParam,
		_: RequestContext<RoleServer>,
	) -> Result<ReadResourceResult, McpError> {
		match uri.as_str() {
			"str:////Users/to/some/path/" => {
				let cwd = "/Users/to/some/path/";
				Ok(ReadResourceResult {
					contents: vec![ResourceContents::text(cwd, uri)],
				})
			}
			"memo://insights" => {
				let memo = "Business Intelligence Memo\n\nAnalysis has revealed 5 key insights ...";
				Ok(ReadResourceResult {
					contents: vec![ResourceContents::text(memo, uri)],
				})
			}
			_ => Err(McpError::resource_not_found(
				"resource_not_found",
				Some(json!({
					"uri": uri
				})),
			)),
		}
	}

	async fn list_prompts(
		&self,
		_request: Option<PaginatedRequestParam>,
		_: RequestContext<RoleServer>,
	) -> Result<ListPromptsResult, McpError> {
		Ok(ListPromptsResult {
			next_cursor: None,
			prompts: vec![Prompt::new(
				"example_prompt",
				Some(
					"This is an example prompt that takes one required argument, message",
				),
				Some(vec![PromptArgument {
					name: "message".to_string(),
					description: Some(
						"A message to put in the prompt".to_string(),
					),
					required: Some(true),
				}]),
			)],
		})
	}

	async fn get_prompt(
		&self,
		GetPromptRequestParam { name, arguments }: GetPromptRequestParam,
		_: RequestContext<RoleServer>,
	) -> Result<GetPromptResult, McpError> {
		match name.as_str() {
			"example_prompt" => {
				let message = arguments
					.and_then(|json| {
						json.get("message")?.as_str().map(|s| s.to_string())
					})
					.ok_or_else(|| {
						McpError::invalid_params(
							"No message provided to example_prompt",
							None,
						)
					})?;

				let prompt = format!(
					"This is an example prompt with your message here: '{message}'"
				);
				Ok(GetPromptResult {
					description: None,
					messages: vec![PromptMessage {
						role: PromptMessageRole::User,
						content: PromptMessageContent::text(prompt),
					}],
				})
			}
			_ => Err(McpError::invalid_params("prompt not found", None)),
		}
	}

	async fn list_resource_templates(
		&self,
		_request: Option<PaginatedRequestParam>,
		_: RequestContext<RoleServer>,
	) -> Result<ListResourceTemplatesResult, McpError> {
		Ok(ListResourceTemplatesResult {
			next_cursor: None,
			resource_templates: Vec::new(),
		})
	}

	async fn initialize(
		&self,
		_request: InitializeRequestParam,
		context: RequestContext<RoleServer>,
	) -> Result<InitializeResult, McpError> {
		if let Some(http_request_part) =
			context.extensions.get::<axum::http::request::Parts>()
		{
			let initialize_headers = &http_request_part.headers;
			let initialize_uri = &http_request_part.uri;
			tracing::info!(?initialize_headers, %initialize_uri, "initialize from http server");
		}
		Ok(self.get_info())
	}
}
