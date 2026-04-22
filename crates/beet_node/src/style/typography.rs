use beet_core::prelude::*;

use super::*;
use crate::prelude::*;

token!(Unit, FONT_SIZE, "font-size");


pub struct Typeface(SmolStr);

pub struct Typography {
	pub size: Unit,
	pub weight: FontWeight,
	pub line_height: Unit,
	pub letter_spacing: Unit,
}


pub enum FontWeight {
	Absolute(u16),
	Normal,
	Bold,
	Lighter,
	Bolder,
}
