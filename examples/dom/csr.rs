//! Example of Client-Side Rendering (CSR) with Beet and Bevy.
//!
//! Usually the beet cli takes care of building, but beet can also be used as a library.
//! Here's an example of how to build with vanilla wasm-bindgen.
//! ```sh
//! cargo build --example csr --target-dir=target --features=template --target wasm32-unknown-unknown
//! wasm-bindgen --out-dir target/csr-demo/wasm --out-name main --target web --no-typescript target/wasm32-unknown-unknown/debug/examples/csr.wasm
//! cp examples/dom/csr.html target/csr-demo/index.html
//! cd target/csr-demo && npx live-server
//! ```
//!
use beet::prelude::*;
use bevy::prelude::*;


#[rustfmt::skip]
fn main() {
	App::new()
		.add_plugins(TemplatePlugin)
    .add_systems(Startup, setup)
    .run();
}

fn setup(mut commands: Commands) {
	commands.spawn(rsx! {
		<Counter initial=7/>
	});
}

#[template]
#[derive(serde::Serialize)]
fn Counter(initial: u32) -> impl Bundle {
	let (get, set) = signal(initial);

	rsx! {
			<p>Count: {get}</p>
			<button onclick={move ||set(get()+1)}>Increment</button>
	}
}
