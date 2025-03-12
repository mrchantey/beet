use crate::prelude::*;
use beet::prelude::*;



pub fn get(_state: DefaultAppState) -> RsxRoot {
	let val = 88;
	rsx! {
		<PageLayout title="Beet".into()>
			<meta
				slot="head"
				name="description"
				content="This is the main file"
			/>
			{val + 8}
			<span>hello world</span>
			<style>
				span{
					color: red;
				}
			</style>
		</PageLayout>
	}
}
