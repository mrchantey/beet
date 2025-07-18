//! This example statically renders html content with a client:load directive,
//! which hydrates a static web page with a wasm module.
//!
//! Building is usually done by the cli but it can also be done manually:
//! ```sh
//! # export the html file
//! cargo run --example hydration
//! # build the wasm module
//! cargo build --example hydration --target-dir=target --target wasm32-unknown-unknown
//! wasm-bindgen --out-dir target/examples/hydration/wasm --out-name main --target web --no-typescript target/wasm32-unknown-unknown/debug/examples/hydration.wasm
//! # serve with your favorite web server
//! sweet serve target/examples/hydration
//! ```
use beet::prelude::*;

#[cfg(target_arch = "wasm32")]
fn main() {
	App::new()
		.add_plugins(TemplatePlugin)
		// register the component in the scene to be loaded
		.register_type::<ClientIslandRoot<Counter>>()
		.set_runner(ReactiveApp::runner)
		.run();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
	let html = HtmlDocument::parse_bundle(rsx! {
		<h1>Hello Hydration</h1>
		<Counter client:load initial=7/>
		<style>
			body{
				background: black;
				color: white;
			}
		</style>
	});
	FsExt::write("target/examples/hydration/index.html", html).unwrap();
}



#[template]
#[derive(Reflect)]
fn Counter(initial: u32) -> impl Bundle {
	let (get, set) = signal(initial);
	rsx! {
		<p>"Count awesome: "{get}</p>
		<button
			onclick={move || set(get()+1)}
		>"Increment"</button>
	}
}
