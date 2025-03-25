use crate::prelude::*;
use beet::prelude::*;

pub fn get() -> RsxRoot {
	let val = 99;
	rsx! {
		<BeetPage>
			{val + 8}
			<span>hello pizza</span>
			<Counter client:load initial=2/>
			<style>
				span{
					color: blue;
				}
			</style>
		</BeetPage>
	}
}
