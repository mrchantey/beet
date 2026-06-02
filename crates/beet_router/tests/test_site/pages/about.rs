use beet::prelude::*;

/// A dynamic page rendered per request.
pub async fn get(_cx: ActionContext<Request>) -> impl Scene {
	rsx! { <p>"About"</p> }
}
