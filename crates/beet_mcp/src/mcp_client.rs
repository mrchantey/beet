use anyhow::Result;
use rmcp::RoleClient;
use rmcp::Service;
use rmcp::model::CallToolRequestParam;
use rmcp::model::ClientCapabilities;
use rmcp::model::ClientInfo;
use rmcp::model::Implementation;
use rmcp::model::InitializeRequestParam;
use rmcp::model::ListToolsResult;
use rmcp::service::RunningService;
use rmcp::service::ServiceExt;
use rmcp::transport::ConfigureCommandExt;
use rmcp::transport::SseClientTransport;
use rmcp::transport::TokioChildProcess;
use tokio::process::Command;

/// A typed example of a beet mcp client, this is for testing and examples, usually
/// an llm would be querying the mcp server directly
pub struct McpClient<S: Service<RoleClient>> {
	service: RunningService<RoleClient, S>,
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
	pub async fn new_sse() -> Result<Self> {
		let transport =
			SseClientTransport::start("http://localhost:8000/sse").await?;
		let client_info = ClientInfo {
			protocol_version: Default::default(),
			capabilities: ClientCapabilities::default(),
			client_info: Implementation {
				name: "test sse client".to_string(),
				version: "0.0.1".to_string(),
			},
		};
		let service = client_info.serve(transport).await?;
		Ok(Self { service })
	}
}

impl<S: Service<RoleClient>> McpClient<S> {
	pub async fn list_tools(&self) -> Result<ListToolsResult> {
		let tools = self.service.list_tools(Default::default()).await?;
		Ok(tools)
	}
	pub async fn query_nexus(
		&self,
		question: &str,
		max_results: usize,
	) -> Result<()> {
		let tool_result = self
			.service
			.call_tool(CallToolRequestParam {
				name: "query_nexus".into(),
				arguments: serde_json::json!({
					"max_results": max_results,
					"question": question,
				})
				.as_object()
				.cloned(),
			})
			.await?;
		tracing::info!("Tool result: {tool_result:#?}");
		Ok(())
	}

	pub async fn cancel(self) -> Result<()> {
		self.service.cancel().await?;
		Ok(())
	}
}
