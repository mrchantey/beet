use beet_core::prelude::*;
use burn::tensor::Device;
use burn::tensor::Int;
use burn::tensor::Tensor;

/// Batch-tokenise `sentences` and produce padded input tensors.
///
/// Returns `(input_ids, attention_mask)` both shaped `[batch, max_seq_len]`.
pub(crate) fn tokenize_batch(
	tokenizer: &tokenizers::Tokenizer,
	sentences: &[&str],
	device: &Device,
) -> Result<(Tensor<2, Int>, Tensor<2>)> {
	let encodings = tokenizer
		.encode_batch(sentences.to_vec(), true)
		.map_err(|err| bevyhow!("failed to tokenize: {err}"))?;

	let max_len = encodings
		.iter()
		.map(|e| e.get_ids().len())
		.max()
		.unwrap_or(0);
	let batch_size = sentences.len();

	let mut input_ids_data = vec![0i64; batch_size * max_len];
	let mut attention_mask_data = vec![0.0f32; batch_size * max_len];

	for (i, encoding) in encodings.iter().enumerate() {
		let ids = encoding.get_ids();
		let mask = encoding.get_attention_mask();
		for (j, &id) in ids.iter().enumerate() {
			input_ids_data[i * max_len + j] = id as i64;
			attention_mask_data[i * max_len + j] = mask[j] as f32;
		}
	}

	let input_ids =
		Tensor::<1, Int>::from_data(input_ids_data.as_slice(), device)
			.reshape([batch_size, max_len]);
	let attention_mask =
		Tensor::<1>::from_data(attention_mask_data.as_slice(), device)
			.reshape([batch_size, max_len]);

	(input_ids, attention_mask).xok()
}
