use crate::prelude::*;
use beet::prelude::*;


pub fn get() -> WebNode {
	rsx! {
		// <BeetSidebarLayout>
			<Broken client:load />
		// </BeetSidebarLayout>
	}
}
