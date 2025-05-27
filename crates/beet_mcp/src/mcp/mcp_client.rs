use crate::prelude::*;
use anyhow::Result;
use rmcp::RoleClient;
use rmcp::Service;
use rmcp::model::CallToolRequestParam;
use rmcp::model::CallToolResult;
use rmcp::model::ClientCapabilities;
use rmcp::model::ClientInfo;
use rmcp::model::Implementation;
use rmcp::model::InitializeRequestParam;
use rmcp::service::RunningService;
use rmcp::service::ServiceExt;
use rmcp::transport::ConfigureCommandExt;
use rmcp::transport::SseClientTransport;
use rmcp::transport::TokioChildProcess;
use tokio::process::Command;

/// A typed example of a beet mcp client, this is for testing and examples, usually
/// an llm would be querying the mcp server directly
pub struct McpClient<S: Service<RoleClient>> {
	pub service: RunningService<RoleClient, S>,
}

impl<S: Service<RoleClient>> std::ops::Deref for McpClient<S> {
	type Target = RunningService<RoleClient, S>;
	fn deref(&self) -> &Self::Target { &self.service }
}

impl McpClient<()> {
	pub async fn new_stdio_dev() -> Result<Self> {
		let args = "cargo run".to_string();
		let mut args = args.split_whitespace();
		let service = ()
			.serve(TokioChildProcess::new(
				Command::new(args.next().unwrap()).configure(|cmd| {
					while let Some(arg) = args.next() {
						cmd.arg(arg);
					}
				}),
			)?)
			.await?;

		Ok(Self { service })
	}
}

impl McpClient<InitializeRequestParam> {
	/// create new mcp client using Server Sent Events (SSE) transport
	/// ## Example
	/// ```no_run
	/// # use beet_mcp::prelude::*;
	/// # tokio_test::block_on(async {
	/// let client = McpClient::new_sse("http://localhost:8000/sse").await.unwrap();
	/// let tools = client.list_tools(Default::default()).await.unwrap();
	/// println!("Available tools: {:#?}", tools);
	/// # })
	/// ```
	pub async fn new_sse(url: &str) -> Result<Self> {
		let transport = SseClientTransport::start(url).await?;
		let client_info = ClientInfo {
			protocol_version: Default::default(),
			capabilities: ClientCapabilities::default(),
			client_info: Implementation {
				name: "beet_mcp client".to_string(),
				version: env!("CARGO_PKG_VERSION").to_string(),
			},
		};
		let service = client_info.serve(transport).await?;
		Ok(Self { service })
	}
}

impl<S: Service<RoleClient>> McpClient<S> {
	pub async fn nexus_rag(&self, query: &RagQuery) -> Result<CallToolResult> {
		let tool_result = self
			.service
			.call_tool(CallToolRequestParam {
				name: "nexus_rag".into(),
				arguments: serde_json::to_value(query)?.as_object().cloned(),
			})
			.await?;
		tracing::debug!("Tool result: {tool_result:#?}");
		Ok(tool_result)
	}

	pub async fn crate_rag(
		&self,
		crate_query: CrateRagQuery,
	) -> Result<CallToolResult> {
		let tool_result = self
			.service
			.call_tool(CallToolRequestParam {
				name: "crate_rag".into(),
				arguments: serde_json::to_value(crate_query)?
					.as_object()
					.cloned(),
			})
			.await?;
		tracing::debug!("Tool result: {tool_result:#?}");
		Ok(tool_result)
	}

	pub async fn cancel(self) -> Result<()> {
		self.service.cancel().await?;
		Ok(())
	}
}
