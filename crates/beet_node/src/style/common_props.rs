#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::{prelude::*, style::CssKeyMap};
use beet_core::prelude::Color;





token2!(BackgroundColor2, Color, DocumentPath::This);
token2!(ForegroundColor2, Color, DocumentPath::Ancestor);


pub fn css_key_map()->CssKeyMap{
	CssKeyMap::default()
		.with_property::<BackgroundColor2>("background-color")
		.with_property::<ForegroundColor2>("color")
}
