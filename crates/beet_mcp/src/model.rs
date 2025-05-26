//! Models that have been tested with beet_mcp
use rig::agent::AgentBuilder;
use rig::providers::ollama;
use rig::providers::openai;


/// Convenience wrapper for [`EmbeddingModel`] with one-liner helper methods
pub struct EmbedModel;

impl EmbedModel {
	/// uses the `OPENAI_API_KEY` environment variable to connect to
	/// the [`openai::TEXT_EMBEDDING_ADA_002`] model.
	/// Loads environment variables from a `.env` file if it exists,
	pub fn openai_ada() -> openai::EmbeddingModel {
		dotenv::dotenv().ok();
		openai::Client::from_env()
			.embedding_model(openai::TEXT_EMBEDDING_ADA_002)
	}
	/// open source model run locally with ollama
	pub fn mxbai_large() -> ollama::EmbeddingModel {
		// ollama::Client::new().embedding_model("mxbai-embed-large")
		ollama::EmbeddingModel::new(
			ollama::Client::new(),
			"mxbai-embed-large",
			1024,
		)
	}
	/// small local model, great for testing
	pub fn all_minilm() -> ollama::EmbeddingModel {
		ollama::EmbeddingModel::new(ollama::Client::new(), "all-minilm", 384)
	}
}


pub struct ChatModel;


impl ChatModel {
	pub fn gpt_4o() -> AgentBuilder<openai::CompletionModel> {
		dotenv::dotenv().ok();
		openai::Client::from_env().agent(openai::GPT_4O)
	}

	pub fn deepseek() -> AgentBuilder<ollama::CompletionModel> {
		ollama::Client::new().agent("deepseek-r1:7b")
	}
}
