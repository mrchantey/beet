#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
use beet_core::prelude::*;
use beet_thread::prelude::*;

#[path = "utils/post_streamer.rs"]
mod post_streamer;

fn completions_streamer() -> CompletionsStreamer {
	env_ext::load_dotenv();
	GeminiProvider::gemini_2_5_flash().unwrap()
}

fn completions_streamer_non_streaming() -> CompletionsStreamer {
	env_ext::load_dotenv();
	GeminiProvider::gemini_2_5_flash()
		.unwrap()
		.without_streaming()
}

// === PostStreamer (Completions) tests ===

#[beet_core::test(timeout_ms = 15_000)]
async fn basic_text_response() {
	post_streamer::basic_text_response(completions_streamer_non_streaming())
		.await;
}

#[beet_core::test(timeout_ms = 15_000)]
async fn streaming_response() {
	post_streamer::streaming_response(completions_streamer()).await;
}

#[beet_core::test(timeout_ms = 15_000)]
async fn system_prompt() {
	post_streamer::system_prompt(completions_streamer_non_streaming()).await;
}

#[beet_core::test(timeout_ms = 15_000)]
async fn tool_calling() {
	post_streamer::tool_calling(completions_streamer_non_streaming()).await;
}

#[beet_core::test(timeout_ms = 15_000)]
async fn image_input() {
	post_streamer::image_input(completions_streamer_non_streaming()).await;
}

#[beet_core::test(timeout_ms = 15_000)]
async fn multi_turn_conversation() {
	post_streamer::multi_turn_conversation(
		completions_streamer_non_streaming(),
	)
	.await;
}

// === Image Roundtrip tests ===

#[beet_core::test(timeout_ms = 30_000)]
async fn image_roundtrip() {
	post_streamer::image_roundtrip(completions_streamer_non_streaming()).await;
}
