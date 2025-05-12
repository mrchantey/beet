use crate::prelude::*;
use beet::prelude::*;


pub fn get() -> WebNode {
	rsx! {
		<BeetSidebarLayout>
			<div>party time dude!</div>
		</BeetSidebarLayout>
	}
}
