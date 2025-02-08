use crate::prelude::*;
use beet::prelude::*;



pub fn get(_state: DefaultAppState) -> RsxRoot {
	let val = 43333;
	rsx! {
		<PageLayout title="foobar".into()>
			<meta
				slot="head"
				name="description"
				content="This is the main file"
			/>
			{val + 3}
			hello world
		</PageLayout>
	}
}




//
//
