use beet::prelude::*;

/// A docs landing page using full ECS access.
pub fn get(_cx: In<ActionContext<Request>>) -> impl Bundle {
	rsx! { <h1>"Docs"</h1> }
}
