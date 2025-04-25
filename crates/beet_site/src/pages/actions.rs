use crate::prelude::*;
use beet::prelude::*;


pub fn get() -> RsxNode {
	rsx! {
		<BeetSidebarLayout>
			<ActionTest client:load />
		</BeetSidebarLayout>
	}
}
