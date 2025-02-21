//! Example usage of dom rsx
//! Run in native mode to generate the static html file,
//! then build with wasm target for the interactive binary
//!
//! for live template reloading command see justfile run-dom-rsx
//!
use beet::prelude::*;
use beet::rsx::sigfault::effect;
use beet::rsx::sigfault::signal;

/// The cli will run the native executable on template reloads
/// *without* recompiling.
#[cfg(not(target_arch = "wasm32"))]
fn main() {
	use sweet::prelude::FsExt;
	// the cli built the template map by looking at this file
	let template_map =
		RsxTemplateMap::load(BuildRsxTemplateMap::DEFAULT_TEMPLATES_DST)
			.unwrap();

	// we'll create the app even though its static parts are stale
	// because we need the rusty parts to fill in the html template
	let stale_app = app();


	// apply the template to the app
	let fresh_app = template_map.apply_template(stale_app).unwrap();

	// build the doc and save it, the web server will detect a change
	// and reload the page.
	let mut doc = RsxToResumableHtml::default().map_root(&fresh_app);
	doc.insert_wasm_script();
	let html = doc.render();
	FsExt::write("target/wasm-example/index.html", &html).unwrap();
}


#[cfg(target_arch = "wasm32")]
fn main() { BeetDom::hydrate(app); }


fn app() -> RsxRoot {
	rsx! {<MyComponent initial=7/>}
}

struct MyComponent {
	initial: u32,
}
#[allow(unused)]
impl Component for MyComponent {
	fn render(self) -> RsxRoot {
		let (value, set_value) = signal(self.initial);
		let value2 = value.clone();
		let value3 = value.clone();

		let effect = effect(move || {
			sweet::log!("value changed to {}", value2());
		});

		rsx! {
			<div>
			<div id="label">we how cool {value}</div>
			<button onclick={move |_| {
				set_value(value3() + 1);
			}}>increment</button>
			</div>
		}
	}
}
