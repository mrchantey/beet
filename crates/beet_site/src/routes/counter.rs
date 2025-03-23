use crate::prelude::*;
use beet::prelude::*;

pub fn get() -> RsxRoot {
	rsx! {
		<Counter  initial=7 client:load />
		<Counter  initial=7 client:load />
	}
}
