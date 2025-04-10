use crate::prelude::*;
use beet::prelude::*;


pub fn get() -> RsxNode {
	rsx! {
		<BeetSidebarLayout>
			<div>party time dude!</div>
		</BeetSidebarLayout>
	}
}
