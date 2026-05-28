use beet::prelude::*;

/// The static home page.
pub fn get() -> impl Bundle {
	rsx_direct!{ <h1>"Home"</h1> }
}
