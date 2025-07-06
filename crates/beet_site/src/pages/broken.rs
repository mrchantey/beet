use crate::prelude::*;
use beet::prelude::*;


pub fn get() -> impl Bundle {
	rsx! {
		// <BeetSidebarLayout>
			<Broken client:load />
		// </BeetSidebarLayout>
	}
}
