use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;

#[derive(Default)]
pub struct StylePlugin;

impl Plugin for StylePlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<TokenPlugin>()
			.init_resource::<CssTokenMap>()
			.init_schedule(PostParseTree)
			.add_systems(PostUpdate, resolve_styles.in_set(ResolveStylesSet));

		#[cfg(feature = "syntax_highlighting")]
		app.init_resource::<SyntaxHighlighting>().add_systems(
			PostParseTree,
			(apply_syntax_highlighting, resolve_styles).chain(),
		);
		#[cfg(not(feature = "syntax_highlighting"))]
		app.add_systems(PostParseTree, resolve_styles);
	}
}


/// A set configured in PostUpdate that applies styles to
/// realtime applications like a tui or gui
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ResolveStylesSet;


/// Ran in non-realtime environments like http servers with
/// html and markdown parsers, that need to run the schedule as a one-off
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct PostParseTree;
