//! Example usage of dom rsx
//! Run in native mode to generate the static html file,
//! then build with wasm target for the interactive binary
//!
//! for live template reloading command see justfile run-dom-rsx
//!
use beet::prelude::*;
use beet::rsx::sigfault::signal;

#[rustfmt::skip]
fn main() { 
	BeetApp::new()
		.add_collection(app)
		.run(); 
}

fn app() -> RsxRoot {
	let (value, set_value) = signal(0);

	rsx! {
		<div>
			<div>"The value is "{value.clone()}</div>
			<button onclick={move |_| set_value(value() + 2)}>
				"increment the number"
			</button>
		</div>
	}
}
