//! Sentence-similarity demo using the bert sentence transformer.
//!
//! On first run this downloads the all-MiniLM-L6-v2 weights (≈90MB)
//! through [`fetch_ext::fetch_bytes`] and caches them under the system temp dir;
//! subsequent runs reuse the cache. To bypass the network entirely call
//! [`Bert::from_bytes`] with bytes you've loaded yourself.
use beet_core::prelude::*;
use beet_ml::prelude::*;

#[beet_core::main]
async fn main() -> Result {
	PrettyTracing::default().init();
	let mut bert = Bert::new(BertConfig::default()).await?;
	let embeddings = bert.get_embeddings([
		"The cat sits outside",
		"A man is playing guitar",
		"I love pasta",
		"The new movie is awesome",
		"The cat plays in the garden",
		"A woman watches TV",
		"The new movie is so great",
		"Do you like pizza?",
	])?;

	let results = embeddings.scores_sorted(0)?;
	assert_eq!(
		embeddings.sentences[results[0].0].as_str(),
		"The cat plays in the garden"
	);

	info!("Results: {:?}", results);
	Ok(())
}
