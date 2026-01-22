use crate::openresponses;
use beet_core::prelude::*;
pub mod ollama;
use beet_net::exports::eventsource_stream::EventStream;
pub use ollama::OllamaProvider;

pub trait ModelProvider {
	/// The recommended model from this provider for short and simple
	/// requests and responses
	fn default_small_model(&self) -> &'static str;
	/// The recommended model from this provider for simple tool calls
	fn default_tool_model(&self) -> &'static str;
	/// The recommended model from this provider for complex tasks and reasoning
	fn default_large_model(&self) -> &'static str;

	fn send(
		&mut self,
		request: openresponses::RequestBody,
	) -> impl Future<Output = Result<openresponses::ResponseBody>>;
	fn stream(
		&mut self,
		request: openresponses::RequestBody,
	) -> impl Future<Output = Result<EventStream<Body>>>;
}
