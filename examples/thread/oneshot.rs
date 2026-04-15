use beet::prelude::*;

#[beet::main]
async fn main() {
	let posts = ThreadMut::spawn()
		.insert_actor(Actor::system())
		.insert_post("make like a duck and quack")
		.thread_view()
		.insert_actor(Actor::agent())
		.with_bundle(
			OllamaProvider::default_12gb()
				// disable streaming since we're aggregating
				.without_streaming(),
		)
		.send_and_collect()
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
