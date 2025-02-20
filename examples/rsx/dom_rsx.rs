//! Example usage of dom rsx
//! Run in native mode to generate the static html file,
//! then build with wasm target for the interactive binary
//!
//! for live template reloading command see justfile run-dom-rsx
//!
use beet::prelude::*;
use beet::rsx::sigfault::effect;
use beet::rsx::sigfault::signal;

#[cfg(target_arch = "wasm32")]
fn main() { BeetDom::mount(app); }


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
			sweet::log!("value change to {}", value2());
		});

		rsx! {
			<div>
			<div id="label">the value is {value}</div>
			<button onclick={move |_| {
				set_value(value3() + 1);
			}}>increment</button>
			</div>
		}
	}
}


#[cfg(not(target_arch = "wasm32"))]
fn main() {
	use sweet::prelude::FsExt;
	let mut doc = RsxToResumableHtml::default().map_root(&app());
	doc.insert_wasm_script();
	let html = doc.render_pretty();
	FsExt::write("target/wasm-example/index.html", &html).unwrap();
}
