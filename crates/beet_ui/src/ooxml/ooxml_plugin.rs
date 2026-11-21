use super::deck_report::*;
use super::node::OoxmlRead;
use crate::prelude::*;
use beet_core::prelude::*;

/// The Office formats as media: their readers and writers, the
/// [`StylePlugin`] whose user agent rules every look reads and writes
/// through, what a deck's tree still carries so a scene holding one
/// serializes, and a deck's triage appended in [`PostParseTree`].
#[derive(Default)]
pub struct OoxmlPlugin;

impl Plugin for OoxmlPlugin {
	fn build(&self, app: &mut App) {
		app.install_type::<StylePlugin>()
			.register_type::<CoreProperties>()
			.register_type::<ListLevel>()
			.register_type::<OoxmlNode>()
			.register_type::<OoxmlRead>()
			.register_type::<Deck>()
			.register_type::<Slide>()
			.register_type::<SlidePicture>()
			.register_type::<DeckReported>()
			.install((
				OoxmlParser,
				WordParser::default(),
				WordRenderer,
				WorkbookParser::default(),
				WorkbookRenderer,
			))
			.init_schedule(PostParseTree)
			.add_systems(PostParseTree, append_deck_reports);
	}
}

/// Appends the triage of every deck read since the last run.
fn append_deck_reports(world: &mut World) { DeckReport::append_all(world); }
