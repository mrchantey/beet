use beet::prelude::*;

/// A dynamic page rendered per request.
pub async fn get(_cx: ActionContext<Request>) -> impl Bundle {
	rsx_direct!{ <p>"About"</p> }
}
