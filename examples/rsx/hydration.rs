//! This example statically renders html content with a client:load directive,
//! which hydrates a static web page with a wasm module.
//!
//! Building is usually done by the cli but it can also be done manually:
//! ```sh
//! # export the html file
//! cargo run --example hydration --features=css
//! # build the wasm module
//! cargo build --example hydration --target-dir=target --target wasm32-unknown-unknown
//! wasm-bindgen --out-dir target/examples/hydration/wasm --out-name main --target web --no-typescript target/wasm32-unknown-unknown/debug/examples/hydration.wasm
//! # serve with your favorite web server
//! sweet serve target/examples/hydration
//! ```
//! For a sense of how hydration can make updates to static parts of the page,
//! try changing some of the text like "Count: " to "Num <b>Bananas</b>: "
//! then rebuild the html file `cargo run --example hydration --features=css`
//! Note how the wasm module still updates the correct locations in the page.
//! This works even without recompiling wasm, so long as the 'rusty parts' of the page
//! are still the same.
use beet::prelude::*;


// first run to generate the html file, which contains
// a serialized form of Counter used for hydration on the client.
#[cfg(not(target_arch = "wasm32"))]
fn main() {
	let html = HtmlDocument::parse_bundle(rsx! {
		<h1>Hello Hydration</h1>
		<Counter client:load initial=7/>
		<style scope:global>
			body{
				font-family: system-ui, sans-serif;
				background: black;
				color: white;
			}
		</style>
	});
	FsExt::write("target/examples/hydration/index.html", html).unwrap();
}

// then build the wasm module, this app must register the `ClientIslandRoot<Counter>`
// component so it can deserialize Counter.
#[cfg(target_arch = "wasm32")]
fn main() {
	App::new()
		.add_plugins(TemplatePlugin)
		// register the component in the scene to be loaded
		.register_type::<ClientIslandRoot<Counter>>()
		.set_runner(ReactiveApp::runner)
		.run();
}



// a simple counter,
// client islands must derive Reflect
#[template]
#[derive(Reflect)]
fn Counter(initial: u32) -> impl Bundle {
	let (get, set) = signal(initial);
	rsx! {
		<p>"Count: "{get}</p>
		<button
			onclick={move || set(get()+1)}
		>"Increment"</button>
		<style>
		button{
			background: #444;
			color: white;
			border: none;
			padding: 0.5.em 1.em;
			border-radius: 0.25.em;
			cursor: pointer;
		}
		</style>
	}
}
