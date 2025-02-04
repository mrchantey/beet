use crate::prelude::*;
use beet::prelude::*;


pub fn get() -> RsxRoot {
	rsx! { <PageLayout title="Beet".into()>party time dude!</PageLayout> }
}
