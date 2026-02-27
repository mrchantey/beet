use crate::prelude::*;
use beet_core::prelude::*;

pub struct Renderer {
	pub tool: Tool<(RequestParts, Entity), Response>,
}
