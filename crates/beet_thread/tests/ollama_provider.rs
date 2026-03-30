#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
use beet_thread::prelude::*;

#[path = "utils/post_streamer.rs"]
mod post_streamer;

fn streamer() -> O11sStreamer { OllamaProvider::qwen_3_8b() }

fn streamer_non_streaming() -> O11sStreamer {
	OllamaProvider::qwen_3_8b().without_streaming()
}

fn completions_streamer() -> CompletionsStreamer {
	OllamaProvider::qwen_3_8b_completions()
}

fn completions_streamer_non_streaming() -> CompletionsStreamer {
	OllamaProvider::qwen_3_8b_completions().without_streaming()
}

// === PostStreamer (O11s) tests ===

#[beet_core::test(timeout_ms = 60_000)]
async fn ps_basic_text_response() {
	post_streamer::basic_text_response(streamer_non_streaming()).await;
}

#[beet_core::test(timeout_ms = 60_000)]
async fn ps_streaming_response() {
	post_streamer::streaming_response(streamer()).await;
}

#[beet_core::test(timeout_ms = 60_000)]
async fn ps_system_prompt() {
	post_streamer::system_prompt(streamer_non_streaming()).await;
}

#[beet_core::test(timeout_ms = 60_000)]
async fn ps_tool_calling() {
	post_streamer::tool_calling(streamer_non_streaming()).await;
}

#[beet_core::test(timeout_ms = 60_000)]
async fn ps_image_input() {
	post_streamer::image_input(streamer_non_streaming()).await;
}

#[beet_core::test(timeout_ms = 60_000)]
async fn ps_multi_turn_conversation() {
	post_streamer::multi_turn_conversation(streamer_non_streaming()).await;
}

// === PostStreamer (Completions) tests ===

#[beet_core::test(timeout_ms = 60_000)]
async fn cs_basic_text_response() {
	post_streamer::basic_text_response(completions_streamer_non_streaming())
		.await;
}

#[beet_core::test(timeout_ms = 60_000)]
async fn cs_streaming_response() {
	post_streamer::streaming_response(completions_streamer()).await;
}

#[beet_core::test(timeout_ms = 60_000)]
async fn cs_system_prompt() {
	post_streamer::system_prompt(completions_streamer_non_streaming()).await;
}

#[beet_core::test(timeout_ms = 60_000)]
async fn cs_tool_calling() {
	post_streamer::tool_calling(completions_streamer_non_streaming()).await;
}

#[beet_core::test(timeout_ms = 60_000)]
async fn cs_image_input() {
	post_streamer::image_input(completions_streamer_non_streaming()).await;
}

#[beet_core::test(timeout_ms = 60_000)]
async fn cs_multi_turn_conversation() {
	post_streamer::multi_turn_conversation(
		completions_streamer_non_streaming(),
	)
	.await;
}

// === Image Roundtrip tests ===

#[beet_core::test(timeout_ms = 60_000)]
async fn ps_image_roundtrip() {
	post_streamer::image_roundtrip(streamer_non_streaming()).await;
}

#[beet_core::test(timeout_ms = 60_000)]
async fn cs_image_roundtrip() {
	post_streamer::image_roundtrip(completions_streamer_non_streaming()).await;
}
