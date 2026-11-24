use crate::prelude::*;
use beet_core::prelude::*;

/// An OpenAI-compatible provider for AWS Bedrock.
///
/// The key is read from the `AWS_BEARER_TOKEN_BEDROCK` environment variable.
pub struct BedrockProvider;

impl BedrockProvider {
	pub const AUTH_ENV: &str = "AWS_BEARER_TOKEN_BEDROCK";

	/// Provider slug for Bedrock.
	pub const PROVIDER_SLUG: &str = "aws-bedrock";

	/// Bedrock Responses API URL.
	pub const RESPONSES_URL: &str =
		"https://bedrock-mantle.us-east-1.api.aws/v1/responses";
	/// Bedrock Chat Completions API URL.
	pub const COMPLETIONS_URL: &str =
		"https://bedrock-mantle.us-east-1.api.aws/v1/chat/completions";

	pub const KIMI_K2_5: &str = "moonshotai.kimi-k2.5";

	/// Returns a [`CompletionsStreamer`] configured for Kimi K2.5
	/// via the Chat Completions endpoint.
	pub fn kimi_k2_5() -> Result<CompletionsStreamer> {
		CompletionsStreamer::new(ModelDef {
			provider_slug: Self::PROVIDER_SLUG.into(),
			model_slug: Self::KIMI_K2_5.into(),
			url: Self::COMPLETIONS_URL.into(),
			auth: EnvVar::new(Self::AUTH_ENV)?.xsome(),
		})
		.xok()
	}
}
