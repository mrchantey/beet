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
			{val + 3}
			<span>hello worldiediedsds</span>
			<style>
				span{
					color: red;
				}
			</style>
		</PageLayout>
	}
}
