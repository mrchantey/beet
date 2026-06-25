//! Sentence-similarity demo using the bert sentence transformer.
//!
//! On first run this downloads the all-MiniLM-L6-v2 weights (≈90MB)
//! through [`fetch_bytes`] and caches them under the system temp dir;
//! subsequent runs reuse the cache. To bypass the network entirely call
//! [`Bert::from_bytes`] with bytes you've loaded yourself.
use beet_core::prelude::*;
use beet_ml::prelude::*;

#[beet_core::main]
async fn main() -> Result {
	PrettyTracing::default().init();
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

	let results = embeddings.scores_sorted(0)?;
	assert_eq!(
		embeddings.sentences[results[0].0].as_ref(),
		"The cat plays in the garden"
	);

	info!("Results: {:?}", results);
	Ok(())
}
