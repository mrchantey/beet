use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;





/// 🌱 Beet CLI 🌱
///
/// Welcome to the beet cli!
pub fn beet_cli() -> impl Bundle {
	(
		Name::new("Cli Router"),
		CliServer,
		ExchangeSpawner::new_flow(|| {
			(
				// temporarily hardcode the beet site as a component
				beet_site_cargo_build_cmd(),
				Fallback,
				children![
					EndpointBuilder::get()
						.with_path("")
						.with_description("🌱 Beet CLI - Use --help to see available commands")
						.with_handler(|| {
							"\n🌱 Welcome to the Beet CLI 🌱\n\nUse --help to see available commands."
						}),
					EndpointBuilder::new(|| { StatusCode::IM_A_TEAPOT })
						.with_path("teapot")
						.with_description("I'm a teapot"),
					EndpointBuilder::default()
						.with_path("run-wasm/*binary-path")
						.with_description("Run a wasm binary")
						.with_handler_bundle(run_wasm()),
					EndpointBuilder::get()
						.with_path("watch/*cmd?")
						.with_description("Watch for file changes and run command")
						.with_handler_bundle(watch()),
					EndpointBuilder::get()
						.with_path("refresh-sst")
						.with_description("Refresh SST configuration")
						.with_handler_bundle(SstCommand::new(SstSubcommand::Refresh)),
					EndpointBuilder::get()
						.with_path("deploy-sst")
						.with_description("Deploy using SST")
						.with_handler_bundle(SstCommand::new(SstSubcommand::Deploy)),
					EndpointBuilder::get()
						.with_path("build-wasm")
						.with_description("Build wasm target")
						.with_handler_bundle(build_wasm()),
					EndpointBuilder::get()
						.with_path("build-lambda")
						.with_description("Compile lambda function")
						.with_handler_bundle(CompileLambda),
					EndpointBuilder::get()
						.with_path("deploy-lambda")
						.with_description("Deploy lambda function")
						.with_handler_bundle(DeployLambda),
					EndpointBuilder::get()
						.with_path("watch-lambda")
						.with_description("Watch lambda logs")
						.with_handler_bundle(WatchLambda),
					EndpointBuilder::get()
						.with_path("push-assets")
						.with_description("Push assets to remote")
						.with_handler_bundle(PushAssets),
					EndpointBuilder::get()
						.with_path("pull-assets")
						.with_description("Pull assets from remote")
						.with_handler_bundle(PullAssets),
					EndpointBuilder::get()
						.with_path("push-html")
						.with_description("Push HTML to remote")
						.with_handler_bundle(PushHtml),
					EndpointBuilder::get()
						.with_path("build")
						.with_description("Build server")
						.with_handler_bundle(BuildServer),
					EndpointBuilder::get()
						.with_path("parse-files")
						.with_description("Import and parse source files")
						.with_handler_bundle(import_and_parse_source_files()),
					EndpointBuilder::get()
						.with_path("parse-source-files")
						.with_description("Parse source files with file watching")
						.with_handler_bundle((Sequence, children![
							import_source_files(),
							(
								Name::new("Run Loop"),
								// only insert the watcher after first run
								InsertOn::<GetOutcome, _>::new(
									FsWatcher::default_cargo()
								),
								RunOnDirEvent,
								InfallibleSequence,
								children![
									ParseSourceFiles::action(),
									(
										Name::new("Full Rebuild Check"),
										Sequence,
										children![
											FileExprChanged::new(),
											(
												Name::new("Pretend Rebuild.."),
												EndWith(Outcome::Pass)
											)
										]
									),
									(
										// never return to emulate server
										Name::new("Pretend Serve..")
									),
								]
							),
						])),
					EndpointBuilder::get()
						.with_path("run")
						.with_description("Run the server with file watching")
						.with_handler_bundle((Sequence, children![
							import_source_files(),
							(
								Name::new("Run Loop"),
								// only insert the watcher after first run
								InsertOn::<GetOutcome, _>::new(
									FsWatcher::default_cargo()
								),
								RunOnDirEvent,
								InfallibleSequence,
								children![
									ParseSourceFiles::action(),
									(
										Name::new("Build Check"),
										Sequence,
										children![
											FileExprChanged::new(),
											build_wasm(),
											BuildServer,
										]
									),
									ExportStaticContent,
									// never returns an outcome
									RunServer,
									// bevyhow!("unreachable! server shouldnt exit")
								]
							),
						])),
					EndpointBuilder::get()
						.with_path("serve")
						.with_description("Build and serve the application")
						.with_handler_bundle((
							Name::new("Serve"),
							Sequence,
							children![
								BuildServer,
								ExportStaticContent,
								RunServer,
							]
						)),
					EndpointBuilder::get()
						.with_path("deploy")
						.with_description("Full deployment pipeline")
						.with_handler_bundle((Sequence, children![
							import_and_parse_source_files(),
							// apply after import to avoid clobber,
							// the scene loaded likely contains a PackageConfig
							apply_deploy_config(),
							build_wasm(),
							BuildServer,
							ExportStaticContent,
							CompileLambda,
							// push assets directly before deploying
							// the lambda function to minimize
							// server version mismatch
							PushAssets,
							PushHtml,
							DeployLambda,
							WatchLambda
						]))
				],
			)
		}),
	)
}

fn watch() -> impl Bundle {
	(
		Name::new("Watch"),
		// only insert the watcher after first run
		InsertOn::<GetOutcome, _>::new(FsWatcher::default_cargo()),
		RunOnDirEvent,
		InfallibleSequence,
		OnSpawn::observe(|ev: On<DirEvent>| {
			println!("Dir event: {}", ev.event());
		}),
		children![
			(
				Name::new("Run command"),
				OnSpawn::observe(
					|ev: On<GetOutcome>,
					 mut cmd_runner: CommandRunner|
					 -> Result {
						let args = env_ext::args();
						// skip the 'watch' command
						let args = &args[1..].join(" ");
						let config = CommandConfig::parse_shell(args);
						cmd_runner.run(ev, config)?;
						Ok(())
					}
				)
			),
			(
				Name::new("Await file change"),
				// never returns
			),
		],
	)
}

/// Apply non-optional settings for a deployed environment:
/// - [`PackageConfig::service_access`] = [`ServiceAccess::Remote`]
/// - [`CargoBuildCmd::release`] = `true`
///   - Note: this setting is indepentent of [`PackageConfig::stage`]
fn apply_deploy_config() -> impl Bundle {
	(
		Name::new("Apply Deploy Config"),
		OnSpawn::observe(
			|ev: On<GetOutcome>,
			 mut pkg_config: ResMut<PackageConfig>,
			 mut cmd: AncestorQuery<&'static mut CargoBuildCmd>,
			 mut commands: Commands|
			 -> Result {
				pkg_config.service_access = ServiceAccess::Remote;
				pkg_config.stage = "prod".to_string();
				cmd.get_mut(ev.target())?.release = true;
				commands.entity(ev.target()).trigger_target(Outcome::Pass);
				Ok(())
			},
		),
	)
}




fn beet_site_cargo_build_cmd() -> CargoBuildCmd {
	CargoBuildCmd::default().package("beet_site")
}




// fn new_from_template() -> impl Bundle {
// // TODO lock down to commit matching the cli release
// let mut command = Command::new("cargo");
// command
// 	.arg("generate")
// 	.arg("--git")
// 	.arg("https://github.com/mrchantey/beet")
// 	.arg("crates/beet_new_web")
// 	.args(&self.additional_args);
//
//
//
//
// }
