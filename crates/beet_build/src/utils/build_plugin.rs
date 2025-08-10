use crate::prelude::*;
use beet_core::prelude::*;
use beet_parse::prelude::*;
use beet_utils::prelude::*;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
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

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, ScheduleLabel)]
pub struct BuildSequence;

impl Plugin for BuildPlugin {
	fn build(&self, app: &mut App) {
		bevy::ecs::error::GLOBAL_ERROR_HANDLER
			.set(bevy::ecs::error::panic)
			.ok();

		app.add_systems(
			Startup,
			(
				|mut commands: Commands| {
					// ensure at least one FileExprHash is present to trigger
					// listeners at least once
					commands.spawn((
						Name::new("empty FileExprHash"),
						FileExprHash::default(),
					));
				},
				// alternatively use import_route_file_collection to only load
				// source files used by file based routes
				load_workspace_source_files
					.run_if(BuildFlag::ImportSnippets.should_run()),
			),
		);

		app.add_event::<WatchEvent>()
			.init_plugin(ParseRsxTokensPlugin)
			// .init_plugin(ApplyDirectivesPlugin)
			.init_plugin(RouteCodegenPlugin)
			.init_plugin(NodeTypesPlugin)
			.insert_schedule_before(Update, BuildSequence)
			.insert_resource(CargoManifest::load().unwrap())
			.init_resource::<BuildFlags>()
			.init_resource::<CargoBuildCmd>()
			.init_resource::<WorkspaceConfig>()
			.init_resource::<LambdaConfig>()
			.init_resource::<ServerHandle>()
			.init_resource::<HtmlConstants>()
			.init_resource::<TemplateMacros>()
			.add_systems(
				ParseRsxTokens,
				import_file_inner_text.in_set(ModifyRsxTree),
			)
			.add_systems(
				BuildSequence,
				(
					parse_file_watch_events,
					reparent_route_collection_source_files,
					import_rsx_snippets_rs,
					import_rsx_snippets_md,
					ParseRsxTokens.run(),
					update_file_expr_hash,
					RouteCodegen.run(),
					// .run_if(BuildFlag::Codegen.should_run()),
					export_snippets
						.run_if(BuildFlag::ExportSnippets.should_run()),
					export_route_codegen
						.run_if(BuildFlag::Codegen.should_run()),
					compile_server
						.run_if(BuildFlag::CompileServer.should_run()),
					export_server_ssg.run_if(BuildFlag::ExportSsg.should_run()),
					compile_client
						.run_if(BuildFlag::CompileClient.should_run()),
					run_server.run_if(BuildFlag::RunServer.should_run()),
					deploy_sst.run_if(BuildFlag::DeploySst.should_run()),
					compile_lambda
						.run_if(BuildFlag::CompileLambda.should_run()),
					deploy_lambda.run_if(BuildFlag::DeployLambda.should_run()),
					sync_bucket.run_if(BuildFlag::SyncBucket.should_run()),
					lambda_log.run_if(BuildFlag::WatchLambda.should_run()),
				)
					.chain(),
			);
	}
}
