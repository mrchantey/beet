//https://github.com/huggingface/candle/blob/main/candle-examples/examples/bert/main.rs
use crate::models::sentence_embeddings::SentenceEmbeddings;
use crate::prelude::*;
use anyhow::Error as E;
use anyhow::Result;
use bevy::prelude::*;
use candle_core::Tensor;
use candle_nn::VarBuilder;
use candle_transformers::models::bert::BertModel;
use candle_transformers::models::bert::Config;
use std::borrow::Cow;
use tokenizers::PaddingParams;
use tokenizers::Tokenizer;


#[derive(Asset, TypePath)]
pub struct Bert {
	config: BertConfig,
	model: BertModel,
	tokenizer: Tokenizer,
}

impl Bert {
	/// When native we use the hf-hub which caches the models for use with this and other applications
	#[cfg(not(target_arch = "wasm32"))]
	pub async fn new(config: BertConfig) -> Result<Self> {
		// TODO more async stuff here
		use candle_transformers::models::bert::HiddenAct;
		use candle_transformers::models::bert::DTYPE;
		use hf_hub::api::sync::Api;
		use hf_hub::Repo;
		use hf_hub::RepoType;

		let device = candle_core::Device::Cpu;

		let repo = Repo::with_revision(
			config.model.model_id.clone(),
			RepoType::Model,
			config.model.revision.clone(),
		);
		let api = Api::new()?;
		let api = api.repo(repo);
		let tokenizer_filename = api.get("tokenizer.json")?;
		let weights_filename = api.get("model.safetensors")?;
		let config_filename = api.get("config.json")?;
		let candle_config = std::fs::read_to_string(config_filename)?;
		let mut candle_config: Config = serde_json::from_str(&candle_config)?;
		let tokenizer =
			Tokenizer::from_file(tokenizer_filename).map_err(E::msg)?;

		let vb = unsafe {
			VarBuilder::from_mmaped_safetensors(
				&[weights_filename],
				DTYPE,
				&device,
			)?
		};
		if config.approximate_gelu {
			candle_config.hidden_act = HiddenAct::GeluApproximate;
		}
		let model = BertModel::load(vb, &candle_config)?;
		Ok(Self {
			config,
			model,
			tokenizer,
		})
	}

	#[cfg(target_arch = "wasm32")]
	pub async fn new(config: BertConfig) -> Result<Self> {
		// use super::bert_loader::BertAssetLoaderError;
		use crate::wasm::open_or_fetch;
		use candle_core::DType;

		let config_url = config.model.config_url();
		let model_url = config.model.model_url();
		let tokenizer_url = config.model.tokenizer_url();

		let model_config = open_or_fetch(&config_url).await;
		let weights = open_or_fetch(&model_url).await;
		let tokenizer = open_or_fetch(&tokenizer_url).await;

		// let (model_config, weights, tokenizer) = futures::join!(
		// 	open_or_fetch(&config_url),
		// 	open_or_fetch(&model_url),
		// 	open_or_fetch(&tokenizer_url)
		// );

		let model_config = model_config
			.map_err(|e| anyhow::anyhow!("config fetch error: {:?}", e))?
			.to_vec();
		let model_config: Config = serde_json::from_slice(&model_config)?;

		let weights = weights
			.map_err(|e| anyhow::anyhow!("weights fetch error: {:?}", e))?
			.to_vec();
		let device = &candle_core::Device::Cpu;
		let vb = VarBuilder::from_buffered_safetensors(
			weights,
			candle_transformers::models::bert::DTYPE,
			device,
		)?;
		// VarBuilder::from_buffered_safetensors(weights, DType::F64, device)?;


		let tokenizer = tokenizer
			.map_err(|e| anyhow::anyhow!("tokenizer fetch error: {:?}", e))?
			.to_vec();
		let tokenizer = Tokenizer::from_bytes(&tokenizer)
			.map_err(|m| anyhow::anyhow!(m.to_string()))?;


		let model = BertModel::load(vb, &model_config)?;

		Ok(Self {
			config,
			model,
			tokenizer,
		})
	}

	/// Calculate the embeddings for a list of sentences.
	/// For a small example this may take 0.5 seconds on desktop targets
	/// or 10 seconds on wasm32
	pub fn get_embeddings(
		&mut self,
		options: Vec<Cow<'static, str>>,
	) -> Result<SentenceEmbeddings> {
		if let Some(pp) = self.tokenizer.get_padding_mut() {
			pp.strategy = tokenizers::PaddingStrategy::BatchLongest
		} else {
			let pp = PaddingParams {
				strategy: tokenizers::PaddingStrategy::BatchLongest,
				..Default::default()
			};
			self.tokenizer.with_padding(Some(pp));
		}
		let tokens = self
			.tokenizer
			.encode_batch(options.clone(), true)
			.map_err(E::msg)?;
		let token_ids = tokens
			.iter()
			.map(|tokens| {
				let tokens = tokens.get_ids().to_vec();
				Ok(Tensor::new(tokens.as_slice(), &self.model.device)?)
			})
			.collect::<Result<Vec<_>>>()?;

		let token_ids = Tensor::stack(&token_ids, 0)?;
		let token_type_ids = token_ids.zeros_like()?;
		let embeddings = self.model.forward(&token_ids, &token_type_ids)?;
		// Apply some avg-pooling by taking the mean embedding value for all tokens (including padding)
		let (_n_sentence, n_tokens, _hidden_size) = embeddings.dims3()?;
		let embeddings = (embeddings.sum(1)? / (n_tokens as f64))?;
		let embeddings = if self.config.normalize_embeddings {
			normalize_l2(&embeddings)?
		} else {
			embeddings
		};

		Ok(SentenceEmbeddings::new(options, embeddings))
	}


	/// Score a list of entities with a [`Sentence`] against a root entity with a [`Sentence`]. This returns a list of entities with their sentence and raw cosine similarity scores. 
	/// Scores are in a range of `0..1`, higher means more similar, the list is sorted in descending order.
	/// This calls [`Bert::get_embeddings`] and has the associated performance implications.
	/// If the root is missing a [`Sentence`] an empty vec will be returned.
	/// If options are missing a [`Sentence`] they will be ignored.
	/// The root is filtered out of the options.
	/// # Errors
	/// Will return an error if the embeddings are not calculated correctly.
	pub fn score_sentences(
		&mut self,
		root_entity: Entity,
		options: impl IntoIterator<Item = Entity>,
		sentences: &Query<&Sentence>,
	) -> Result<Vec<(Entity, Sentence, f32)>> {
		let Ok(root_sentence) = sentences.get(root_entity) else {
			return Ok(vec![]);
		};

		let options = options
			.into_iter()
			.filter(|option| option != &root_entity)
			.filter_map(|e| sentences.get(e).ok().map(|s| (e, s)))
			.collect::<Vec<_>>();

		let mut all_sentences = vec![root_sentence.0.clone()];
		all_sentences.extend(options.iter().map(|c| (c.1 .0.clone())));

		let embeddings = self.get_embeddings(all_sentences)?;

		let scores = embeddings
			.scores(0)?
			.into_iter()
			.map(|(score_index, score)| {
				// subtract 1 because the first index is the agent
				let (entity, sentence) = options[score_index - 1];

				(entity, sentence.clone(), score)
			})
			.collect::<Vec<_>>();

		Ok(scores)
	}


	pub fn prompt_tensor(
		&mut self,
		prompt: &str,
		iterations: usize,
	) -> Result<Vec<Tensor>> {
		let tokenizer = self
			.tokenizer
			.with_padding(None)
			.with_truncation(None)
			.map_err(E::msg)?;
		let tokens = tokenizer
			.encode(prompt, true)
			.map_err(E::msg)?
			.get_ids()
			.to_vec();
		let token_ids =
			Tensor::new(&tokens[..], &self.model.device)?.unsqueeze(0)?;
		let token_type_ids = token_ids.zeros_like()?;

		let tensors = (0..iterations)
			.map(|_| self.model.forward(&token_ids, &token_type_ids))
			.collect::<Result<Vec<_>, candle_core::Error>>()?;

		Ok(tensors)
	}
}

fn normalize_l2(v: &Tensor) -> Result<Tensor> {
	Ok(v.broadcast_div(&v.sqr()?.sum_keepdim(1)?.sqrt()?)?)
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use sweet::*;

	#[tokio::test]
	async fn works() -> Result<()> {
		pretty_env_logger::try_init().ok();

		let mut bert = Bert::new(BertConfig::default()).await?;
		let embeddings = bert.get_embeddings(vec![
			"The cat sits outside".into(),
			"A man is playing guitar".into(),
			"I love pasta".into(),
			"The new movie is awesome".into(),
			"The cat plays in the garden".into(),
			"A woman watches TV".into(),
			"The new movie is so great".into(),
			"Do you like pizza?".into(),
		])?;

		let results = embeddings.scores(0)?;
		expect(embeddings.sentences[results[0].0].as_ref())
			.to_be("The cat plays in the garden")?;

		Ok(())
	}
}
