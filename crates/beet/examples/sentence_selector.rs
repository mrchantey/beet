use beet::prelude::*;

fn main() {
	pretty_env_logger::try_init().ok();

	let mut bert = Bert::new(BertConfig::default()).unwrap();
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

	let results = embeddings.scores(0).unwrap();
	assert_eq!(
		embeddings.sentences[results[0].0].as_ref(),
		"The cat plays in the garden"
	);
}
