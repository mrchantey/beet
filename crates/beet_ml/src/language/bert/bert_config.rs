use serde::Deserialize;
use serde::Serialize;

/// User-facing config: identifies which HuggingFace model to download
/// and how the resulting embeddings should be post-processed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BertConfig {
	/// Repo identification and download URLs.
	pub model: BertSourceConfig,
	/// L2-normalise embeddings after pooling (matches sentence-transformers).
	pub normalize_embeddings: bool,
}

impl Default for BertConfig {
	fn default() -> Self {
		Self {
			model: BertSourceConfig::default(),
			normalize_embeddings: true,
		}
	}
}

impl BertConfig {
	/// Build a [`BertConfig`] from a [`BertSourceConfig`].
	pub fn new(model: BertSourceConfig) -> Self {
		Self {
			model,
			normalize_embeddings: true,
		}
	}
}

/// Identifies a remote bert model and the urls used to download its
/// config, weights, and tokenizer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BertSourceConfig {
	/// Base URL containing `config.json`, `model.safetensors`, etc.
	pub base_url: String,
	/// Query-time prefix (sentence-transformers convention).
	pub search_prefix: String,
	/// Document-time prefix (sentence-transformers convention).
	pub document_prefix: String,
	/// HuggingFace model id.
	pub model_id: String,
	/// HuggingFace revision (branch or commit).
	pub revision: String,
}

impl Default for BertSourceConfig {
	fn default() -> Self {
		Self {
			base_url: "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/refs%2Fpr%2F21/".into(),
			search_prefix: "".into(),
			document_prefix: "".into(),
			model_id: "sentence-transformers/all-MiniLM-L6-v2".into(),
			revision: "refs/pr/21".into(),
		}
	}
}

impl BertSourceConfig {
	/// URL of `model.safetensors`.
	pub fn model_url(&self) -> String {
		self.base_url.clone() + "model.safetensors"
	}

	/// URL of `config.json`.
	pub fn config_url(&self) -> String { self.base_url.clone() + "config.json" }

	/// URL of `tokenizer.json`.
	pub fn tokenizer_url(&self) -> String {
		self.base_url.clone() + "tokenizer.json"
	}
}
