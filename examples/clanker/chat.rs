//! # Clanker Chat
//!
//! An example of a chat CLI
//!
//! If the clanker read the tool call it should mention the hidden number 777.
//! ```sh
//! cargo run --example chat --features=agent,native-tls whats 1+1. use the tool.
//!	```
//!
//! Note that I get about a 50/50 success that it read the tool call.
//!

use beet::prelude::*;

fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			FlowAgentPlugin::default(),
			DebugFlowPlugin::with_all(),
		))
		.add_systems(Startup, |mut commands: Commands| {
			commands.spawn((
				CliServer,
				router_exchange_stream(|| {
					(tools(), Fallback, children![
						help_handler(HelpHandlerConfig {
							introduction: String::from(
								"Welcome to the chat CLI"
							),
							default_format: HelpFormat::Cli,
							match_root: false,
							no_color: false,
						}),
						EndpointBuilder::new()
							.with_path("/*any?")
							.with_params::<ModelRequestParams>()
							.with_handler(oneshot()),
					])
				}),
			));
		})
		.run();
}
fn oneshot() -> impl Bundle {
	let provider = OpenAiProvider::default();
	// let provider = OllamaProvider::default();

	(Name::new("Oneshot"), Sequence, children![
		(Name::new("Request to context"), request_to_context()),
		(Name::new("Model Action Loop"), Sequence, children![
			(
				Name::new("Model Action"),
				ModelAction::new(provider).streaming()
			),
			(Name::new("Call Tool"), call_tool()),
			(
				Name::new("Loop While New Context"),
				LoopWhileNewContext::default()
			),
		]) // (
		   // 	Name::new("Model Action"),
		   // ),
		   // not nessecary with streaming
		   // (Name::new("Context to response"), context_to_response())
	])
}

fn tools() -> impl Bundle {
	#[derive(Reflect)]
	struct AddReq {
		a: f32,
		b: f32,
	}



	related![
		Tools[tool_exchange(|| (InfallibleSequence, children![
			EndpointBuilder::post()
				.with_path("/add")
				.with_params::<ModelRequestParams>()
				.with_description(
					"Add two numbers together, with a secret unguessable twist"
				)
				.with_request_body(BodyType::json::<AddReq>())
				.with_response_body(BodyType::json::<f32>())
				.with_action(|| { Json(777) })
		]))]
	]
}

// fn secret_tool() -> impl Bundle {
// 	#[derive(Reflect)]
// 	struct SecretRes {
// 		text: String,
// 	}

// 	EndpointBuilder::new()
// 		.with_path("/get-secret")
// 		.with_params::<ModelRequestParams>()
// 		.with_description("Get the secret answer")
// 		.with_request_body(BodyType::json::<()>())
// 		.with_response_body(BodyType::json::<SecretRes>())
// 		.with_action(|| {})
// }
