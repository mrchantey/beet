//! OpenAI provider supporting the OpenAI API.
//!
//! OpenAI provides cloud-based LLM inference with OpenResponses-compatible streaming.
use crate::prelude::*;
use base64::prelude::*;
use beet_core::prelude::*;
use bevy::tasks::BoxedFuture;

impl OpenAiProvider {
	/// GPT-5 Nano - smallest and fastest model.
	pub const GPT_5_NANO: &str = "gpt-5-nano";
	/// GPT-5 Mini - balanced speed and capability.
	pub const GPT_5_MINI: &str = "gpt-5-mini";
	/// GPT-5.2 - most capable model.
	pub const GPT_5_2: &str = "gpt-5.2";

	/// OpenAI Responses API URL.
	pub const RESPONSES_URL: &str = "https://api.openai.com/v1/responses";
}

/// An OpenResponses-compatible provider for OpenAI API.
///
/// OpenAI API key must be set via the `OPENAI_API_KEY` environment variable.
/// By default, connects to `https://api.openai.com/v1`.
pub struct OpenAiProvider {
	inner: OpenResponsesProvider,
}

impl Default for OpenAiProvider {
	fn default() -> Self { Self::new().unwrap() }
}

impl OpenAiProvider {
	/// Creates a new provider with the API key from the environment.
	pub fn new() -> Result<Self> {
		let api_key = env_ext::var("OPENAI_API_KEY")?;
		Ok(Self {
			inner: OpenResponsesProvider::new(Self::RESPONSES_URL)
				.with_auth(api_key),
		})
	}

	fn convert_files(
		mut request: openresponses::RequestBody,
	) -> openresponses::RequestBody {
		if let openresponses::request::Input::Items(items) = &mut request.input
		{
			for item in items {
				if let openresponses::request::InputItem::Message(msg) = item {
					if let openresponses::request::MessageContent::Parts(
						parts,
					) = &mut msg.content
					{
						for part in parts {
							if let openresponses::ContentPart::InputFile(file) =
								part
							{
								let text = if let Some(data) = &file.file_data {
									match BASE64_STANDARD.decode(data) {
										Ok(bytes) => String::from_utf8(bytes)
											.unwrap_or_else(|_| {
												"[Binary data]".to_string()
											}),
										Err(_) => {
											"[Invalid base64 data]".to_string()
										}
									}
								} else if let Some(url) = &file.file_url {
									format!("[File URL: {}]", url)
								} else {
									"[Empty file]".to_string()
								};
								let filename = file
									.filename
									.as_deref()
									.unwrap_or("unknown");
								let content =
									format!("File: {}\n\n{}", filename, text);
								*part = openresponses::ContentPart::input_text(
									content,
								);
							}
						}
					}
				}
			}
		}
		request
	}
}

impl ModelProvider for OpenAiProvider {
	fn provider_slug(&self) -> &'static str { "openai" }

	fn default_small_model(&self) -> &'static str { Self::GPT_5_NANO }
	fn default_tool_model(&self) -> &'static str { Self::GPT_5_MINI }
	fn default_large_model(&self) -> &'static str { Self::GPT_5_2 }

	fn send(
		&self,
		request: openresponses::RequestBody,
	) -> BoxedFuture<'_, Result<openresponses::ResponseBody>> {
		let request = Self::convert_files(request);
		Box::pin(self.inner.send(request))
	}

	fn stream(
		&self,
		request: openresponses::RequestBody,
	) -> BoxedFuture<'_, Result<StreamingEventStream>> {
		let request = Self::convert_files(request);
		Box::pin(self.inner.stream(request))
	}
}
