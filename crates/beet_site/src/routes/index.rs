use crate::prelude::*;
use beet::prelude::*;

pub fn get(_state: DefaultAppState) -> RsxRoot {
	let val = 88;

	rsx! {
		<BeetPage>
			{val + 8}
			<span>hello worlds</span>
			<style>
				span{
					color: red;
				}
			</style>
		</BeetPage>
	}
}
