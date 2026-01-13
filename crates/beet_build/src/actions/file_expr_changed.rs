use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;


/// Triggers an [`Outcome::Pass`] if a [`FileExprHash`] was changed,
/// and it passes the filter if provided.
#[action(run_on_file_expr)]
#[derive(Debug, Default, Component)]
#[require(Name = Name::new("FileExprChanged Check"))]
pub struct FileExprChanged {
	filter: Option<GlobFilter>,
}

impl FileExprChanged {
	/// Create a new [`FileExprChanged`] action with no filter
	pub fn new() -> Self { Self { filter: None } }

	/// Create a new [`FileExprChanged`] action with a glob filter
	pub fn with_filter(filter: GlobFilter) -> Self {
		Self {
			filter: Some(filter),
		}
	}
}

fn run_on_file_expr(
	ev: On<GetOutcome>,
	action_query: Query<&FileExprChanged>,
	changed_query: Query<&SourceFile, Changed<FileExprHash>>,
	mut commands: Commands,
) -> Result {
	let outcome = if changed_query.is_empty() {
		// no changed files
		Outcome::Fail
	} else if let Some(filter) = &action_query.get(ev.target())?.filter {
		if changed_query
			.iter()
			.any(|source_file| filter.passes(source_file))
		{
			// at least one changed file passes the filter
			Outcome::Pass
		} else {
			// files changed but none matching the filter
			Outcome::Fail
		}
	} else {
		// at least one changed file
		Outcome::Pass
	};

	commands.entity(ev.target()).trigger_target(outcome);

	Ok(())
}


#[cfg(test)]
mod tests {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;

	#[sweet::test]
	async fn no_files() {
		let mut app = App::new();
		app.add_plugins(CliPlugin)
			.world_mut()
			.spawn((FileExprChanged::default(), ExitOnEnd))
			.trigger_target(GetOutcome);
		// no changed files
		app.run_async().await.xpect_eq(AppExit::error());
	}
	#[sweet::test]
	async fn one_file() {
		let mut app = App::new();
		app.world_mut()
			.spawn(SourceFile::new(WsPathBuf::new(file!()).into_abs()));
		app.add_plugins(CliPlugin)
			.world_mut()
			.spawn((FileExprChanged::default(), ExitOnEnd))
			.trigger_target(GetOutcome);
		// one changed file
		app.run_async().await.xpect_eq(AppExit::Success);
	}
	#[sweet::test]
	async fn one_file_filtered_out() {
		let mut app = App::new();
		app.world_mut()
			.spawn(SourceFile::new(WsPathBuf::new(file!()).into_abs()));
		app.add_plugins(CliPlugin)
			.world_mut()
			.spawn((
				FileExprChanged::with_filter(
					GlobFilter::default().with_exclude("*beet_build*"),
				),
				ExitOnEnd,
			))
			.trigger_target(GetOutcome);
		// one changed file
		app.run_async().await.xpect_eq(AppExit::error());
	}
}
