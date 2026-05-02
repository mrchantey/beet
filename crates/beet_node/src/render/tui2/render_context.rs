use crate::prelude::*;
use beet_core::prelude::*;
// use ratatui::buffer::Buffer;
// use ratatui::prelude::Rect;
// use ratatui::prelude::*;


// #[derive(Resource)]



#[derive(Get)]
pub struct TuiRenderContext<'a> {
	pub entity: EntityWorldMut<'a>,
	/// The full area of the terminal
	pub terminal_area: Rect,
	/// A subset of the terminal area, for the root
	/// this will be the same as the terminal area
	pub draw_area: Rect,
	pub buffer: &'a mut Buffer,
}


