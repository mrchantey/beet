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
		CliRouter,
		InfallibleSequence,
		beet_site_cmd(),
		children![
			(single_action_route(
				"refresh-sst",
				SstCommand::new(SstSubcommand::Refresh)
			)),
			(single_action_route(
				"deploy-sst",
				SstCommand::new(SstSubcommand::Deploy)
			)),
			(single_action_route("compile-wasm", build_wasm())),
			(single_action_route("compile-lambda", CompileLambda)),
			(single_action_route("deploy-lambda", DeployLambda)),
			(single_action_route("watch-lambda", WatchLambda)),
			(single_action_route("push-assets", PushAssets)),
			(single_action_route("pull-assets", PullAssets)),
			(single_action_route("push-html", PushHtml)),
			(single_action_route("build", BuildServer)),
			(single_action_route(
				"parse-files",
				import_and_parse_source_files()
			)),
			(named_route("parse-source-files", children![
				exact_route_match(),
				import_source_files(),
				(
					Name::new("Run Loop"),
					// only insert the watcher after first run
					InsertOn::<GetOutcome, _>::new(FsWatcher::default_cargo()),
					RunOnDirEvent,
					InfallibleSequence,
					children![
						ParseSourceFiles::action(),
						(Name::new("Full Rebuild Check"), Sequence, children![
							FileExprChanged::new(),
							(
								Name::new("Pretend Rebuild.."),
								EndWith(Outcome::Pass)
							)
						]),
						(
							// never return to emulate server
							Name::new("Pretend Serve..")
						),
					]
				),
				// respond_ok()
			])),
			(named_route("run", children![
				exact_route_match(),
				import_source_files(),
				(
					Name::new("Run Loop"),
					// only insert the watcher after first run
					InsertOn::<GetOutcome, _>::new(FsWatcher::default_cargo()),
					RunOnDirEvent,
					Sequence,
					children![
						ParseSourceFiles::action(),
						// build_wasm(),
						BuildServer,
						ExportStaticContent,
						// never returns an outcome
						RunServer,
					]
				),
			])),
			(named_route("serve", children![
				exact_route_match(),
				(Name::new("Serve"), Sequence, children![
					BuildServer,
					RunServer,
				]),
			])),
			(named_route("deploy", children![
				exact_route_match(),
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
			]))
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
			|mut ev: On<GetOutcome>,
			 mut pkg_config: ResMut<PackageConfig>,
			 mut cmd: AncestorQuery<&'static mut CargoBuildCmd>|
			 -> Result {
				pkg_config.service_access = ServiceAccess::Remote;
				cmd.get_mut(ev.action())?.release = true;
				ev.trigger_with_cx(Outcome::Pass);
				Ok(())
			},
		),
	)
}


fn named_route(name: impl AsRef<str>, children: impl Bundle) -> impl Bundle {
	let name = name.as_ref();
	(
		Name::new(name.to_string()),
		RoutePartial::new(name),
		Sequence,
		children,
	)
}

fn single_action_route(
	name: impl AsRef<str>,
	action: impl Bundle,
) -> impl Bundle {
	named_route(name, children![exact_route_match(), action, respond_ok()])
}

fn beet_site_cmd() -> CargoBuildCmd {
	CargoBuildCmd::default().package("beet_site")
}

fn respond_ok() -> impl Bundle {
	(Name::new("Response"), StatusCode::OK.into_endpoint())
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
