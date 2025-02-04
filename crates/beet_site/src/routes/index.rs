use crate::prelude::*;
use beet::prelude::*;


pub fn get(state: DefaultAppState) -> impl Rsx {
	rsx! {
		<PageLayout title=state.app_name.into()>
			<meta slot="head" name="description" content="A simple page layout component" />
				party time i think, yep it is. ok for sure it party yes.
		</PageLayout>
	}
}
