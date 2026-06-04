//! Gemini provider supporting the OpenAI-compatible Chat Completions endpoint.
//!
//! For new code, prefer [`GeminiProvider::gemini_2_5_flash`] which uses
//! the completions-based [`CompletionsStreamer`].
use crate::prelude::*;
use beet_core::prelude::*;

/// An OpenResponses-compatible provider for Google Gemini API.
///
/// Gemini API key must be set via the `GEMINI_API_KEY` environment variable.
pub struct GeminiProvider;

impl GeminiProvider {
	/// Provider slug for Gemini.
	pub const PROVIDER_SLUG: &str = "gemini";
	/// Environment variable for the Gemini API key.
	pub const AUTH_ENV: &str = "GEMINI_API_KEY";

	/// Gemini 2.5 Flash - fast and efficient.
	pub const GEMINI_2_5_FLASH: &str = "gemini-2.5-flash";
	/// Gemini 2.5 Flash with image generation support.
	pub const GEMINI_2_5_FLASH_IMAGE: &str = "gemini-2.5-flash-preview-05-20";
	/// Gemini 2.5 Pro - most capable.
	pub const GEMINI_2_5_PRO: &str = "gemini-2.5-pro";

	/// OpenAI-compatible completions endpoint for Gemini.
	pub const COMPLETIONS_URL: &str = "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions";

	/// Returns a [`CompletionsStreamer`] configured for Gemini 2.5 Flash
	/// via the OpenAI-compatible completions endpoint.
	pub fn gemini_2_5_flash() -> Result<CompletionsStreamer> {
		CompletionsStreamer::new(ModelDef {
			provider_slug: Self::PROVIDER_SLUG.into(),
			model_slug: Self::GEMINI_2_5_FLASH.into(),
			url: Self::COMPLETIONS_URL.into(),
			auth: EnvVar::new(Self::AUTH_ENV)?.xsome(),
		})
		.xok()
	}
}
