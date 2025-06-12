//! This example statically renders html content with a client:load directive.
//! This instructs the browser to load the wasm, hooking up events and
//! effects, but not re-rendering the page.
//!
//! To build, first export the html file, then build the wasm module.
//! ```sh
//! cargo run --example hydration
//! cargo build --example hydration --target-dir=target --features=template --target wasm32-unknown-unknown
//! wasm-bindgen --out-dir target/examples/hydration/wasm --out-name main --target web --no-typescript target/wasm32-unknown-unknown/debug/examples/hydration.wasm
//! ```
use beet::prelude::*;
use bevy::prelude::*;
use sweet::prelude::*;

#[rustfmt::skip]
#[cfg(target_arch = "wasm32")]
fn main() {
	App::new()
		.add_plugins(TemplatePlugin)
    .add_systems(Startup, |mut commands:Commands|commands.spawn(app()))
    .run();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
	let html = HtmlDocument::parse_bundle(app());
	FsExt::write("target/examples/hydration/index.html", html).unwrap();
}


fn app() -> impl Bundle {
	// the client:load directive results in a script tag being added which will
	// load the wasm module
	rsx! {<Counter client:load initial=7/>}
}

#[template]
#[derive(serde::Serialize)]
fn Counter(initial: u32) -> impl Bundle {
	let (get, set) = signal(initial);
	rsx! {
			<p>Count: {get}</p>
			<button onclick={move || set(get()+1)}>Increment</button>
	}
}
