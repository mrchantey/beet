#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet::prelude::*;

fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			CliPlugin,
			LogPlugin {
				level: Level::WARN,
				..default()
			},
			// DebugFlowPlugin::default(),
		))
		.spawn_then(sweet_router())
		.run()
		.into_exit_native();
}

fn sweet_router() -> impl Bundle {
	(
		Name::new("Sweet Router"),
		CliServer,
		ExchangeSpawner::new_flow(|| {
			(Fallback, children![
				help_handler(HelpFormat::Cli),
				EndpointBuilder::get()
					.with_params::<HelpParams>()
					.with_description("🤘 Welcome to the Sweet CLI 🤘\n{}")
					.with_path(""),
				EndpointBuilder::default()
					// match trailing positionals too, they will be
					// passed to the wasm runtime
					.with_path("run-wasm/*binary-path")
					.with_handler_bundle((
						Name::new("Run Wasm"),
						InsertOn::<GetOutcome, _>::new(
							FsWatcher::default_cargo()
						),
						RunOnDirEvent,
						Fallback,
						children![
							run_wasm(),
							StatusCode::BAD_REQUEST.into_endpoint_handler(),
						]
					)),
			])
		}),
	)
}
