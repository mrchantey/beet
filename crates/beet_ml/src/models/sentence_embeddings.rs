use anyhow::Result;
use candle_core::Tensor;
use std::borrow::Cow;


/// Container for a list of sentences and their embeddings.
/// This can be used to calculate the similarity between sentences.
pub struct SentenceEmbeddings {
	pub sentences: Vec<Cow<'static, str>>,
	pub embeddings: Tensor,
}


impl SentenceEmbeddings {
	pub fn new(sentences: Vec<Cow<'static, str>>, embeddings: Tensor) -> Self {
		Self {
			sentences,
			embeddings,
		}
	}


	/// Given a sentence index, returns a list of all other sentences indices and their score,
	/// sorted by score in descending order. Scores are in a range of 0..1 where 1 is the most similar.
	pub fn scores(&self, index: usize) -> Result<Vec<(usize, f32)>> {
		let e_i = self.embeddings.get(index)?;
		let mut results = Vec::with_capacity(self.sentences.len() - 1);
		for i in 0..self.sentences.len() {
			if i == index {
				continue;
			}
			let e_j = self.embeddings.get(i)?;
			let similarity = Self::similarity(&e_i, &e_j)?;
			results.push((i, similarity));
		}
		results.sort_by(|a, b| b.1.total_cmp(&a.1));

		Ok(results)
	}

	fn similarity(a: &Tensor, b: &Tensor) -> Result<f32> {
		let sum_ij = (a * b)?.sum_all()?.to_scalar::<f32>()?;
		let sum_i2 = (a * a)?.sum_all()?.to_scalar::<f32>()?;
		let sum_j2 = (b * b)?.sum_all()?.to_scalar::<f32>()?;
		Ok(sum_ij / (sum_i2 * sum_j2).sqrt())
	}
}
