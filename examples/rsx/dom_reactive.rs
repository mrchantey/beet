// use beet_rsx as beet;
use beet::prelude::*;
use beet::rsx::sigfault::effect;
use beet::rsx::sigfault::signal;

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
			<div id="label">the value is {value}</div>
			<button onclick={move |_| {
				set_value(value3() + 1);
			}}>increment</button>
			</div>
		}
	}
}


fn main() { render(); }
#[cfg(not(target_arch = "wasm32"))]
fn render() {}

#[cfg(target_arch = "wasm32")]
fn render() {
	use beet::prelude::*;
	use sweet::prelude::wasm::set_timeout_ms;

	console_error_panic_hook::set_once();

	let app = || rsx! {<MyComponent initial=7/>};
	// effects are called on render
	let doc = RsxToResumableHtml::default().map_root(&app());
	DomMounter::mount_doc(&doc);
	DomMounter::normalize();
	// sweet_utils::log!("mounted");

	// give the dom time to mount
	set_timeout_ms(100, move || {
		// sweet_utils::log!("hydrating");
		DomTarget::set(BrowserDomTarget::default());
		// effects called here too
		app().register_effects();
		EventRegistry::initialize().unwrap();
	});
}
