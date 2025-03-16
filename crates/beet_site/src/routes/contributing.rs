use crate::prelude::*;
use beet::prelude::*;


pub fn get() -> RsxRoot {
	rsx! {
		<BeetPage>

		<div>party time dude!</div>
		</BeetPage>
	}
	// rsx! { <PageLayout title="Beet".into()>party time dude!</PageLayout> }
}
