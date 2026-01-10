use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_rsx::prelude::*;


/// Create a [`SourceFile`] for each file specified in the [`WorkspaceConfig`].
/// This will run once for the initial load, afterwards [`parse_file_watch_events`]
/// will incrementally add, remove and mark changed as needed.
///
/// These files are initially loaded as children of the [`SourceFileRoot`],
/// but may be moved to a [`RouteFileCollection`] if the path matches.
//  we could alternatively use import_route_file_collection to only load
// source files used by file based routes, but other files are currently watched
// for live reloading
#[construct]
pub fn AddWorkspaceSourceFiles() -> impl Bundle {
	(
		Name::new("Add Workspace Source Files"),
		OnSpawn::observe(
			|ev: On<GetOutcome>,
			 config: Res<WorkspaceConfig>,
			 mut commands: Commands|
			 -> Result {
				commands.spawn((
					NonCollectionSourceFiles,
					Children::spawn(SpawnIter(
						config
							.get_files()?
							.into_iter()
							.map(|path| SourceFile::new(path)),
					)),
				));
				commands.entity(ev.target()).trigger_target(Outcome::Pass);
				Ok(())
			},
		),
	)
}



pub fn import_source_files() -> impl Bundle {
	(Name::new("Import Source Files"), Sequence, children![
		launch_sequence(),
		AddWorkspaceSourceFiles,
	])
}

pub fn import_and_parse_source_files() -> impl Bundle {
	(
		Name::new("Import and Parse Source Files"),
		Sequence,
		children![import_source_files(), ParseSourceFiles::action()],
	)
}

// #[construct]
// pub fn
/// ensure at least one FileExprHash is present to trigger
/// listeners at least once
#[deprecated = "unneeded cos we use beet flow now?"]
pub fn init_file_expr_hash(mut commands: Commands) {
	commands.spawn((Name::new("Empty FileExprHash"), FileExprHash::default()));
}
