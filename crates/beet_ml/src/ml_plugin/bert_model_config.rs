/// Config containing both native and wasm urls for a model
#[derive(Debug, Clone)]
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

	pub fn default_list() -> Vec<Self> {
		vec![
			Self::intfloat_e5_small_v2(),
			Self::intfloat_e5_base_v2(),
			Self::intfloat_multilingual_e5_small(),
			Self::sentence_transformers_all_MiniLM_L6_v2(),
			Self::sentence_transformers_all_MiniLM_L12_v2(),
		]
	}


	pub fn intfloat_e5_small_v2() -> Self {
		Self {
			base_url: String::from(
				"https://huggingface.co/intfloat/e5-small-v2/resolve/main/",
			),
			search_prefix: String::from("query: "),
			document_prefix: String::from("passage: "),
			model_id: String::from("intfloat/e5-small-v2"),
			revision: String::from("main"),
		}
	}

	pub fn intfloat_e5_base_v2() -> Self {
		Self {
			base_url: String::from(
				"https://huggingface.co/intfloat/e5-base-v2/resolve/main/",
			),
			search_prefix: String::from("query: "),
			document_prefix: String::from("passage:"),
			model_id: String::from("intfloat/e5-base-v2"),
			revision: String::from("main"),
		}
	}

	pub fn intfloat_multilingual_e5_small() -> Self {
		Self {
			base_url: String::from("https://huggingface.co/intfloat/multilingual-e5-small/resolve/main/"),
			search_prefix: String::from("query: "),
			document_prefix: String::from("passage: "),
			model_id: String::from("intfloat/multilingual-e5-small"),
			revision: String::from("main"),
		}
	}

	#[allow(non_snake_case)]
	pub fn sentence_transformers_all_MiniLM_L6_v2() -> Self {
		Self {
			base_url: String::from("https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/refs%2Fpr%2F21/"),
			search_prefix: String::new(),
			document_prefix: String::new(),
			model_id: String::from("sentence-transformers/all-MiniLM-L6-v2"),
			revision: String::from("main"),
		}
	}
	#[allow(non_snake_case)]
	pub fn new_default() -> Self {
		let mut model = Self::sentence_transformers_all_MiniLM_L6_v2();
		// guaranteed sability for default
		model.revision = "refs/pr/21".to_string();
		model
	}

	#[allow(non_snake_case)]
	pub fn sentence_transformers_all_MiniLM_L12_v2() -> Self {
		Self {
			base_url: String::from("https://huggingface.co/sentence-transformers/all-MiniLM-L12-v2/resolve/refs%2Fpr%2F4/"),
			search_prefix: String::new(),
			document_prefix: String::new(),
			model_id: String::from("sentence-transformers/all-MiniLM-L12-v2"),
			revision: String::from("main"),
		}
	}
}
