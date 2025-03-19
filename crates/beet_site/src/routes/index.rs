use crate::prelude::*;
use beet::prelude::*;

pub fn get(_state: DefaultAppState) -> RsxRoot {
	let val = 90;

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
