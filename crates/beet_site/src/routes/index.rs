use crate::prelude::*;
use beet::prelude::*;



pub fn get(_state: DefaultAppState) -> RsxRoot {
	let val = 44;
	rsx! {
		<PageLayout title="fooba".into()>
			<meta
				slot="head"
				name="description"
				content="This is the main file"
			/>
			{val + 3}
			<span>hello world</span>
			<style>
				span{
					color: red;
				}
			</style>
		</PageLayout>
	}
}
