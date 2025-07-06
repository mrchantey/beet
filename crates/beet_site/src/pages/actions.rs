use crate::prelude::*;
use beet::prelude::*;

pub fn get() -> impl Bundle {
	rsx! {
		// <BeetSidebarLayout>
			<ActionTest client:load />
		// </BeetSidebarLayout>
	}
}
