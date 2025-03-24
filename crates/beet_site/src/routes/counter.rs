use crate::prelude::*;
use beet::prelude::*;

pub fn get() -> RsxRoot {
	let rsx = rsx! {
		<Counter  initial=7 client:load />
		<Counter  initial=7 client:load />
	};
	rsx
}
