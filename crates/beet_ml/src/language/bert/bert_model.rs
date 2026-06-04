use crate::language::bert::BertEmbeddingsConfig;
use crate::language::bert::bert_embedding::BertEmbeddings;
use burn::config::Config;
use burn::module::Module;
use burn::nn::Initializer::KaimingUniform;
use burn::nn::transformer::TransformerEncoder;
use burn::nn::transformer::TransformerEncoderConfig;
use burn::nn::transformer::TransformerEncoderInput;
use burn::tensor::Bool;
use burn::tensor::Int;
use burn::tensor::Tensor;
use burn::tensor::backend::Backend;

/// HuggingFace-compatible bert model config (sentence-transformers
/// MiniLM family). Load from a `config.json` via [`serde_json`]; extra
/// fields are ignored.
#[derive(Config, Debug)]
pub struct BertModelConfig {
	pub hidden_size: usize,
	pub num_attention_heads: usize,
	pub num_hidden_layers: usize,
	pub intermediate_size: usize,
	pub vocab_size: usize,
	pub max_position_embeddings: usize,
	pub type_vocab_size: usize,
	pub hidden_dropout_prob: f64,
	pub layer_norm_eps: f64,
}

/// Bert encoder for sentence embeddings.
#[derive(Module, Debug)]
pub struct BertModel<B: Backend> {
	pub(crate) embeddings: BertEmbeddings<B>,
	pub(crate) encoder: TransformerEncoder<B>,
}

/// Output of [`BertModel::forward`].
#[derive(Debug, Clone)]
pub struct BertOutput<B: Backend> {
	/// Hidden states from the last encoder layer, shape `[batch, seq_len, hidden]`.
	pub hidden_states: Tensor<B, 3>,
}

impl BertModelConfig {
	/// Initialise a model with random weights on `device`.
	pub fn init<B: Backend>(&self, device: &B::Device) -> BertModel<B> {
		BertModel {
			embeddings: self.embeddings_config().init(device),
			encoder: self.encoder_config().init(device),
		}
	}

	fn embeddings_config(&self) -> BertEmbeddingsConfig {
		BertEmbeddingsConfig::new(
			self.vocab_size,
			self.max_position_embeddings,
			self.type_vocab_size,
			self.hidden_size,
			self.hidden_dropout_prob,
			self.layer_norm_eps,
		)
	}

	fn encoder_config(&self) -> TransformerEncoderConfig {
		TransformerEncoderConfig::new(
			self.hidden_size,
			self.intermediate_size,
			self.num_attention_heads,
			self.num_hidden_layers,
		)
		.with_dropout(self.hidden_dropout_prob)
		// BERT uses post-LayerNorm
		.with_norm_first(false)
		.with_quiet_softmax(false)
		.with_initializer(KaimingUniform {
			gain: 1.0 / 3.0f64.sqrt(),
			fan_out_only: false,
		})
	}
}

impl<B: Backend> BertModel<B> {
	/// Forward pass.
	///
	/// * `input_ids`: `[batch, seq_len]`
	/// * `attention_mask`: `[batch, seq_len]` — 1.0 for real tokens, 0.0 for padding
	/// * `token_type_ids`: optional `[batch, seq_len]`
	pub fn forward(
		&self,
		input_ids: Tensor<B, 2, Int>,
		attention_mask: Tensor<B, 2>,
		token_type_ids: Option<Tensor<B, 2, Int>>,
	) -> BertOutput<B> {
		let embeddings = self.embeddings.forward(input_ids, token_type_ids);

		// convert (1=real, 0=pad) → bool mask where true = padding
		let device = attention_mask.device();
		let zeros = Tensor::<B, 2>::zeros(attention_mask.shape(), &device);
		let mask_pad: Tensor<B, 2, Bool> = attention_mask.equal(zeros);

		let encoder_input =
			TransformerEncoderInput::new(embeddings).mask_pad(mask_pad);
		BertOutput {
			hidden_states: self.encoder.forward(encoder_input),
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::language::bert::DefaultBackend;
	use crate::language::bert::default_device;
	use crate::language::bert::mean_pool;
	use crate::language::bert::normalize_l2;
	use beet_core::prelude::*;

	/// Architectural smoke test: build a tiny bert with random weights
	/// and run a forward pass + pooling. Verifies the burn modules wire
	/// together and that shape arithmetic round-trips. Real parity
	/// against pretrained weights is exercised by the `works` test in
	/// [`bert_asset`](super::bert_asset).
	#[beet_core::test]
	fn forward_smoke() {
		use burn::tensor::Tensor;

		let device = default_device();
		let config = BertModelConfig::new(
			16,    // hidden_size
			2,     // num_attention_heads
			1,     // num_hidden_layers
			32,    // intermediate_size
			128,   // vocab_size
			32,    // max_position_embeddings
			2,     // type_vocab_size
			0.0,   // hidden_dropout_prob
			1e-12, // layer_norm_eps
		);
		let model = config.init::<DefaultBackend>(&device);

		// batch=2, seq_len=4, one padded token in row 0
		let input_ids =
			Tensor::<DefaultBackend, 1, burn::tensor::Int>::from_data(
				[7i64, 9, 11, 0, 4, 5, 6, 7].as_slice(),
				&device,
			)
			.reshape([2, 4]);
		let attention_mask = Tensor::<DefaultBackend, 1>::from_data(
			[1.0f32, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0].as_slice(),
			&device,
		)
		.reshape([2, 4]);
		let out = model.forward(input_ids, attention_mask.clone(), None);
		out.hidden_states.dims().xpect_eq([2, 4, 16]);

		let pooled = mean_pool(out.hidden_states, attention_mask);
		pooled.dims().xpect_eq([2, 16]);

		let normed = normalize_l2(pooled);
		normed.dims().xpect_eq([2, 16]);
	}
}
