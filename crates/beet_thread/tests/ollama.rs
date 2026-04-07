#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
use beet_thread::prelude::*;

#[path = "utils/post_streamer.rs"]
mod post_streamer;

fn streamer() -> O11sStreamer { OllamaProvider::qwen() }

fn streamer_non_streaming() -> O11sStreamer {
	OllamaProvider::qwen().without_streaming()
}

fn completions_streamer() -> CompletionsStreamer {
	OllamaProvider::qwen_completions()
}

fn completions_streamer_non_streaming() -> CompletionsStreamer {
	OllamaProvider::qwen_completions().without_streaming()
}

// === PostStreamer (O11s) tests ===

#[ignore = "requires Ollama running locally"]
#[beet_core::test(timeout_ms = 60_000)]
async fn basic_text_response() {
	post_streamer::basic_text_response(streamer_non_streaming()).await;
}

#[ignore = "requires Ollama running locally"]
#[beet_core::test(timeout_ms = 60_000)]
async fn streaming_response() {
	post_streamer::streaming_response(streamer()).await;
}

#[ignore = "requires Ollama running locally"]
#[beet_core::test(timeout_ms = 60_000)]
async fn system_prompt() {
	post_streamer::system_prompt(streamer_non_streaming()).await;
}

#[ignore = "requires Ollama running locally"]
#[beet_core::test(timeout_ms = 60_000)]
async fn tool_calling() {
	post_streamer::tool_calling(streamer_non_streaming()).await;
}

#[ignore = "requires Ollama running locally"]
#[beet_core::test(timeout_ms = 60_000)]
async fn image_input() {
	post_streamer::image_input(streamer_non_streaming()).await;
}

#[ignore = "requires Ollama running locally"]
#[beet_core::test(timeout_ms = 60_000)]
async fn multi_turn_conversation() {
	post_streamer::multi_turn_conversation(streamer_non_streaming()).await;
}

// === PostStreamer (Completions) tests ===

#[ignore = "requires Ollama running locally"]
#[beet_core::test(timeout_ms = 60_000)]
async fn cs_basic_text_response() {
	post_streamer::basic_text_response(completions_streamer_non_streaming())
		.await;
}

#[ignore = "requires Ollama running locally"]
#[beet_core::test(timeout_ms = 60_000)]
async fn cs_streaming_response() {
	post_streamer::streaming_response(completions_streamer()).await;
}

#[ignore = "requires Ollama running locally"]
#[beet_core::test(timeout_ms = 60_000)]
async fn cs_system_prompt() {
	post_streamer::system_prompt(completions_streamer_non_streaming()).await;
}

#[ignore = "requires Ollama running locally"]
#[beet_core::test(timeout_ms = 60_000)]
async fn cs_tool_calling() {
	post_streamer::tool_calling(completions_streamer_non_streaming()).await;
}

#[ignore = "requires Ollama running locally"]
#[beet_core::test(timeout_ms = 60_000)]
async fn cs_image_input() {
	post_streamer::image_input(completions_streamer_non_streaming()).await;
}

#[ignore = "requires Ollama running locally"]
#[beet_core::test(timeout_ms = 60_000)]
async fn cs_multi_turn_conversation() {
	post_streamer::multi_turn_conversation(
		completions_streamer_non_streaming(),
	)
	.await;
}

// === Image Roundtrip tests ===

#[ignore = "requires Ollama running locally"]
#[beet_core::test(timeout_ms = 60_000)]
async fn image_roundtrip() {
	post_streamer::image_roundtrip(streamer_non_streaming()).await;
}

#[ignore = "requires Ollama running locally"]
#[beet_core::test(timeout_ms = 60_000)]
async fn cs_image_roundtrip() {
	post_streamer::image_roundtrip(completions_streamer_non_streaming()).await;
}
