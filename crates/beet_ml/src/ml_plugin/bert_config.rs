use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;
use std::fs;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BertConfig {
	pub model: BertModelConfig,
	pub normalize_embeddings: bool,
	pub approximate_gelu: bool,
}

impl BertConfig {

	pub fn load_default() -> Result<Self> {
		let str = fs::read_to_string("assets/default-bert.ron")?;

		let val = ron::de::from_str(&str)?;

		Ok(val)
	}

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
		self.base_url.clone() + "tokenizer.json"
	}
}
