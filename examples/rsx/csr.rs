//! Example of Client-Side Rendering (CSR) with Beet and Bevy.
//!
//! Note that this approach is not recommended because the entire
//! wasm app must be downloaded, parsed and run before the HTML is rendered,
//! resulting in a long time-to-first-paint. See hydration.rs for a more performant alternative.
//!
//! Here's an example of how to build with vanilla wasm-bindgen.
//! ```sh
//! cargo run --example csr --features=client
//! cargo build --example csr --features=client --target wasm32-unknown-unknown
//! wasm-bindgen --out-dir target/examples/csr/wasm --out-name main --target web --no-typescript $CARGO_TARGET_DIR/wasm32-unknown-unknown/debug/examples/csr.wasm
//! sweet serve target/examples/csr
//! ```
//!
use beet::prelude::*;

#[rustfmt::skip]
#[cfg(target_arch = "wasm32")]
fn main() {
	App::new()
    .insert_resource(pkg_config!())
		.add_plugins(BeetPlugins)
    .add_systems(Startup, |mut commands: Commands| {
			// the client:only directive instructs the wasm build to render and mount the component in the browser
			commands.spawn(rsx! {<MyApp client:only/>});
		})
    .run();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
	// usually this would be built using templating like rsx!{}
	// but we're hardcoding here to clearly demonstrate the ''
	let html = r#"
<!DOCTYPE html>
<html lang="en">
<head>
</head>
<body>
	<script type="module">
	import init from './wasm/main.js'
	init({module_or_path: './wasm/main_bg.wasm'})
		.catch((error) => {
			if (!error.message.startsWith("Using exceptions for control flow,"))
				throw error
	})
	</script>
	<div id="root"></div>
</body>
</html>
"#;
	fs_ext::write("target/examples/csr/index.html", html).unwrap();
}

#[template]
// components with client directives must be serde
#[derive(Reflect)]
fn MyApp() -> impl Bundle {
	rsx! {
		<style scope:global>
		body{
			color: #ddd;
			background-color: #333;
		}
		</style>
		// <Counter initial=7/>
		// <AttributeChanged />
		<List/>
	}
}


#[template]
fn Counter(initial: u32) -> impl Bundle {
	let (get, set) = signal(initial);

	rsx! {
		<article>
			<p>Count: {get}</p>
			<button onclick={move || set(get()+1)}>Increment</button>
		</article>
	}
}

#[template]
fn AttributeChanged() -> impl Bundle {
	let (style, set_style) = signal("display: block;");

	rsx! {
		<article>
			<button onclick={move || set_style("display: block;")}>Show Them</button>
			<button
				style={style}
				onclick={move || set_style("display: none;")}>
				"Hide Me"
			</button>
		</article>
	}
}
#[template]
fn List() -> impl Bundle {
	let (get_children, set_children) =
		signal::<Vec<(usize, String)>>(Vec::new());

	let (get_counter, set_counter) = signal(0);
	let next_id = move || {
		set_counter.update(|prev| *prev = &*prev + 1);
		get_counter()
	};

	let add_thingie = move || {
		set_children.update(|prev| {
			prev.push((next_id(), format!("Thingie number {}", prev.len())));
		});
	};

	let remove_seventh = move || {
		set_children.update(|prev| {
			if prev.len() > 6 {
				prev.remove(6);
			}
		});
	};

	add_thingie();


	let children_view = move || {
		get_children()
			.into_iter()
			.map(|(id, text)| {
				OnSpawnClone::insert(move || {
					rsx! {
						<div>{text.clone()}" "
					<button onclick={move||{
						set_children.update(|prev|{
							prev.retain(|(other,_)| id != *other);
						})
					}}>remove</button>
					</div>}
				})
			})
			.collect::<Vec<_>>()
	};

	rsx! {
		<article>
			<button onclick={move ||add_thingie()}>Add Thingie</button>
			<button onclick={move ||remove_seventh()}>Remove 7th Thingie</button>
			{children_view}
		</article>
	}
}
