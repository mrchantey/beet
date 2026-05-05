// use crate::render::*;
use beet_core::prelude::*;


pub struct TestBackend {
	size: UVec2,
	buffer: Buffer,
	cursor_position: UVec2,
	cursor_hidden: bool,
}
