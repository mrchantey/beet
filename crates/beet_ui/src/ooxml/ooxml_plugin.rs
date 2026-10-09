use super::deck_report::*;
use crate::prelude::*;
use beet_core::prelude::*;

/// Registers what an Office tree carries, so a scene holding one serializes,
/// the [`OoxmlRenderer`] that writes one back, and appends a deck's triage in
/// [`PostParseTree`].
#[derive(Default)]
pub struct OoxmlPlugin;

impl Plugin for OoxmlPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<ParagraphStyle>()
			.register_type::<ListLevel>()
			.register_type::<RunLook>()
			.register_type::<OoxmlNode>()
			.register_type::<SheetCellAddress>()
			.register_type::<CellLocked>()
			.register_type::<CellFormula>()
			.register_type::<CoveredBy>()
			.register_type::<SheetCellStyle>()
			.register_type::<Deck>()
			.register_type::<Slide>()
			.register_type::<SlidePicture>()
			.register_type::<DeckReported>()
			.register_render_target(OoxmlRenderer)
			.init_schedule(PostParseTree)
			.add_systems(PostParseTree, append_deck_reports);
	}
}

/// Appends the triage of every deck read since the last run.
fn append_deck_reports(world: &mut World) { DeckReport::append_all(world); }
