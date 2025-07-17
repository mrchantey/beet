//! This example statically renders html content with a client:load directive,
//! which hydrates a static web page with a wasm module.
//!
//! Building is usually done by the cli but it can also be done manually:
//! ```sh
//! # export the html file
//! cargo run --example hydration
//! cargo build --example hydration --target-dir=target --features=template --target wasm32-unknown-unknown
//! wasm-bindgen --out-dir target/examples/hydration/wasm --out-name main --target web --no-typescript target/wasm32-unknown-unknown/debug/examples/hydration.wasm
//! ```
use beet::prelude::*;

#[rustfmt::skip]
#[cfg(target_arch = "wasm32")]
fn main() {
	App::new()
		.add_plugins(TemplatePlugin)
    .add_systems(Startup, |mut commands:Commands|{
			commands.spawn(app());
		})
		.set_runner(ReactiveApp::runner)
    .run();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
	let html = HtmlDocument::parse_bundle(app());
	FsExt::write("target/examples/hydration/index.html", html).unwrap();
}


fn app() -> impl Bundle {
	// the client:load directive adds a script tag for loading the wasm module
	rsx! {
		<Counter client:load initial=7/>
		<style>
			body{
				background: black;
				color: white;
			}
		</style>
	}
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
	}
}
