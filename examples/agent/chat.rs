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
				router_exchange(|| {
					(Fallback, children![
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
	(Name::new("Oneshot"), Sequence, children![
		(Name::new("Request to context"), request_to_context()),
		(
			Name::new("Model Action"),
			// ModelAction::new(OpenAiProvider::default()),
			ModelAction::new(OllamaProvider::default()).streaming()
		),
		(Name::new("Context to response"), context_to_response())
	])
}
