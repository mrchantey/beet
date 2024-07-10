use beet_ml::prelude::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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

	let target_index = 0;

	let results = embeddings.scores_sorted(target_index)?;
	assert_eq!(
		embeddings.sentences[results[0].0].as_ref(),
		"The cat plays in the garden"
	);

	println!("Results: {:?}", results);

	Ok(())
}
