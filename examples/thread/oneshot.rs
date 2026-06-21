use beet::prelude::*;

#[beet::main]
async fn main() {
	env_ext::load_dotenv();

	let posts = run_oneshot(children![
		(
			Actor::system(),
			children![Post::spawn("make like a duck and quack")]
		),
		(
			Actor::agent(),
			// disable streaming since we're aggregating
			BedrockProvider::kimi_k2_5().unwrap().without_streaming(),
		),
	])
	.await
	.unwrap();

	// hide reasoning in release builds
	#[cfg(not(debug_assertions))]
	let posts = posts
		.into_iter()
		.filter(|post| post.intent().is_display())
		.collect::<Vec<_>>();

	println!("{posts:#?}");
}
