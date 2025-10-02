use beet_core::prelude::*;
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


	/// This filteres out the sentence at the given index and returns the rest.
	/// Given a sentence index, returns a list of all other sentences indices and their score,
	/// sorted by score in descending order. Scores are in a range of 0..1 where 1 is the most similar.
	pub fn scores_sorted(&self, index: usize) -> Result<Vec<(usize, f32)>> {
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

	/// - Assumes the first sentence is the target to score against.
	/// - Returns a Vec of size N-1 where N is the number of sentences.
	/// - This filteres out the sentence at the given index and returns the rest.
	/// - Map back to the original [Target,...Options] vec.
	pub fn scores_from_first(&self) -> Result<Vec<f32>> {
		let e_i = self.embeddings.get(0)?;
		let mut results = Vec::with_capacity(self.sentences.len() - 1);
		for i in 1..self.sentences.len() {
			let e_j = self.embeddings.get(i)?;
			let similarity = Self::similarity(&e_i, &e_j)?;
			results.push(similarity);
		}
		Ok(results)
	}

	fn similarity(a: &Tensor, b: &Tensor) -> Result<f32> {
		let sum_ij = (a * b)?.sum_all()?.to_scalar::<f32>()?;
		let sum_i2 = (a * a)?.sum_all()?.to_scalar::<f32>()?;
		let sum_j2 = (b * b)?.sum_all()?.to_scalar::<f32>()?;
		Ok(sum_ij / (sum_i2 * sum_j2).sqrt())
	}
}
