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

	/// Using ollama, get the model from `BEET_OLLAMA_AGENT`, defaulting to `qwen3:latest`.
	pub fn ollama_from_env() -> ollama::EmbeddingModel {
		ollama::EmbeddingModel::new(
			ollama::Client::new(),
			"mxbai-embed-large",
			1024,
		)
	}

	/// Using the `OPENAI_API_KEY`, get the model from `BEET_OPENAI_AGENT`, defaulting to [`openai::TEXT_EMBEDDING_ADA_002`].
	pub fn openai_from_env() -> openai::EmbeddingModel {
		dotenv::dotenv().ok();
		let model_name = std::env::var("BEET_OPENAI_AGENT")
			.unwrap_or(openai::TEXT_EMBEDDING_ADA_002.to_string());
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

	/// Using ollama, get the model from `BEET_OLLAMA_AGENT`, defaulting to `qwen3:latest`.
	pub fn ollama_from_env() -> AgentBuilder<ollama::CompletionModel> {
		dotenv::dotenv().ok();
		let model_name = std::env::var("BEET_OLLAMA_AGENT")
			.unwrap_or("qwen3:latest".to_string());
		ollama::Client::new().agent(&model_name)
	}

	/// Using the `OPENAI_API_KEY`, get the model from `BEET_OPENAI_AGENT`, defaulting to [`openai::GPT_4_1`].
	pub fn openai_from_env() -> AgentBuilder<openai::CompletionModel> {
		dotenv::dotenv().ok();
		let model_name = std::env::var("BEET_OPENAI_AGENT")
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
}


impl Model for openai::CompletionModel {
	fn model_name(&self) -> &str { self.model.as_str() }
	fn provider_name(&self) -> &str { "openai" }
	fn max_tokens_per_request(&self) -> usize {
		// untested, apparently the max for completion models
		4096
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
}
impl Model for ollama::EmbeddingModel {
	fn model_name(&self) -> &str { self.model.as_str() }
	fn provider_name(&self) -> &str { "ollama" }
	fn max_tokens_per_request(&self) -> usize {
		// local dev doesnt have a max?
		usize::MAX
	}
}
