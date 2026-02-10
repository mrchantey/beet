use crate::prelude::*;
use beet_core::prelude::*;

/// An interface for rendering content and tool options
/// directly to stdout, and getting calls from stdin.
pub struct StdIo;


pub fn stdio_server() -> impl Bundle {
	(
		Name::new("stdio server"),
		Interface::new_this(),
		direct_tool(handler),
	)
}

async fn handler(request: Request) -> Response { request.mirror() }
