use beet_net::exports::eventsource_stream;
use beet_net::prelude::*;
use beet_utils::utils::SendBoxedFuture;
use bevy::prelude::*;
use bevy::tasks::futures_lite::Stream;
use serde_json::Value;
use std::pin::Pin;
use std::task::Poll;

#[derive(Component)]
pub struct Agent {
	provider: Box<dyn AgentProvider>,
}

impl Agent {
	pub fn new(provider: impl AgentProvider) -> Self {
		Self {
			provider: Box::new(provider),
		}
	}
	pub fn stream_completion(
		&self,
		req: impl Into<CompletionsRequest>,
	) -> SendBoxedFuture<Result<CompletionsStream>> {
		self.provider.stream_completion(req.into())
	}
}

pub trait AgentProvider: 'static + Send + Sync {
	fn stream_completion(
		&self,
		request: CompletionsRequest,
	) -> SendBoxedFuture<Result<CompletionsStream>>;
}


pub trait CompletionProvider {
	fn send(content: &ContentList) -> impl Future<Output = Result>;
}

pub struct CompletionsRequest {
	pub content: Vec<Content>,
	pub reasoning: Option<ReasoningEffort>,
}
impl CompletionsRequest {
	fn new(content: Vec<Content>) -> Self {
		Self {
			content,
			reasoning: None,
		}
	}
}

impl Into<CompletionsRequest> for &str {
	fn into(self) -> CompletionsRequest {
		CompletionsRequest::new(vec![Content::User(InputContent::Text(
			TextContent(self.to_string()),
		))])
	}
}


pub enum ReasoningEffort {
	Min,
	Max,
}

pub struct ContentList {
	content: Vec<Content>,
}

pub enum Content {
	/// Input content that takes precedence over [`Content::User`],
	/// this should only be used for trusted input
	System(InputContent),
	/// Input content from the user
	User(InputContent),
	/// Output content from the agent
	Agent(OutputContent),
}


pub enum InputContent {
	Text(TextContent),
}
pub enum OutputContent {
	Text(TextContent),
}

pub struct TextContent(pub String);

pub struct CompletionsStream {
	pub(super) stream: eventsource_stream::EventStream<Body>,
	pub(super) map_data: Box<
		dyn Send
			+ Sync
			+ Fn(eventsource_stream::Event) -> Result<CompletionsEvent>,
	>,
}

pub enum CompletionsEvent {
	Created {
		/// A string representing this session with the provider.
		/// The 'response id' in openai
		session_id: String,
	},
	TextDelta(String),
	Other {
		value: Value,
	},
	/// Couldnt extract as json, nothing to return
	NoJson(eventsource_stream::Event),
}

impl Stream for CompletionsStream {
	type Item = Result<CompletionsEvent>;

	fn poll_next(
		mut self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> Poll<Option<Self::Item>> {
		let stream = &mut self.stream;
		match Pin::new(stream).poll_next(cx) {
			Poll::Ready(Some(Ok(body))) => {
				let event = (self.map_data)(body);
				Poll::Ready(Some(event))
			}
			Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e.into()))),
			Poll::Ready(None) => Poll::Ready(None),
			Poll::Pending => Poll::Pending,
		}
	}
}

// #[derive(Component)]
// pub enum Content<'a> {
// 	Text(Cow<'a, str>),
// 	ImageBytes {
// 		bytes: Cow<'a, [u8]>,
// 		mime_type: Cow<'a, str>,
// 	},
// }



pub trait ImageProvider {}


// #[cfg(test)]
// mod test {
// 	use crate::prelude::*;
// 	use sweet::prelude::*;

// 	#[test]
// 	fn works() {
// 		expect(true).to_be_false();

// 	}

// }
