use crate::language::bert::*;
use crate::prelude::*;
use beet_core::prelude::*;
use burn::tensor::Tensor;
use burn_store::ModuleSnapshot;
use burn_store::PyTorchToBurnAdapter;
use burn_store::SafetensorsStore;
use std::borrow::Cow;
use tokenizers::Tokenizer;

/// A loaded bert encoder + tokenizer bundled as a bevy [`Asset`].
///
/// Built via [`Bert::from_bytes`] when raw model bytes are already on
/// hand (eg downloaded by [`fetch_bytes`](crate::fetch)) or via
/// [`Bert::new`] which adds the fetch step.
///
/// The default backend is configurable via cargo features — see
/// [`DefaultBackend`].
#[derive(Asset, TypePath)]
pub struct Bert {
	/// User-facing options (eg `normalize_embeddings`).
	pub config: BertConfig,
	/// The encoder.
	pub model: BertModel<DefaultBackend>,
	/// The tokenizer, padded for batch encoding.
	pub tokenizer: Tokenizer,
	/// Device the model lives on.
	pub device: DefaultDevice,
}

impl Bert {
	/// Download the model + tokenizer from the urls in `config` and
	/// return a fully-constructed [`Bert`].
	///
	/// The network/storage work is done by [`fetch_bytes`](crate::fetch::fetch_bytes);
	/// see its docs for the currently supported targets.
	pub async fn new(config: BertConfig) -> Result<Self> {
		let model_config_bytes =
			crate::fetch_bytes::fetch_bytes(&config.model.config_url()).await?;
		let weights_bytes =
			crate::fetch_bytes::fetch_bytes(&config.model.model_url()).await?;
		let tokenizer_bytes =
			crate::fetch_bytes::fetch_bytes(&config.model.tokenizer_url())
				.await?;
		Self::from_bytes(
			config,
			&model_config_bytes,
			weights_bytes,
			&tokenizer_bytes,
		)
	}

	/// Construct a [`Bert`] from pre-loaded bytes.
	///
	/// * `model_config_bytes` — the HuggingFace `config.json` payload
	/// * `weights_bytes` — the safetensors-encoded weights
	/// * `tokenizer_bytes` — the tokenizer.json payload
	pub fn from_bytes(
		config: BertConfig,
		model_config_bytes: &[u8],
		weights_bytes: Vec<u8>,
		tokenizer_bytes: &[u8],
	) -> Result<Self> {
		let device = default_device();

		let model_config: BertModelConfig =
			serde_json::from_slice(model_config_bytes)?;
		let mut model = model_config.init::<DefaultBackend>(&device);
		load_safetensors(&mut model, weights_bytes)?;

		let mut tokenizer = Tokenizer::from_bytes(tokenizer_bytes)
			.map_err(|e| bevyhow!("tokenizer parse: {e}"))?;
		// align padding behaviour with batch tokenisation
		use tokenizers::PaddingParams;
		use tokenizers::PaddingStrategy;
		if let Some(pp) = tokenizer.get_padding_mut() {
			pp.strategy = PaddingStrategy::BatchLongest;
		} else {
			tokenizer.with_padding(Some(PaddingParams {
				strategy: PaddingStrategy::BatchLongest,
				..Default::default()
			}));
		}

		Self {
			config,
			model,
			tokenizer,
			device,
		}
		.xok()
	}

	/// Compute sentence embeddings for `options`.
	///
	/// This is the heavy bit — for the all-MiniLM-L6-v2 model on CPU it
	/// is in the order of ~100ms per batch on desktop.
	pub fn get_embeddings(
		&mut self,
		options: Vec<Cow<'static, str>>,
	) -> Result<SentenceEmbeddings> {
		let refs: Vec<&str> = options.iter().map(|s| s.as_ref()).collect();
		let (input_ids, attention_mask) = tokenize_batch::<DefaultBackend>(
			&self.tokenizer,
			&refs,
			&self.device,
		);
		let output =
			self.model.forward(input_ids, attention_mask.clone(), None);
		let pooled = mean_pool(output.hidden_states, attention_mask);
		let pooled = if self.config.normalize_embeddings {
			normalize_l2(pooled)
		} else {
			pooled
		};
		SentenceEmbeddings::new(options, pooled).xok()
	}

	/// Returns the entity from `options` whose [`Sentence`] is closest to
	/// `target`. Entities lacking a [`Sentence`] are skipped.
	pub fn closest_sentence_entity(
		&mut self,
		target: impl Into<Cow<'static, str>>,
		options: impl IntoIterator<Item = Entity>,
		sentences: &Query<&Sentence>,
	) -> Result<Entity> {
		let options = options
			.into_iter()
			.filter_map(|e| sentences.get(e).ok().map(|s| (e, s)))
			.collect::<Vec<_>>();

		self.closest_option_index(target, options.iter().map(|c| c.1.0.clone()))
			.map(|i| options[i].0)
	}

	/// Returns the index of the option whose sentence is closest to `target`.
	pub fn closest_option_index(
		&mut self,
		target: impl Into<Cow<'static, str>>,
		options: impl IntoIterator<Item = Cow<'static, str>>,
	) -> Result<usize> {
		let mut all_sentences = vec![target.into()];
		all_sentences.extend(options);
		let embeddings = self.get_embeddings(all_sentences)?;
		let scores = embeddings.scores_from_first()?;
		scores
			.iter()
			.enumerate()
			.max_by(|a, b| {
				a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
			})
			.ok_or_else(|| bevyhow!("No scores returned"))
			.map(|(i, _)| i)
	}
}

/// Container for a batch of sentence embeddings plus the input strings.
pub struct SentenceEmbeddings {
	/// One entry per input sentence, in input order.
	pub sentences: Vec<Cow<'static, str>>,
	/// Pooled (and optionally normalised) embeddings, shape
	/// `[n_sentences, hidden]`.
	pub embeddings: Tensor<DefaultBackend, 2>,
}

impl SentenceEmbeddings {
	pub(crate) fn new(
		sentences: Vec<Cow<'static, str>>,
		embeddings: Tensor<DefaultBackend, 2>,
	) -> Self {
		Self {
			sentences,
			embeddings,
		}
	}

	/// Cosine similarity of every sentence against the first sentence
	/// (the "target" in [`Bert::closest_option_index`]). Returns `n - 1`
	/// scores, sentence-1 vs all others.
	pub fn scores_from_first(&self) -> Result<Vec<f32>> {
		let [n, _hidden] = self.embeddings.dims();
		if n < 2 {
			bevybail!("scores_from_first requires at least 2 sentences");
		}
		let target = self.embeddings.clone().narrow(0, 0, 1);
		let others = self.embeddings.clone().narrow(0, 1, n - 1);
		// rows are L2-normalised, so cosine sim is just the dot product
		let scores = (others * target).sum_dim(1).reshape([n - 1]);
		scores
			.into_data()
			.to_vec::<f32>()
			.map_err(|e| bevyhow!("failed to extract embedding scores: {e:?}"))
	}

	/// Same as [`scores_from_first`](Self::scores_from_first) but returns
	/// `(sentence_index, score)` sorted by score descending.
	pub fn scores_sorted(
		&self,
		target_idx: usize,
	) -> Result<Vec<(usize, f32)>> {
		if target_idx != 0 {
			bevybail!(
				"scores_sorted currently only supports target_idx = 0 \
				(MiniLM helper)"
			);
		}
		let mut pairs: Vec<(usize, f32)> = self
			.scores_from_first()?
			.into_iter()
			.enumerate()
			// indices in `pairs` correspond to options[1..], so shift back
			.map(|(i, s)| (i + 1, s))
			.collect();
		pairs.sort_by(|a, b| {
			b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
		});
		Ok(pairs)
	}
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// End-to-end sentence-similarity check. Downloads the all-MiniLM-L6-v2
	/// model on first run (cached under the system temp dir afterwards),
	/// runs the bert forward pass, and verifies cosine similarity ranks
	/// the obvious match first.
	///
	/// Network-bound (~90MB download on first run) — `#[ignore]` so test
	/// runs stay fast. Run explicitly via:
	/// `cargo test -p beet_ml --features bevy_default works -- --ignored`.
	#[beet_core::test(timeout_ms = 600_000)]
	#[ignore = "downloads ~90MB of model weights; run with --ignored"]
	async fn works() {
		pretty_env_logger::try_init().ok();
		let mut bert = Bert::new(BertConfig::default()).await.unwrap();
		let embeddings = bert
			.get_embeddings(vec![
				"The cat sits outside".into(),
				"A man is playing guitar".into(),
				"I love pasta".into(),
				"The new movie is awesome".into(),
				"The cat plays in the garden".into(),
				"A woman watches TV".into(),
				"The new movie is so great".into(),
				"Do you like pizza?".into(),
			])
			.unwrap();
		let results = embeddings.scores_sorted(0).unwrap();
		embeddings.sentences[results[0].0]
			.as_ref()
			.xpect_eq("The cat plays in the garden");
	}
}

/// Loads HuggingFace bert safetensors into a fresh [`BertModel`].
///
/// The HuggingFace key naming differs from burn's `TransformerEncoder`;
/// the remap below mirrors the one in `minilm-burn::loader`.
fn load_safetensors(
	model: &mut BertModel<DefaultBackend>,
	bytes: Vec<u8>,
) -> Result<()> {
	use burn_store::KeyRemapper;

	let key_mappings: Vec<(&str, &str)> = vec![
		// strip the `bert.` prefix
		("^bert\\.(.+)", "$1"),
		// `encoder.layer.N` → `encoder.layers.N`
		("encoder\\.layer\\.([0-9]+)", "encoder.layers.$1"),
		// attention
		("attention\\.self\\.query", "mha.query"),
		("attention\\.self\\.key", "mha.key"),
		("attention\\.self\\.value", "mha.value"),
		("attention\\.output\\.dense", "mha.output"),
		("attention\\.output\\.LayerNorm", "norm_1"),
		// feed-forward
		("intermediate\\.dense", "pwff.linear_inner"),
		("(layers\\.[0-9]+)\\.output\\.dense", "$1.pwff.linear_outer"),
		("(layers\\.[0-9]+)\\.output\\.LayerNorm", "$1.norm_2"),
		// embeddings
		("embeddings\\.LayerNorm", "embeddings.layer_norm"),
	];

	let remapper = KeyRemapper::from_patterns(key_mappings)
		.map_err(|e| bevyhow!("KeyRemapper: {e}"))?;
	let mut store = SafetensorsStore::from_bytes(Some(bytes))
		.with_from_adapter(PyTorchToBurnAdapter)
		.remap(remapper);
	model
		.load_from(&mut store)
		.map_err(|e| bevyhow!("load_from: {e}"))?;
	Ok(())
}
