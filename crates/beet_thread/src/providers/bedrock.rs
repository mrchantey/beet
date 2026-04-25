use crate::prelude::*;
use beet_core::prelude::*;

pub struct BedrockProvider;

impl BedrockProvider {
	pub const AUTH_ENV: &str = "AWS_BEARER_TOKEN_BEDROCK";

	/// Provider slug for OpenAI.
	pub const PROVIDER_SLUG: &str = "aws-bedrock";

	/// OpenAI Responses API URL.
	pub const RESPONSES_URL: &str =
		"https://bedrock-mantle.us-east-1.api.aws/v1/responses";
	pub const COMPLETIONS_URL: &str =
		"https://bedrock-mantle.us-east-1.api.aws/v1/chat/completions";

	pub const KIMI_K2_5: &str = "moonshotai.kimi-k2.5";

	/// Returns an [`O11sStreamer`] configured for GPT-5 Mini
	/// via the OpenResponses endpoint.
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
