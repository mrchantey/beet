use beet::prelude::*;

/// The static home page.
pub fn get() -> impl Scene {
	rsx! { <h1>"Home"</h1> }
}
