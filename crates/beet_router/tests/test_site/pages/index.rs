use beet::prelude::*;

/// The static home page.
pub fn get() -> impl Bundle {
	rsx! { <h1>"Home"</h1> }
}
