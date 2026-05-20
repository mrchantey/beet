use burn::tensor::Tensor;
use burn::tensor::backend::Backend;

/// Mean-pool over the sequence dimension, ignoring padded positions.
///
/// Mirrors the pooling used by sentence-transformers:
/// * `hidden_states`: `[batch, seq_len, hidden]`
/// * `attention_mask`: `[batch, seq_len]` — 1.0 for real tokens, 0.0 for padding
///
/// Returns `[batch, hidden]`.
pub fn mean_pool<B: Backend>(
	hidden_states: Tensor<B, 3>,
	attention_mask: Tensor<B, 2>,
) -> Tensor<B, 2> {
	let [batch_size, seq_len, hidden_size] = hidden_states.dims();

	let mask_expanded = attention_mask
		.clone()
		.reshape([batch_size, seq_len, 1])
		.expand([batch_size, seq_len, hidden_size]);

	let sum_hidden: Tensor<B, 2> = (hidden_states * mask_expanded)
		.sum_dim(1)
		.reshape([batch_size, hidden_size]);

	let token_counts: Tensor<B, 2> = attention_mask
		.sum_dim(1)
		.reshape([batch_size, 1])
		.expand([batch_size, hidden_size])
		.clamp_min(1e-9);

	sum_hidden / token_counts
}

/// L2-normalise each row of `embeddings` to unit length.
///
/// Matches the default behaviour of sentence-transformers.
pub fn normalize_l2<B: Backend>(embeddings: Tensor<B, 2>) -> Tensor<B, 2> {
	use burn::tensor::linalg::Norm;
	use burn::tensor::linalg::vector_normalize;
	vector_normalize(embeddings, Norm::L2, 1, 1e-12)
}
