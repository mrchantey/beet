//! Example of Client-Side Rendering (CSR) with Beet and Bevy.
//!
//! Note that this approach is not recommended because the entire wasm
//! app must be built and run before the HTML is rendered, resulting in a long time
//! to first paint.
//!
//! That said, here's an example of how to build with vanilla wasm-bindgen.
//! ```sh
//! cargo build --example csr --target-dir=target --features=template --target wasm32-unknown-unknown
//! wasm-bindgen --out-dir target/csr-demo/wasm --out-name main --target web --no-typescript target/wasm32-unknown-unknown/debug/examples/csr.wasm
//! cp examples/dom/csr.html target/csr-demo/index.html
//! cd target/csr-demo && npx live-server
//! ```
//!
use beet::prelude::*;
use bevy::prelude::*;
use sweet::prelude::*;

#[rustfmt::skip]
fn main() {
	App::new()
		.add_plugins(TemplatePlugin)
    .add_systems(Startup, setup)
    .run();
}

fn setup(mut commands: Commands) {
	// the client:only directive instructs the wasm build to render and mount the component in the browser
	commands.spawn(rsx! {
		<Counter client:only initial=7/>
	});
}

#[template]
// components with client directives must be serde
#[derive(serde::Serialize)]
fn Counter(initial: u32) -> impl Bundle {
	let (get, set) = signal(initial);

	rsx! {
			<p>Count: {get}</p>
			<button onclick={move || {
				set(get()+1);
				sweet::log!("clicked! {}", get());
			}}>Increment</button>
	}
}
