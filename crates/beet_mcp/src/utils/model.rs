//! Models that have been tested with beet_mcp
use rig::agent::AgentBuilder;
use rig::embeddings::EmbeddingModel;
use rig::providers::ollama;
use rig::providers::openai;


/// Convenience wrapper for [`EmbeddingModel`] with one-liner helper methods
pub struct EmbedModel;

impl EmbedModel {
	/// Gets either [`Self::openai_from_env`] or [`Self::ollama_from_env`],
	/// depending on the feature flag `openai`.
	#[cfg(feature = "openai")]
	pub fn from_env() -> openai::EmbeddingModel { Self::openai_from_env() }

	/// Gets either [`Self::openai_from_env`] or [`Self::ollama_from_env`],
	/// depending on the feature flag `openai`.
	#[cfg(not(feature = "openai"))]
	pub fn from_env() -> ollama::EmbeddingModel { Self::ollama_from_env() }

	/// Using ollama, get the model from `BEET_MODEL_EMBED_OLLAMA`
	/// and `BEET_MODEL_EMBED_OLLAMA_DIMS` environment variables.
	/// defaulting to `mxbai-embed-large`.
	pub fn ollama_from_env() -> ollama::EmbeddingModel {
		dotenv::dotenv().ok();
		let (model_name, dims) = match std::env::var("BEET_MODEL_EMBED_OLLAMA")
		{
			Ok(model_name) => {
				let dims = std::env::var("BEET_MODEL_EMBED_OLLAMA_DIMS")
					.expect("when setting BEET_MODEL_EMBED_OLLAMA, you must also set  the number of dimensions via BEET_MODEL_EMBED_OLLAMA_DIMS");
				let dims = dims
					.parse::<usize>()
					.expect("BEET_MODEL_EMBED_OLLAMA_DIMS must be a number");
				(model_name, dims)
			}
			_ => ("mxbai-embed-large".to_string(), 1024),
		};
		ollama::EmbeddingModel::new(ollama::Client::new(), &model_name, dims)
	}

	/// Using the `OPENAI_API_KEY`, get the model from `BEET_MODEL_EMBED_OPENAI`, defaulting to [`openai::TEXT_EMBEDDING_3_LARGE`].
	pub fn openai_from_env() -> openai::EmbeddingModel {
		dotenv::dotenv().ok();
		let model_name = std::env::var("BEET_MODEL_EMBED_OPENAI")
			.unwrap_or(openai::TEXT_EMBEDDING_3_LARGE.to_string());
		openai::Client::from_env().embedding_model(&model_name)
	}

	/// tiny ollama model, great for testing
	pub fn all_minilm() -> ollama::EmbeddingModel {
		ollama::EmbeddingModel::new(
			ollama::Client::new(),
			ollama::ALL_MINILM,
			384,
		)
	}
}

/// One-liner helpers for getting a [`CompletionModel`]
pub struct AgentModel;


impl AgentModel {
	/// Gets either [`Self::openai_from_env`] or [`Self::ollama_from_env`],
	/// depending on the feature flag `openai`.
	#[cfg(feature = "openai")]
	pub fn from_env() -> AgentBuilder<openai::CompletionModel> {
		Self::openai_from_env()
	}
	/// Gets either [`Self::openai_from_env`] or [`Self::ollama_from_env`],
	/// depending on the feature flag `openai`.
	#[cfg(not(feature = "openai"))]
	pub fn from_env() -> AgentBuilder<ollama::CompletionModel> {
		Self::ollama_from_env()
	}

	/// Using ollama, get the model from `BEET_MODEL_AGENT_OLLAMA`, defaulting to `qwen3:latest`.
	pub fn ollama_from_env() -> AgentBuilder<ollama::CompletionModel> {
		dotenv::dotenv().ok();
		let model_name = std::env::var("BEET_MODEL_AGENT_OLLAMA")
			.unwrap_or("qwen3:latest".to_string());
		ollama::Client::new().agent(&model_name)
	}

	/// Using the `OPENAI_API_KEY`, get the model from `BEET_MODEL_AGENT_OPENAI`, defaulting to [`openai::GPT_4_1`].
	pub fn openai_from_env() -> AgentBuilder<openai::CompletionModel> {
		dotenv::dotenv().ok();
		let model_name = std::env::var("BEET_MODEL_AGENT_OPENAI")
			.unwrap_or(openai::GPT_4_1.to_string());
		openai::Client::from_env().agent(&model_name)
	}
}
/// newtype [`EmbeddingModel`] with [`Model`] trait
pub trait BeetEmbedModel: 'static + Clone + EmbeddingModel + Model {}
impl<T> BeetEmbedModel for T where T: 'static + Clone + EmbeddingModel + Model {}

/// Base information for each instance of an
/// [`EmbeddingModel`] or [`CompletionModel`].
pub trait Model {
	/// The underlying model name, irrespective of the provider.
	/// Some providers offer the same model so use [`Model::provider_name`]
	fn model_name(&self) -> &str;
	/// The name of the provider, ie `ollama`, `deepseek`, `openai`
	fn provider_name(&self) -> &str;
	/// The maximum number of tokens in a single api request.
	/// This is different from an agent's 'context window' which is much larger.
	fn max_tokens_per_request(&self) -> usize;
	/// The number of characters per token, used for estimating the number of tokens in a string.
	fn characters_per_token(&self) -> f64 {
		// each model is different but this is a conservative estimate for english text
		4.0
	}
	/// The cost per token of this model in Us if it makes requests to a paid API.
	fn cost_per_input_token(&self) -> Option<f64> { None }
	fn estimate_cost(&self, num_characters: usize) -> Option<f64> {
		self.cost_per_input_token().map(|cost| {
			let tokens =
				(num_characters as f64 / self.characters_per_token()) as usize;
			cost * tokens as f64
		})
	}
}

/// multiply by this to convert from cost per million tokens
/// to cost per token
const MILLION_TO_ONE: f64 = 0.000_001;

impl Model for openai::CompletionModel {
	fn model_name(&self) -> &str { self.model.as_str() }
	fn provider_name(&self) -> &str { "openai" }
	fn max_tokens_per_request(&self) -> usize {
		// untested, apparently the max for completion models
		4096
	}
	#[rustfmt::skip]
	fn cost_per_input_token(&self) -> Option<f64> {
		// https://platform.openai.com/docs/pricing
		let model = self.model.as_str();
		match model {
			_ if model.starts_with("o3") => 					Some(10. * MILLION_TO_ONE),
			_ if model.starts_with("o4-mini") => 			Some(1.1 * MILLION_TO_ONE),
			_ if model.starts_with("gpt-4.1-nano") => Some(0.1 * MILLION_TO_ONE),
			_ if model.starts_with("gpt-4.1-mini") => Some(0.4 * MILLION_TO_ONE),
			_ if model.starts_with("gpt-4.1") => 			Some(2.0 * MILLION_TO_ONE),
			_ => {
				tracing::warn!(
					"Cost per input token not set for model {}, defaulting to gpt-4.1 ($2 per million)",
					model
				);
				Some(2.0 * MILLION_TO_ONE)
			}
		}
	}
}
impl Model for ollama::CompletionModel {
	fn model_name(&self) -> &str { self.model.as_str() }
	fn provider_name(&self) -> &str { "ollama" }
	fn max_tokens_per_request(&self) -> usize {
		// local dev doesnt have a max?
		usize::MAX
	}
}
impl Model for openai::EmbeddingModel {
	fn model_name(&self) -> &str { self.model.as_str() }
	fn provider_name(&self) -> &str { "openai" }
	fn max_tokens_per_request(&self) -> usize {
		// the number i got back when doing an embedding request
		300_000
	}
	#[rustfmt::skip]
	fn cost_per_input_token(&self) -> Option<f64> {
		// https://platform.openai.com/docs/pricing
		let model = self.model.as_str();
		match model {
			_ if model.starts_with("text-embedding-3-small") => 
				Some(0.02 * MILLION_TO_ONE),
			_ if model.starts_with("text-embedding-3-large") => 
				Some(0.13 * MILLION_TO_ONE),
			_ if model.starts_with("text-embedding-ada-002") => 
				Some(0.10 * MILLION_TO_ONE),
			_ => {
				tracing::warn!(
					"Cost per input token not set for model {}, defaulting to gpt-4.1 ($2 per million)",
					model
				);
				Some(2.0 * MILLION_TO_ONE)
			}
		}
	}
}
impl Model for ollama::EmbeddingModel {
	fn model_name(&self) -> &str { self.model.as_str() }
	fn provider_name(&self) -> &str { "ollama" }
	fn max_tokens_per_request(&self) -> usize {
		// local dev doesnt have a max?
		usize::MAX
	}
}
