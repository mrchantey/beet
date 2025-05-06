use crate::prelude::*;
use beet::prelude::*;


pub fn get() -> WebNode {
	rsx! {
		<BeetSidebarLayout>
			<ActionTest client:load />
		</BeetSidebarLayout>
	}
}
