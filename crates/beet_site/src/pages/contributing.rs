use crate::prelude::*;
use beet::prelude::*;


pub fn get() -> impl Bundle {
	rsx! {
		<BeetSidebarLayout>
			<div>party time dude!</div>
		</BeetSidebarLayout>
	}
}
