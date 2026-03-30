#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
use beet_core::prelude::*;
use beet_thread::prelude::*;

#[path = "utils/model_provider.rs"]
mod model_provider;
#[path = "utils/post_streamer.rs"]
mod post_streamer;

fn provider() -> impl ModelProvider {
	env_ext::load_dotenv();
	OpenAiProvider::default()
}

fn streamer() -> O11sStreamer {
	env_ext::load_dotenv();
	OpenAiProvider::gpt_5_mini().unwrap()
}

fn streamer_non_streaming() -> O11sStreamer {
	env_ext::load_dotenv();
	OpenAiProvider::gpt_5_mini().unwrap().without_streaming()
}

fn completions_streamer() -> CompletionsStreamer {
	env_ext::load_dotenv();
	OpenAiProvider::gpt_5_mini_completions().unwrap()
}

fn completions_streamer_non_streaming() -> CompletionsStreamer {
	env_ext::load_dotenv();
	OpenAiProvider::gpt_5_mini_completions()
		.unwrap()
		.without_streaming()
}

// === ModelProvider tests ===

#[beet_core::test(timeout_ms = 15_000)]
async fn basic_text_response() {
	model_provider::basic_text_response(provider()).await;
}

#[beet_core::test(timeout_ms = 15_000)]
async fn streaming_response() {
	model_provider::streaming_response(provider()).await;
}

#[beet_core::test(timeout_ms = 15_000)]
async fn system_prompt() { model_provider::system_prompt(provider()).await; }

#[beet_core::test(timeout_ms = 15_000)]
async fn tool_calling() { model_provider::tool_calling(provider()).await; }

#[beet_core::test(timeout_ms = 15_000)]
async fn image_input() { model_provider::image_input(provider()).await; }

#[beet_core::test(timeout_ms = 15_000)]
async fn multi_turn_conversation() {
	model_provider::multi_turn_conversation(provider()).await;
}

// === PostStreamer (O11s) tests ===

#[beet_core::test(timeout_ms = 15_000)]
async fn ps_basic_text_response() {
	post_streamer::basic_text_response(streamer_non_streaming()).await;
}

#[beet_core::test(timeout_ms = 15_000)]
async fn ps_streaming_response() {
	post_streamer::streaming_response(streamer()).await;
}

#[beet_core::test(timeout_ms = 15_000)]
async fn ps_system_prompt() {
	post_streamer::system_prompt(streamer_non_streaming()).await;
}

#[beet_core::test(timeout_ms = 15_000)]
async fn ps_tool_calling() {
	post_streamer::tool_calling(streamer_non_streaming()).await;
}

#[beet_core::test(timeout_ms = 15_000)]
async fn ps_image_input() {
	post_streamer::image_input(streamer_non_streaming()).await;
}

#[beet_core::test(timeout_ms = 15_000)]
async fn ps_multi_turn_conversation() {
	post_streamer::multi_turn_conversation(streamer_non_streaming()).await;
}

// === PostStreamer (Completions) tests ===

#[beet_core::test(timeout_ms = 15_000)]
async fn cs_basic_text_response() {
	post_streamer::basic_text_response(completions_streamer_non_streaming())
		.await;
}

#[beet_core::test(timeout_ms = 15_000)]
async fn cs_streaming_response() {
	post_streamer::streaming_response(completions_streamer()).await;
}

#[beet_core::test(timeout_ms = 15_000)]
async fn cs_system_prompt() {
	post_streamer::system_prompt(completions_streamer_non_streaming()).await;
}

#[beet_core::test(timeout_ms = 15_000)]
async fn cs_tool_calling() {
	post_streamer::tool_calling(completions_streamer_non_streaming()).await;
}

#[beet_core::test(timeout_ms = 15_000)]
async fn cs_image_input() {
	post_streamer::image_input(completions_streamer_non_streaming()).await;
}

#[beet_core::test(timeout_ms = 15_000)]
async fn cs_multi_turn_conversation() {
	post_streamer::multi_turn_conversation(
		completions_streamer_non_streaming(),
	)
	.await;
}

// === Image Roundtrip tests ===

#[beet_core::test(timeout_ms = 30_000)]
async fn ps_image_roundtrip() {
	post_streamer::image_roundtrip(
		streamer_non_streaming(),
		streamer_non_streaming(),
	)
	.await;
}

#[beet_core::test(timeout_ms = 30_000)]
async fn cs_image_roundtrip() {
	post_streamer::image_roundtrip(
		completions_streamer_non_streaming(),
		completions_streamer_non_streaming(),
	)
	.await;
}
