use crate::prelude::*;
use beet::prelude::*;

pub fn get() -> RsxNode {
	let val = 99;
	rsx! {
		<BeetPage>
			{val + 8}
			<span>hello pizzadoodie</span>
			<Counter client:load initial=2/>
			<style>
				span{
					color: blue;
				}
			</style>
		</BeetPage>
	}
}
