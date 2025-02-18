use crate::prelude::*;
use beet::prelude::*;



pub fn get(_state: DefaultAppState) -> RsxRoot {
	let val = 433;
	rsx! {
		<PageLayout title="foobarbasszz".into()>
			<meta
				slot="head"
				name="description"
				content="This is the main file"
			/>
			{val + 3}
			<span>hello worlds</span>
			<style>
				span{
					color: red;
				}
			</style>
		</PageLayout>
	}
}




//
//
