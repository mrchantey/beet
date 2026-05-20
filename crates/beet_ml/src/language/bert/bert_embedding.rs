use burn::config::Config;
use burn::module::Module;
use burn::nn::Dropout;
use burn::nn::DropoutConfig;
use burn::nn::Embedding;
use burn::nn::EmbeddingConfig;
use burn::nn::LayerNorm;
use burn::nn::LayerNormConfig;
use burn::tensor::Int;
use burn::tensor::Tensor;
use burn::tensor::backend::Backend;

/// Configuration for the bert embedding layer.
#[derive(Config, Debug)]
pub(crate) struct BertEmbeddingsConfig {
	pub vocab_size: usize,
	pub max_position_embeddings: usize,
	pub type_vocab_size: usize,
	pub hidden_size: usize,
	pub hidden_dropout_prob: f64,
	pub layer_norm_eps: f64,
}

/// Word + position + token-type embeddings followed by LayerNorm and dropout.
#[derive(Module, Debug)]
pub struct BertEmbeddings<B: Backend> {
	word_embeddings: Embedding<B>,
	position_embeddings: Embedding<B>,
	token_type_embeddings: Embedding<B>,
	layer_norm: LayerNorm<B>,
	dropout: Dropout,
}

impl BertEmbeddingsConfig {
	pub(crate) fn init<B: Backend>(
		&self,
		device: &B::Device,
	) -> BertEmbeddings<B> {
		BertEmbeddings {
			word_embeddings: EmbeddingConfig::new(
				self.vocab_size,
				self.hidden_size,
			)
			.init(device),
			position_embeddings: EmbeddingConfig::new(
				self.max_position_embeddings,
				self.hidden_size,
			)
			.init(device),
			token_type_embeddings: EmbeddingConfig::new(
				self.type_vocab_size,
				self.hidden_size,
			)
			.init(device),
			layer_norm: LayerNormConfig::new(self.hidden_size)
				.with_epsilon(self.layer_norm_eps)
				.init(device),
			dropout: DropoutConfig::new(self.hidden_dropout_prob).init(),
		}
	}
}

impl<B: Backend> BertEmbeddings<B> {
	/// Forward pass.
	///
	/// * `input_ids`: `[batch, seq_len]`
	/// * `token_type_ids`: optional `[batch, seq_len]` (defaults to zeros)
	///
	/// Returns `[batch, seq_len, hidden]`.
	pub fn forward(
		&self,
		input_ids: Tensor<B, 2, Int>,
		token_type_ids: Option<Tensor<B, 2, Int>>,
	) -> Tensor<B, 3> {
		let [batch_size, seq_len] = input_ids.dims();
		let device = input_ids.device();

		let word = self.word_embeddings.forward(input_ids);

		let position_ids = Tensor::arange(0..seq_len as i64, &device)
			.reshape([1, seq_len])
			.expand([batch_size, seq_len]);
		let pos = self.position_embeddings.forward(position_ids);

		let token_type_ids = token_type_ids
			.unwrap_or_else(|| Tensor::zeros([batch_size, seq_len], &device));
		let ty = self.token_type_embeddings.forward(token_type_ids);

		let embeddings = self.layer_norm.forward(word + pos + ty);
		self.dropout.forward(embeddings)
	}
}
