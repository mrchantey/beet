use beet::prelude::*;

fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			FlowAgentPlugin::default(),
			DebugFlowPlugin::default(),
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
		(
			Name::new("Model Action Loop"),
			RepeatWhileNewContext::default(),
			Sequence,
			children![
				(
					Name::new("Model Action"),
					ModelAction::new(provider).streaming()
				),
				(Name::new("Call Tool"), call_tool()),
			]
		) // (
		  // 	Name::new("Model Action"),
		  // ),
		  // not nessecary with streaming
		  // (Name::new("Context to response"), context_to_response())
	])
}

fn tools() -> impl Bundle {
	#[derive(Reflect)]
	struct AddReq {
		a: u32,
		b: u32,
	}
	#[derive(Reflect)]
	struct AddRes {
		a: u32,
	}



	related![
		Tools[tool_exchange(|| (InfallibleSequence, children![
			EndpointBuilder::new()
				.with_path("/add")
				.with_params::<ModelRequestParams>()
				.with_description("Add two numbers together")
				.with_request_body(BodyMeta::json::<AddReq>())
				.with_response_body(BodyMeta::json::<AddRes>())
				.with_action(|| {})
		]))]
	]
}
