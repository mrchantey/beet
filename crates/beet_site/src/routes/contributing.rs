use crate::prelude::*;
use beet::prelude::*;


pub fn get() -> impl Rsx {
	rsx! {
		<PageLayout title="Beet".into()>
				party time dude!
		</PageLayout>
	}
}
