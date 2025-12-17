use crate::prelude::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;
use beet_parse::prelude::*;
// use beet_rsx::prelude::ApplyDirectives;
use bevy::ecs::schedule::ScheduleLabel;
use cargo_manifest::Manifest;


#[derive(Resource, Deref)]
pub struct CargoManifest(Manifest);

impl CargoManifest {
	pub fn load() -> Result<CargoManifest> {
		Self::load_from_path(&AbsPathBuf::new_workspace_rel("Cargo.toml")?)
	}

	pub fn load_from_path(path: &AbsPathBuf) -> Result<CargoManifest> {
		Manifest::from_path(&path)
			.map_err(|e| {
				bevyhow!(
					"Failed to load Cargo manifest\nPath:{}\nError:{}",
					path,
					e
				)
			})
			.map(|manifest| CargoManifest(manifest))
	}
	pub fn package_name(&self) -> Option<&str> {
		self.0.package.as_ref().map(|p| p.name.as_str())
	}
}

/// Main plugin for beet_build
#[derive(Debug, Default, Clone)]
pub struct BuildPlugin;


/// A schedule to load source files in a workspace,
/// convert to an ECS representation using [`ParseRsxTokens`],
/// and then export the rsx snippets and codegen.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, ScheduleLabel)]
pub struct ParseSourceFiles;

impl Plugin for BuildPlugin {
	fn build(&self, app: &mut App) {
		app.try_set_error_handler(bevy::ecs::error::panic);
		app.add_message::<WatchEvent>()
			.init_plugin::<ParseRsxTokensPlugin>()
			.init_plugin::<RouteCodegenPlugin>()
			.init_plugin::<NodeTypesPlugin>()
			.insert_resource(CargoManifest::load().unwrap())
			.init_resource::<BuildFlags>()
			.init_resource::<CargoBuildCmd>()
			.init_resource::<WorkspaceConfig>()
			.init_resource::<LambdaConfig>()
			.init_resource::<HtmlConstants>()
			.init_resource::<TemplateMacros>()
			.add_systems(
				ParseRsxTokens,
				import_file_inner_text.in_set(ModifyRsxTree),
			)
			.add_systems(
				ParseSourceFiles,
				(
					parse_file_watch_events,
					reparent_route_collection_source_files,
					import_rsx_snippets_rs,
					import_rsx_snippets_md,
					ParseRsxTokens.run(),
					update_file_expr_hash,
					RouteCodegen.run(),
					// ApplyDirectives.run(),
					export_snippets,
					export_codegen,
					// compile_server
					// 	.run_if(BuildFlag::CompileServer.should_run()),
					// export_server_ssg.run_if(BuildFlag::ExportSsg.should_run()),
					// compile_client
					// 	.run_if(BuildFlag::CompileClient.should_run()),
					// run_server.run_if(BuildFlag::RunServer.should_run()),
					// (
					// 	refresh_sst.run_if(BuildFlag::RefreshSst.should_run()),
					// 	deploy_sst.run_if(BuildFlag::DeploySst.should_run()),
					// )
					// 	.chain(),
					// compile_lambda
					// 	.run_if(BuildFlag::CompileLambda.should_run()),
					// deploy_lambda.run_if(BuildFlag::DeployLambda.should_run()),
					// (
					// 	// push_html.run_if(BuildFlag::PushHtml.should_run()),
					// 	// push_assets.run_if(BuildFlag::PushAssets.should_run()),
					// 	// pull_assets.run_if(BuildFlag::PullAssets.should_run()),
					// )
					// 	.chain(),
					// lambda_log.run_if(BuildFlag::WatchLambda.should_run()),
				)
					.chain(),
			);
	}
}
