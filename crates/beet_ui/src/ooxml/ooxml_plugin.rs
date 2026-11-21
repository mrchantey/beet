use crate::prelude::*;
use beet_core::prelude::*;

/// The Office formats as media: each format's reader and writer, and the
/// [`StylePlugin`] whose user agent rules every look reads and writes
/// through.
#[derive(Default)]
pub struct OoxmlPlugin;

impl Plugin for OoxmlPlugin {
	fn build(&self, app: &mut App) {
		app.install_type::<StylePlugin>().install((
			WordParser::default(),
			WordRenderer,
			WorkbookParser::default(),
			WorkbookRenderer,
			DeckParser::default(),
			DeckRenderer,
		));
	}
}
