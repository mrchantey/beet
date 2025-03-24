use crate::prelude::*;
use beet::prelude::*;

pub fn get() -> RsxRoot {
	let rsx = rsx! {
		<div>
		<span>Counter</span>
		<Counter  initial=7 client:load />
		<Counter  initial=7 client:load />
		</div>
	};
	rsx
}
