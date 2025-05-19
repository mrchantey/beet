use serde::Deserialize;
use serde::Serialize;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BertConfig {
	pub model: BertModelConfig,
	pub normalize_embeddings: bool,
	pub approximate_gelu: bool,
}

impl Default for BertConfig {
	fn default() -> Self {
		Self{
			model: BertModelConfig {
					base_url: "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/refs%2Fpr%2F21/".into(),
					search_prefix: "".into(),
					document_prefix: "".into(),
					model_id: "sentence-transformers/all-MiniLM-L6-v2".into(),
					revision: "refs/pr/21".into(),
		},
			normalize_embeddings: true,
			approximate_gelu: false,
		}
	}
}

impl BertConfig {
	pub fn new(model: BertModelConfig) -> Self {
		Self {
			model,
			normalize_embeddings: true,
			approximate_gelu: false,
		}
	}
}


/// Config containing both native and wasm urls for a model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BertModelConfig {
	pub base_url: String,
	pub search_prefix: String,
	pub document_prefix: String,
	/// used with hf-hub
	pub model_id: String,
	pub revision: String,
}


impl BertModelConfig {
	pub fn model_url(&self) -> String {
		self.base_url.clone() + "model.safetensors"
	}

	pub fn config_url(&self) -> String { self.base_url.clone() + "config.json" }

	pub fn tokenizer_url(&self) -> String {
		// rejected 403 in github actions, use s3 instead
		// self.base_url.clone() + "tokenizer.json"
		"https://bevyhub-public.s3.us-west-2.amazonaws.com/assets/ml/tokenizer.json".into()
	}
}
