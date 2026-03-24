use beet::prelude::*;

#[beet::main]
async fn main() {
	let actions = ThreadMut::new()
		.insert_actor(Actor::system())
		.insert_post("make like a duck and quack")
		.thread_view()
		.insert_actor(Actor::agent())
		.with_streamer(OllamaProvider::qwen_3_8b().without_streaming())
		.send_and_collect()
		.await
		.unwrap();
	println!("{actions:#?}");
}
