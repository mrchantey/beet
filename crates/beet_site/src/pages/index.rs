use crate::prelude::*;
use beet::prelude::*;


pub fn get(state: DefaultAppState) -> RsxNode {
	rsx! {
		<PageLayout title=state.app_name.into()>
				party time!
		</PageLayout>
	}
}
