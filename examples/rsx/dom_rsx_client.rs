// use beet_rsx as beet;
use beet::prelude::*;
use beet::rsx::sigfault::effect;
use beet::rsx::sigfault::signal;

#[cfg(target_arch = "wasm32")]
fn main() { BeetDom::mount(|| rsx! {<MyComponent initial=7/>}); }


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


#[cfg(not(target_arch = "wasm32"))]
fn main() {}
