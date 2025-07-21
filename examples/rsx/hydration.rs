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
//! try changing some of the text like "Count" then running `cargo run --example hydration --features=css` again
//! Note how the wasm module still updates the correct locations in the page, even without recompiling wasm.
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
