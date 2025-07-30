//! Example of Client-Side Rendering (CSR) with Beet and Bevy.
//!
//! Note that this approach is not recommended because the entire
//! wasm app must be downloaded, parsed and run before the HTML is rendered,
//! resulting in a long time to first paint. See hydration.rs for a faster alternative.
//!
//! Here's an example of how to build with vanilla wasm-bindgen.
//! ```sh
//! cargo run --example csr
//! cargo build --example csr --target-dir=target --features=template --target wasm32-unknown-unknown
//! wasm-bindgen --out-dir target/examples/csr/wasm --out-name main --target web --no-typescript target/wasm32-unknown-unknown/debug/examples/csr.wasm
//! sweet serve target/examples/csr
//! ```
//!
use beet::prelude::*;

#[rustfmt::skip]
#[cfg(target_arch = "wasm32")]
fn main() {
	App::new()
		.add_plugins(ApplyDirectivesPlugin)
    .add_systems(Startup, |mut commands: Commands| {	
			// the client:only directive instructs the wasm build to render and mount the component in the browser
			commands.spawn(rsx! {<Counter client:only initial=7/>});
		})
		.set_runner(ReactiveApp::runner)
    .run();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
	let html = r#"
<!DOCTYPE html>
<html lang="en">
<body>
	<script type="module">
	import init from './wasm/main.js'
	init('./wasm/main_bg.wasm')
		.catch((error) => {
			if (!error.message.startsWith("Using exceptions for control flow,"))
				throw error
	})
	</script>
</body>
</html>
"#;
	FsExt::write("target/examples/csr/index.html", html).unwrap();
}

#[template]
// components with client directives must be serde
#[derive(Reflect)]
fn Counter(initial: u32) -> impl Bundle {
	let (get, set) = signal(initial);
	let (style, set_style) = signal("display: block;");

	rsx! {
			<p>Count: {get}</p>
			<button onclick={move || set(get()+1)}>Increment</button>
			<button
				style={style}
				onclick={move || set_style("display: none;")}>
				Hide Me
			</button>
	}
}
