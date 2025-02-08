use crate::prelude::*;
use beet::prelude::*;



pub fn get(state: DefaultAppState) -> RsxRoot {
	let val = 43333;
	rsx! {
		<PageLayout title="foobar".into()>
			<metasdjdsk
				slot="head"
				name="description"
				content="A simple page layout component"
			/>
			{val + val}
			hello world
		</PageLayout>
	}
}




//
//
