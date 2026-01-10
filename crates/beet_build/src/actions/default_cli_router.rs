use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;



/// 🌱 Beet CLI 🌱
///
/// Welcome to the beet cli!
pub fn default_cli_router() -> impl Bundle {
	(
		Name::new("Cli Router"),
		CliServer,
		ExchangeSpawner::new_flow(|| {
			(
				// temporarily hardcode the beet site
				beet_site_cmd(),
				Fallback,
				children![
					EndpointBuilder::new(|tree: Res<EndpointTree>| {
						format!(
							"\n🌱 Welcome to the Beet CLI 🌱\n{}",
							tree.to_string()
						)
						// StatusCode::OK
					})
					.with_params::<HelpParams>()
					.with_path(""),
					EndpointBuilder::new(|| { StatusCode::IM_A_TEAPOT })
						.with_path("teapot"),
					EndpointBuilder::default()
						.with_path("run-wasm/*binary-path")
						.with_handler_bundle(run_wasm()),
					single_action_route("watch/*cmd?", watch()),
					single_action_route(
						"refresh-sst",
						SstCommand::new(SstSubcommand::Refresh)
					),
					single_action_route(
						"deploy-sst",
						SstCommand::new(SstSubcommand::Deploy)
					),
					single_action_route("build-wasm", build_wasm()),
					single_action_route("build-lambda", CompileLambda),
					single_action_route("deploy-lambda", DeployLambda),
					single_action_route("watch-lambda", WatchLambda),
					single_action_route("push-assets", PushAssets),
					single_action_route("pull-assets", PullAssets),
					single_action_route("push-html", PushHtml),
					single_action_route("build", BuildServer),
					single_action_route(
						"parse-files",
						import_and_parse_source_files()
					),
					named_route("parse-source-files", children![
						exact_path_match(),
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
						// respond_ok()
					]),
					named_route("run", children![
						exact_path_match(),
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
					]),
					named_route("serve", children![
						exact_path_match(),
						(Name::new("Serve"), Sequence, children![
							BuildServer,
							ExportStaticContent,
							RunServer,
						]),
					]),
					named_route("deploy", children![
						exact_path_match(),
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
						WatchLambda,
						respond_ok()
					])
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


fn named_route(name: impl AsRef<str>, children: impl Bundle) -> impl Bundle {
	let name = name.as_ref();
	(
		Name::new(name.to_string()),
		PathPartial::new(name),
		Sequence,
		children,
	)
}

fn single_action_route(
	name: impl AsRef<str>,
	action: impl Bundle,
) -> impl Bundle {
	named_route(name, children![exact_path_match(), action, respond_ok()])
}

fn beet_site_cmd() -> CargoBuildCmd {
	CargoBuildCmd::default().package("beet_site")
}

fn respond_ok() -> impl Bundle {
	(
		Name::new("Response"),
		StatusCode::OK.into_endpoint_handler(),
	)
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
