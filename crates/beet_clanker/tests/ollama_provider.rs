#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]

use beet_clanker::prelude::*;

#[path = "utils/model_provider.rs"]
mod model_provider;

fn provider() -> impl ModelProvider {
	dotenv::dotenv().ok();
	OllamaProvider::default()
}

#[beet_core::test(timeout_ms = 60_000)]
async fn basic_text_response() {
	model_provider::basic_text_response(provider()).await;
}

#[beet_core::test(timeout_ms = 60_000)]
async fn streaming_response() {
	model_provider::streaming_response(provider()).await;
}

#[beet_core::test(timeout_ms = 60_000)]
async fn system_prompt() { model_provider::system_prompt(provider()).await; }

#[beet_core::test(timeout_ms = 60_000)]
async fn tool_calling() { model_provider::tool_calling(provider()).await; }

#[beet_core::test(timeout_ms = 60_000)]
async fn image_input() { model_provider::image_input(provider()).await; }

#[beet_core::test(timeout_ms = 60_000)]
async fn multi_turn_conversation() {
	model_provider::multi_turn_conversation(provider()).await;
}
