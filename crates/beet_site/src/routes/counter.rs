use crate::prelude::*;
use beet::prelude::*;

pub fn get() -> RsxRoot {
	rsx! {
		<div>
		howdy
			<Counter  initial=7 client:load />
		</div>
	}
}
