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
				flow_exchange(|| {
					(InfallibleSequence, children![
						EndpointBuilder::new()
							.with_path("/*any?")
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
			ModelAction::new(OpenAiProvider::default()) // ModelAction::new(OllamaProvider::default())
		),
		(Name::new("Context to response"), context_to_response())
	])
}
