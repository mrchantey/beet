use crate::prelude::*;
use beet::prelude::*;

pub fn get() -> RsxRoot {
	let val = 98;
	rsx! {
		<BeetPage>
			{val + 8}
			<span>hello world</span>
			<style>
				span{
					color: red;
				}
			</style>
		</BeetPage>
	}
}
